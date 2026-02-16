use crate::state::AppState;
use axum::{
    Json, Router,
    extract::State,
    http::HeaderMap,
    routing::{get, post},
};
use chrono::{Duration, Utc};
use crab_cert::{CaProfile, CertMetadata, CertProfile, CertificateAuthority};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use shared::activation::{
    ActivationData, ActivationResponse, EntityType, PlanType, SignedBinding, SubscriptionInfo,
    SubscriptionStatus,
};
use std::sync::Arc;

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String,
    exp: usize,
    scopes: Vec<String>,
}

#[derive(Deserialize)]
pub struct LoginRequest {
    username: String,
    password: String,
}

async fn login(
    State(state): State<Arc<AppState>>,
    Json(req): Json<LoginRequest>,
) -> Json<serde_json::Value> {
    if let Some(tenant_id) = state
        .user_store
        .authenticate(&req.username, &req.password)
        .await
    {
        // Generate Provisioning Token (Short-lived, can issue certs)
        let expiration = Utc::now()
            .checked_add_signed(Duration::minutes(5))
            .expect("valid timestamp")
            .timestamp();

        let claims = Claims {
            sub: req.username.clone(),
            exp: expiration as usize,
            scopes: vec!["cert:issue".to_string()],
        };

        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(state.jwt_secret.as_bytes()),
        )
        .unwrap_or_default();

        Json(serde_json::json!({
            "success": true,
            "message": "Login successful",
            "token": token,
            "tenant_id": tenant_id
        }))
    } else {
        Json(serde_json::json!({
            "success": false,
            "message": "Invalid credentials"
        }))
    }
}

#[derive(Deserialize)]
pub struct IssueCertRequest {
    tenant_id: String,
    common_name: String,
    is_server: bool,
    device_id: Option<String>,
}

async fn issue_cert(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<IssueCertRequest>,
) -> Json<serde_json::Value> {
    // 0. Verify Auth Token
    let auth_header = headers
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");

    if !auth_header.starts_with("Bearer ") {
        return Json(serde_json::json!({
            "success": false,
            "error": "Missing or invalid Authorization header"
        }));
    }

    let token = &auth_header[7..];
    let token_data = match decode::<Claims>(
        token,
        &DecodingKey::from_secret(state.jwt_secret.as_bytes()),
        &Validation::new(Algorithm::HS256),
    ) {
        Ok(c) => c,
        Err(e) => {
            return Json(
                serde_json::json!({ "success": false, "error": format!("Invalid token: {}", e) }),
            );
        }
    };

    if !token_data.claims.scopes.contains(&"cert:issue".to_string()) {
        return Json(
            serde_json::json!({ "success": false, "error": "Token missing 'cert:issue' scope" }),
        );
    }

    // 1. Load Root CA
    let root_ca = match state.auth_storage.get_or_create_root_ca() {
        Ok(ca) => ca,
        Err(e) => return Json(serde_json::json!({ "success": false, "error": e.to_string() })),
    };

    // 2. Ensure Tenant CA exists (or create it signed by Root)
    let tenant_dir = state.auth_storage.get_tenant_dir(&req.tenant_id);
    let tenant_ca_name = "tenant_ca";

    let tenant_ca = if tenant_dir.join(format!("{}.crt", tenant_ca_name)).exists() {
        CertificateAuthority::load_from_file(
            &tenant_dir.join(format!("{}.crt", tenant_ca_name)),
            &tenant_dir.join(format!("{}.key", tenant_ca_name)),
        )
        .unwrap()
    } else {
        // 2. Create Tenant CA if needed
        let profile = CaProfile::intermediate(&req.tenant_id, &format!("Tenant {}", req.tenant_id));
        let ca = CertificateAuthority::new_intermediate(profile, &root_ca).unwrap();
        ca.save(&tenant_dir, tenant_ca_name).unwrap();
        ca
    };

    // 3. Issue Leaf Cert
    if req.is_server && req.device_id.is_none() {
        return Json(serde_json::json!({
            "success": false,
            "error": "Server certificate requires device_id"
        }));
    }

    let profile = if req.is_server {
        let mut p = CertProfile::new_server(
            &req.common_name,
            vec![req.common_name.clone(), "localhost".to_string()],
            Some(req.tenant_id.clone()),
            req.device_id.clone().expect("device_id checked above"),
        );
        // Allow Server cert to also be used for Client Auth (Dual Use)
        // This enables the server to also act as a client (e.g. for debugging or mesh)
        p.is_client = true;
        p
    } else {
        CertProfile::new_client(
            &req.common_name,
            Some(req.tenant_id.clone()),
            req.device_id.clone(),
            None,
        )
    };

    let (cert, key) = tenant_ca.issue_cert(&profile).unwrap();

    // For Server Certs, we should ideally bundle the Intermediate CA so the client can verify the chain
    // But for flexibility, we return them separately. The consumer can concatenate if needed.
    // Actually, let's bundle it for 'cert' field if it's a server cert to ensure full chain.
    let full_cert_chain = if req.is_server {
        format!("{}\n{}", cert, tenant_ca.cert_pem())
    } else {
        cert
    };

    // 4. Generate Identity Token (Long-lived, verify only)
    // Set to 10 years to support "maintenance free" operation without frequent re-login
    let expiration = Utc::now()
        .checked_add_signed(Duration::days(3650))
        .expect("valid timestamp")
        .timestamp();

    let identity_claims = Claims {
        sub: req.common_name.clone(),
        exp: expiration as usize,
        scopes: vec!["identity:verify".to_string()],
    };

    let identity_token = encode(
        &Header::default(),
        &identity_claims,
        &EncodingKey::from_secret(state.jwt_secret.as_bytes()),
    )
    .unwrap_or_default();

    // 5. Sign the credential data using Tenant CA private key
    let credential_expires_at = expiration as u64;
    let signable_data = format!(
        "{}|{}|{}|{}",
        req.common_name,
        req.tenant_id,
        credential_expires_at,
        req.device_id.as_deref().unwrap_or("")
    );
    let credential_signature = match crab_cert::sign(&tenant_ca.key_pem(), signable_data.as_bytes())
    {
        Ok(sig) => {
            use base64::Engine;
            Some(base64::engine::general_purpose::STANDARD.encode(&sig))
        }
        Err(e) => {
            tracing::warn!("Failed to sign credential: {}", e);
            None
        }
    };

    // 6. Return bundles
    Json(serde_json::json!({
        "success": true,
        "cert": full_cert_chain, // Contains Leaf + Intermediate (for Server)
        "key": key,
        "tenant_ca_cert": tenant_ca.cert_pem(), // For verifying clients (Trust Anchor)
        "root_ca_cert": root_ca.cert_pem(),      // Global Trust Anchor (PEM format)
        "identity_token": identity_token,        // Long-lived token for verification
        "credential_signature": credential_signature, // Credential signature by Tenant CA
        "credential_expires_at": credential_expires_at // Credential expiration timestamp
    }))
}

#[derive(Deserialize)]
pub struct ActivateRequest {
    pub username: String,
    pub password: String,
    pub device_id: String,
}

async fn activate(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ActivateRequest>,
) -> Json<ActivationResponse> {
    // 1. Authenticate
    let tenant_id = match state
        .user_store
        .authenticate(&req.username, &req.password)
        .await
    {
        Some(id) => id,
        None => {
            return Json(ActivationResponse {
                success: false,
                error: Some("Invalid credentials".to_string()),
                data: None,
                quota_info: None,
            });
        }
    };

    // 2. Load Root CA
    let root_ca = match state.auth_storage.get_or_create_root_ca() {
        Ok(ca) => ca,
        Err(e) => {
            return Json(ActivationResponse {
                success: false,
                error: Some(e.to_string()),
                data: None,
                quota_info: None,
            });
        }
    };

    // 3. Ensure Tenant CA exists
    let tenant_dir = state.auth_storage.get_tenant_dir(&tenant_id);
    let tenant_ca_name = "tenant_ca";
    let tenant_ca = if tenant_dir.join(format!("{}.crt", tenant_ca_name)).exists() {
        match CertificateAuthority::load_from_file(
            &tenant_dir.join(format!("{}.crt", tenant_ca_name)),
            &tenant_dir.join(format!("{}.key", tenant_ca_name)),
        ) {
            Ok(ca) => ca,
            Err(e) => {
                return Json(ActivationResponse {
                    success: false,
                    error: Some(format!("Failed to load Tenant CA: {}", e)),
                    data: None,
                    quota_info: None,
                });
            }
        }
    } else {
        let profile = CaProfile::intermediate(&tenant_id, &format!("Tenant {}", tenant_id));
        let ca = match CertificateAuthority::new_intermediate(profile, &root_ca) {
            Ok(ca) => ca,
            Err(e) => {
                return Json(ActivationResponse {
                    success: false,
                    error: Some(format!("Failed to create Tenant CA: {}", e)),
                    data: None,
                    quota_info: None,
                });
            }
        };
        if let Err(e) = ca.save(&tenant_dir, tenant_ca_name) {
            return Json(ActivationResponse {
                success: false,
                error: Some(format!("Failed to save Tenant CA: {}", e)),
                data: None,
                quota_info: None,
            });
        }
        ca
    };

    // 4. Generate Server ID
    let server_id = format!("edge-server-{}", uuid::Uuid::new_v4());

    // 5. Issue Server Cert
    let mut profile = CertProfile::new_server(
        &server_id,
        vec![server_id.clone(), "localhost".to_string()],
        Some(tenant_id.clone()),
        req.device_id.clone(),
    );
    profile.is_client = true; // Dual identity

    let (entity_cert, entity_key) = match tenant_ca.issue_cert(&profile) {
        Ok(pair) => pair,
        Err(e) => {
            return Json(ActivationResponse {
                success: false,
                error: Some(format!("Failed to issue certificate: {}", e)),
                data: None,
                quota_info: None,
            });
        }
    };

    // 6. Calculate certificate fingerprint
    let fingerprint = match CertMetadata::from_pem(&entity_cert) {
        Ok(meta) => meta.fingerprint_sha256,
        Err(e) => {
            return Json(ActivationResponse {
                success: false,
                error: Some(format!("Failed to parse certificate metadata: {}", e)),
                data: None,
                quota_info: None,
            });
        }
    };

    // 7. Create and sign the binding
    let binding = SignedBinding::new(
        &server_id,
        &tenant_id,
        &req.device_id,
        &fingerprint,
        EntityType::Server,
    );

    let signed_binding = match binding.sign(&tenant_ca.key_pem()) {
        Ok(b) => b,
        Err(e) => {
            return Json(ActivationResponse {
                success: false,
                error: Some(format!("Failed to sign binding: {}", e)),
                data: None,
                quota_info: None,
            });
        }
    };

    // 8. Build subscription info (mock: 根据 tenant_id 返回不同状态)
    let signature_valid_until = shared::util::now_millis() + 7 * 24 * 60 * 60 * 1000;
    let pro_features = vec![
        "audit_log".to_string(),
        "advanced_reporting".to_string(),
        "api_access".to_string(),
        "marketing".to_string(),
    ];
    let (sub_status, sub_plan, sub_features) = match tenant_id.as_str() {
        "tenant-inactive" => (SubscriptionStatus::Inactive, PlanType::Basic, vec![]),
        "tenant-expired" | "expired_tenant" => {
            (SubscriptionStatus::Expired, PlanType::Basic, vec![])
        }
        "tenant-canceled" => (SubscriptionStatus::Canceled, PlanType::Pro, vec![]),
        "tenant-unpaid" => (SubscriptionStatus::Unpaid, PlanType::Pro, vec![]),
        "tenant-pastdue" => (
            SubscriptionStatus::PastDue,
            PlanType::Pro,
            pro_features.clone(),
        ),
        _ => (SubscriptionStatus::Active, PlanType::Pro, pro_features),
    };

    let subscription = SubscriptionInfo {
        tenant_id: tenant_id.clone(),
        id: None,
        status: sub_status,
        plan: sub_plan,
        starts_at: shared::util::now_millis(),
        expires_at: Some(shared::util::now_millis() + 365 * 24 * 60 * 60 * 1000),
        features: sub_features,
        max_stores: sub_plan.max_stores() as u32,
        max_clients: 0,
        signature_valid_until,
        signature: String::new(),
        last_checked_at: 0,
        p12: None,
    };

    // Sign the subscription with Tenant CA
    let signed_subscription = match subscription.sign(&tenant_ca.key_pem()) {
        Ok(s) => s,
        Err(e) => {
            return Json(ActivationResponse {
                success: false,
                error: Some(format!("Failed to sign subscription: {}", e)),
                data: None,
                quota_info: None,
            });
        }
    };

    // 9. Build activation data
    let data = ActivationData {
        entity_id: server_id,
        tenant_id,
        device_id: req.device_id,
        root_ca_cert: root_ca.cert_pem().to_string(),
        tenant_ca_cert: tenant_ca.cert_pem().to_string(),
        entity_cert,
        entity_key,
        binding: signed_binding,
        subscription: Some(signed_subscription),
    };

    tracing::info!(
        "🚀 Activated server: entity_id={}, tenant_id={}",
        data.entity_id,
        data.tenant_id
    );

    Json(ActivationResponse {
        success: true,
        error: None,
        data: Some(data),
        quota_info: None,
    })
}

async fn get_root_ca(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    match state.auth_storage.get_or_create_root_ca() {
        Ok(ca) => Json(serde_json::json!({
            "success": true,
            "root_ca_cert": ca.cert_pem()
        })),
        Err(e) => Json(serde_json::json!({
            "success": false,
            "error": e.to_string()
        })),
    }
}

async fn get_subscription_status(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let tenant_id = match payload.get("tenant_id").and_then(|v| v.as_str()) {
        Some(id) => id.to_string(),
        None => {
            return Json(serde_json::json!({
                "success": false,
                "error": "tenant_id is required"
            }));
        }
    };

    // Load Tenant CA for signing
    let tenant_dir = state.auth_storage.get_tenant_dir(&tenant_id);
    let tenant_ca_name = "tenant_ca";
    let cert_path = tenant_dir.join(format!("{}.crt", tenant_ca_name));
    let key_path = tenant_dir.join(format!("{}.key", tenant_ca_name));
    let tenant_ca = match CertificateAuthority::load_from_file(&cert_path, &key_path) {
        Ok(ca) => ca,
        Err(e) => {
            return Json(serde_json::json!({
                "success": false,
                "error": format!("Tenant not found or CA error: {}", e)
            }));
        }
    };

    // Mock subscription logic
    // In production: query database (e.g. Stripe/Paddle webhook data)
    let pro_features = vec![
        "audit_log".to_string(),
        "advanced_reporting".to_string(),
        "api_access".to_string(),
        "marketing".to_string(),
    ];
    let (status, plan, features) = match tenant_id.as_str() {
        "tenant-inactive" => (SubscriptionStatus::Inactive, PlanType::Basic, vec![]),
        "tenant-expired" | "expired_tenant" => {
            (SubscriptionStatus::Expired, PlanType::Basic, vec![])
        }
        "tenant-canceled" => (SubscriptionStatus::Canceled, PlanType::Pro, vec![]),
        "tenant-unpaid" => (SubscriptionStatus::Unpaid, PlanType::Pro, vec![]),
        "tenant-pastdue" => (
            SubscriptionStatus::PastDue,
            PlanType::Pro,
            pro_features.clone(),
        ),
        _ => (SubscriptionStatus::Active, PlanType::Pro, pro_features),
    };

    // Subscription signature valid for 7 days (short TTL for security)
    let signature_valid_until = shared::util::now_millis() + 7 * 24 * 60 * 60 * 1000;

    // Build subscription info
    let subscription = SubscriptionInfo {
        tenant_id: tenant_id.clone(),
        id: None,
        status,
        plan,
        starts_at: shared::util::now_millis(),
        expires_at: Some(shared::util::now_millis() + 365 * 24 * 60 * 60 * 1000),
        features,
        max_stores: plan.max_stores() as u32,
        max_clients: 0,
        signature_valid_until,
        signature: String::new(),
        last_checked_at: 0,
        p12: None,
    };

    // Sign the subscription with Tenant CA
    let signed_subscription = match subscription.sign(&tenant_ca.key_pem()) {
        Ok(s) => s,
        Err(e) => {
            return Json(serde_json::json!({
                "success": false,
                "error": format!("Failed to sign subscription: {}", e)
            }));
        }
    };

    Json(serde_json::json!({
        "success": true,
        "subscription": signed_subscription
    }))
}

/// 刷新 Binding 请求
#[derive(Deserialize)]
pub struct RefreshBindingRequest {
    /// 当前 binding (用于验证身份)
    pub binding: SignedBinding,
}

/// 统一的认证失败响应（防止信息泄露）
fn auth_failed() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "success": false,
        "error": "Authentication failed"
    }))
}

/// 刷新 Binding (更新 last_verified_at 并重新签名)
///
/// 边缘服务器和客户端定期调用此 API 来刷新 binding，
/// 用于防止时钟篡改。
///
/// 安全检查：
/// 1. 验证 binding 签名
/// 2. 检查实体是否被撤销
async fn refresh_binding(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RefreshBindingRequest>,
) -> Json<serde_json::Value> {
    // 1. Load Tenant CA for this tenant
    let tenant_dir = state.auth_storage.get_tenant_dir(&req.binding.tenant_id);
    let tenant_ca_name = "tenant_ca";
    let cert_path = tenant_dir.join(format!("{}.crt", tenant_ca_name));
    let key_path = tenant_dir.join(format!("{}.key", tenant_ca_name));

    let tenant_ca = match CertificateAuthority::load_from_file(&cert_path, &key_path) {
        Ok(ca) => ca,
        Err(e) => {
            // 统一错误消息，不泄露租户是否存在
            tracing::warn!(
                "Failed to load Tenant CA for tenant={}: {}",
                req.binding.tenant_id,
                e
            );
            return auth_failed();
        }
    };

    // 2. Verify the current binding signature (防止伪造请求)
    if let Err(e) = req.binding.verify_signature(tenant_ca.cert_pem()) {
        tracing::warn!(
            "Invalid binding signature for entity={}, tenant={}: {}",
            req.binding.entity_id,
            req.binding.tenant_id,
            e
        );
        return auth_failed();
    }

    // 3. 检查实体是否被撤销
    if state
        .revocation_store
        .is_revoked(&req.binding.tenant_id, &req.binding.entity_id)
        .await
    {
        tracing::warn!(
            "🚫 Rejected refresh for revoked entity={}, tenant={}",
            req.binding.entity_id,
            req.binding.tenant_id
        );
        return auth_failed();
    }

    // 4. Refresh and re-sign
    let refreshed = req.binding.refresh();
    let signed = match refreshed.sign(&tenant_ca.key_pem()) {
        Ok(b) => b,
        Err(e) => {
            tracing::error!("Failed to sign binding: {}", e);
            return auth_failed();
        }
    };

    tracing::info!(
        "🔄 Refreshed binding for entity={}, tenant={}",
        signed.entity_id,
        signed.tenant_id
    );

    Json(serde_json::json!({
        "success": true,
        "binding": signed
    }))
}

/// 客户端凭证刷新请求
#[derive(Deserialize)]
pub struct CredentialRefreshRequest {
    pub client_name: String,
    pub tenant_id: String,
    pub device_id: Option<String>,
}

/// 刷新客户端凭证时间戳 (由 Tenant CA 签名)
async fn refresh_credential(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CredentialRefreshRequest>,
) -> Json<serde_json::Value> {
    // 1. Load Tenant CA
    let tenant_dir = state.auth_storage.get_tenant_dir(&req.tenant_id);
    let tenant_ca_name = "tenant_ca";
    let cert_path = tenant_dir.join(format!("{}.crt", tenant_ca_name));
    let key_path = tenant_dir.join(format!("{}.key", tenant_ca_name));

    let tenant_ca = match CertificateAuthority::load_from_file(&cert_path, &key_path) {
        Ok(ca) => ca,
        Err(e) => {
            return Json(serde_json::json!({
                "success": false,
                "error": format!("Tenant not found or CA error: {}", e)
            }));
        }
    };

    // 2. Generate signed timestamp
    let now = Utc::now().timestamp() as u64;
    let data_to_sign = format!(
        "{}|{}|{}|{}",
        now,
        req.client_name,
        req.tenant_id,
        req.device_id.as_deref().unwrap_or("")
    );

    let sig_bytes = match crab_cert::sign(&tenant_ca.key_pem(), data_to_sign.as_bytes()) {
        Ok(s) => s,
        Err(e) => {
            return Json(serde_json::json!({
                "success": false,
                "error": format!("Failed to sign timestamp: {}", e)
            }));
        }
    };

    use base64::{Engine as _, engine::general_purpose::STANDARD};
    let sig_b64 = STANDARD.encode(&sig_bytes);

    tracing::info!(
        "🔄 Refreshed credential timestamp for client={}, tenant={}",
        req.client_name,
        req.tenant_id
    );

    Json(serde_json::json!({
        "success": true,
        "last_verified_at": now,
        "last_verified_at_signature": sig_b64
    }))
}

// ============================================================================
// 实体撤销管理 API
// ============================================================================

/// 撤销实体请求
#[derive(Deserialize)]
pub struct RevokeEntityRequest {
    pub tenant_id: String,
    pub entity_id: String,
}

/// 撤销实体（需要管理员权限）
async fn revoke_entity(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<RevokeEntityRequest>,
) -> Json<serde_json::Value> {
    // 验证管理员 JWT
    if !verify_admin_token(&state, &headers) {
        return Json(serde_json::json!({
            "success": false,
            "error": "Unauthorized"
        }));
    }

    state
        .revocation_store
        .revoke(&req.tenant_id, &req.entity_id)
        .await;

    Json(serde_json::json!({
        "success": true,
        "message": format!("Entity {} has been revoked", req.entity_id)
    }))
}

/// 恢复实体（需要管理员权限）
async fn restore_entity(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<RevokeEntityRequest>,
) -> Json<serde_json::Value> {
    // 验证管理员 JWT
    if !verify_admin_token(&state, &headers) {
        return Json(serde_json::json!({
            "success": false,
            "error": "Unauthorized"
        }));
    }

    let restored = state
        .revocation_store
        .restore(&req.tenant_id, &req.entity_id)
        .await;

    if restored {
        Json(serde_json::json!({
            "success": true,
            "message": format!("Entity {} has been restored", req.entity_id)
        }))
    } else {
        Json(serde_json::json!({
            "success": false,
            "error": "Entity was not revoked"
        }))
    }
}

/// 列出已撤销的实体
#[derive(Deserialize)]
pub struct ListRevokedRequest {
    pub tenant_id: String,
}

async fn list_revoked_entities(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<ListRevokedRequest>,
) -> Json<serde_json::Value> {
    // 验证管理员 JWT
    if !verify_admin_token(&state, &headers) {
        return Json(serde_json::json!({
            "success": false,
            "error": "Unauthorized"
        }));
    }

    let entities = state.revocation_store.list_revoked(&req.tenant_id).await;

    Json(serde_json::json!({
        "success": true,
        "revoked_entities": entities
    }))
}

/// 验证管理员 JWT token
fn verify_admin_token(state: &AppState, headers: &HeaderMap) -> bool {
    let auth_header = match headers.get("Authorization") {
        Some(h) => h,
        None => return false,
    };

    let auth_str = match auth_header.to_str() {
        Ok(s) => s,
        Err(_) => return false,
    };

    if !auth_str.starts_with("Bearer ") {
        return false;
    }

    let token = &auth_str[7..];

    match decode::<Claims>(
        token,
        &DecodingKey::from_secret(state.jwt_secret.as_bytes()),
        &Validation::new(Algorithm::HS256),
    ) {
        Ok(data) => {
            // 检查是否有 cert:issue 权限（管理员权限）
            data.claims.scopes.contains(&"cert:issue".to_string())
        }
        Err(_) => false,
    }
}

/// POST /api/client/activate — 客户端激活 (签发 Client 证书)
async fn client_activate(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ActivateRequest>,
) -> Json<ActivationResponse> {
    // 1. Authenticate
    let tenant_id = match state
        .user_store
        .authenticate(&req.username, &req.password)
        .await
    {
        Some(id) => id,
        None => {
            return Json(ActivationResponse {
                success: false,
                error: Some("Invalid credentials".to_string()),
                data: None,
                quota_info: None,
            });
        }
    };

    // 2. Load Root CA
    let root_ca = match state.auth_storage.get_or_create_root_ca() {
        Ok(ca) => ca,
        Err(e) => {
            return Json(ActivationResponse {
                success: false,
                error: Some(e.to_string()),
                data: None,
                quota_info: None,
            });
        }
    };

    // 3. Ensure Tenant CA exists
    let tenant_dir = state.auth_storage.get_tenant_dir(&tenant_id);
    let tenant_ca_name = "tenant_ca";
    let tenant_ca = if tenant_dir.join(format!("{}.crt", tenant_ca_name)).exists() {
        match CertificateAuthority::load_from_file(
            &tenant_dir.join(format!("{}.crt", tenant_ca_name)),
            &tenant_dir.join(format!("{}.key", tenant_ca_name)),
        ) {
            Ok(ca) => ca,
            Err(e) => {
                return Json(ActivationResponse {
                    success: false,
                    error: Some(format!("Failed to load Tenant CA: {}", e)),
                    data: None,
                    quota_info: None,
                });
            }
        }
    } else {
        let profile = CaProfile::intermediate(&tenant_id, &format!("Tenant {}", tenant_id));
        let ca = match CertificateAuthority::new_intermediate(profile, &root_ca) {
            Ok(ca) => ca,
            Err(e) => {
                return Json(ActivationResponse {
                    success: false,
                    error: Some(format!("Failed to create Tenant CA: {}", e)),
                    data: None,
                    quota_info: None,
                });
            }
        };
        if let Err(e) = ca.save(&tenant_dir, tenant_ca_name) {
            return Json(ActivationResponse {
                success: false,
                error: Some(format!("Failed to save Tenant CA: {}", e)),
                data: None,
                quota_info: None,
            });
        }
        ca
    };

    // 4. Generate Client ID
    let client_id = format!("client-{}", uuid::Uuid::new_v4());

    // 5. Issue Client Cert (client-only, no server auth)
    let profile = CertProfile::new_client(
        &client_id,
        Some(tenant_id.clone()),
        Some(req.device_id.clone()),
        None,
    );

    let (entity_cert, entity_key) = match tenant_ca.issue_cert(&profile) {
        Ok(pair) => pair,
        Err(e) => {
            return Json(ActivationResponse {
                success: false,
                error: Some(format!("Failed to issue certificate: {}", e)),
                data: None,
                quota_info: None,
            });
        }
    };

    // 6. Calculate certificate fingerprint
    let fingerprint = match CertMetadata::from_pem(&entity_cert) {
        Ok(meta) => meta.fingerprint_sha256,
        Err(e) => {
            return Json(ActivationResponse {
                success: false,
                error: Some(format!("Failed to parse certificate metadata: {}", e)),
                data: None,
                quota_info: None,
            });
        }
    };

    // 7. Create and sign the binding (EntityType::Client)
    let binding = SignedBinding::new(
        &client_id,
        &tenant_id,
        &req.device_id,
        &fingerprint,
        EntityType::Client,
    );

    let signed_binding = match binding.sign(&tenant_ca.key_pem()) {
        Ok(b) => b,
        Err(e) => {
            return Json(ActivationResponse {
                success: false,
                error: Some(format!("Failed to sign binding: {}", e)),
                data: None,
                quota_info: None,
            });
        }
    };

    // 8. Build subscription info (same mock logic as server activate)
    let signature_valid_until = shared::util::now_millis() + 7 * 24 * 60 * 60 * 1000;
    let subscription = SubscriptionInfo {
        tenant_id: tenant_id.clone(),
        id: None,
        status: SubscriptionStatus::Active,
        plan: PlanType::Pro,
        starts_at: shared::util::now_millis(),
        expires_at: Some(shared::util::now_millis() + 365 * 24 * 60 * 60 * 1000),
        features: vec![
            "audit_log".to_string(),
            "advanced_reporting".to_string(),
            "api_access".to_string(),
        ],
        max_stores: PlanType::Pro.max_stores() as u32,
        max_clients: 0,
        signature_valid_until,
        signature: String::new(),
        last_checked_at: 0,
        p12: None,
    };

    let signed_subscription = match subscription.sign(&tenant_ca.key_pem()) {
        Ok(s) => s,
        Err(e) => {
            return Json(ActivationResponse {
                success: false,
                error: Some(format!("Failed to sign subscription: {}", e)),
                data: None,
                quota_info: None,
            });
        }
    };

    // 9. Build activation data
    let data = ActivationData {
        entity_id: client_id,
        tenant_id,
        device_id: req.device_id,
        root_ca_cert: root_ca.cert_pem().to_string(),
        tenant_ca_cert: tenant_ca.cert_pem().to_string(),
        entity_cert,
        entity_key,
        binding: signed_binding,
        subscription: Some(signed_subscription),
    };

    tracing::info!(
        "Activated client: entity_id={}, tenant_id={}",
        data.entity_id,
        data.tenant_id
    );

    Json(ActivationResponse {
        success: true,
        error: None,
        data: Some(data),
        quota_info: None,
    })
}

/// POST /api/tenant/verify — 只验证身份，不签发证书
async fn verify_tenant(
    State(state): State<Arc<AppState>>,
    Json(req): Json<serde_json::Value>,
) -> Json<shared::activation::TenantVerifyResponse> {
    let username = req.get("username").and_then(|v| v.as_str()).unwrap_or("");
    let password = req.get("password").and_then(|v| v.as_str()).unwrap_or("");

    let tenant_id = match state.user_store.authenticate(username, password).await {
        Some(id) => id,
        None => {
            return Json(shared::activation::TenantVerifyResponse {
                success: false,
                error: Some("Invalid credentials".to_string()),
                data: None,
            });
        }
    };

    // Mock: return tenant info with quota slots
    let data = shared::activation::TenantVerifyData {
        tenant_id,
        subscription_status: SubscriptionStatus::Active,
        plan: PlanType::Pro,
        server_slots_remaining: 2,
        client_slots_remaining: 5,
        has_active_server: false,
        has_active_client: false,
    };

    Json(shared::activation::TenantVerifyResponse {
        success: true,
        error: None,
        data: Some(data),
    })
}

/// POST /api/server/deactivate — 注销 Server 激活
async fn deactivate_server(
    State(state): State<Arc<AppState>>,
    Json(req): Json<serde_json::Value>,
) -> Json<shared::activation::DeactivateResponse> {
    let username = req.get("username").and_then(|v| v.as_str()).unwrap_or("");
    let password = req.get("password").and_then(|v| v.as_str()).unwrap_or("");

    if state
        .user_store
        .authenticate(username, password)
        .await
        .is_none()
    {
        return Json(shared::activation::DeactivateResponse {
            success: false,
            error: Some("Invalid credentials".to_string()),
        });
    }

    // Mock: always succeed
    tracing::info!(
        entity_id = req.get("entity_id").and_then(|v| v.as_str()).unwrap_or("?"),
        "Server deactivated (mock)"
    );

    Json(shared::activation::DeactivateResponse {
        success: true,
        error: None,
    })
}

/// POST /api/client/deactivate — 注销 Client 激活
async fn deactivate_client(
    State(state): State<Arc<AppState>>,
    Json(req): Json<serde_json::Value>,
) -> Json<shared::activation::DeactivateResponse> {
    let username = req.get("username").and_then(|v| v.as_str()).unwrap_or("");
    let password = req.get("password").and_then(|v| v.as_str()).unwrap_or("");

    if state
        .user_store
        .authenticate(username, password)
        .await
        .is_none()
    {
        return Json(shared::activation::DeactivateResponse {
            success: false,
            error: Some("Invalid credentials".to_string()),
        });
    }

    // Mock: always succeed
    tracing::info!(
        entity_id = req.get("entity_id").and_then(|v| v.as_str()).unwrap_or("?"),
        "Client deactivated (mock)"
    );

    Json(shared::activation::DeactivateResponse {
        success: true,
        error: None,
    })
}

pub fn router(state: Arc<AppState>) -> Router {
    use tower::limit::ConcurrencyLimitLayer;

    // 并发限制：最多 100 个并发请求（防止 DoS）
    let concurrency_limit = ConcurrencyLimitLayer::new(100);

    Router::new()
        .route("/api/auth/login", post(login))
        .route("/api/server/activate", post(activate))
        .route("/api/client/activate", post(client_activate))
        .route("/api/tenant/verify", post(verify_tenant))
        .route("/api/server/deactivate", post(deactivate_server))
        .route("/api/client/deactivate", post(deactivate_client))
        .route("/api/cert/issue", post(issue_cert))
        .route("/api/binding/refresh", post(refresh_binding))
        .route("/api/credential/refresh", post(refresh_credential))
        .route("/pki/root_ca", get(get_root_ca))
        .route("/api/tenant/subscription", post(get_subscription_status))
        // 撤销管理 API
        .route("/api/entity/revoke", post(revoke_entity))
        .route("/api/entity/restore", post(restore_entity))
        .route("/api/entity/revoked", post(list_revoked_entities))
        // 并发限制
        .layer(concurrency_limit)
        .with_state(state)
}
