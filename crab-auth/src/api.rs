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
    ActivationData, ActivationResponse, EntityType, PlanType, SignedBinding,
    SubscriptionInfo, SubscriptionStatus,
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
    let credential_signature = match crab_cert::sign(&tenant_ca.key_pem(), signable_data.as_bytes()) {
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
                });
            }
        };
        if let Err(e) = ca.save(&tenant_dir, tenant_ca_name) {
            return Json(ActivationResponse {
                success: false,
                error: Some(format!("Failed to save Tenant CA: {}", e)),
                data: None,
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
            });
        }
    };

    // 8. Build subscription info (mock for now)
    // Subscription signature valid for 7 days
    let signature_valid_until = Utc::now()
        .checked_add_signed(Duration::days(7))
        .expect("valid timestamp")
        .to_rfc3339();

    let subscription = SubscriptionInfo {
        tenant_id: tenant_id.clone(),
        id: None,
        status: SubscriptionStatus::Active,
        plan: PlanType::Pro,
        starts_at: Utc::now().to_rfc3339(),
        expires_at: Some(
            Utc::now()
                .checked_add_signed(Duration::days(365))
                .expect("valid timestamp")
                .to_rfc3339(),
        ),
        features: vec![
            "audit_log".to_string(),
            "advanced_reporting".to_string(),
            "api_access".to_string(),
        ],
        signature_valid_until,
        signature: String::new(), // Will be signed below
    };

    // Sign the subscription with Tenant CA
    let signed_subscription = match subscription.sign(&tenant_ca.key_pem()) {
        Ok(s) => s,
        Err(e) => {
            return Json(ActivationResponse {
                success: false,
                error: Some(format!("Failed to sign subscription: {}", e)),
                data: None,
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
    let (status, plan, features) = if tenant_id == "expired_tenant" {
        (SubscriptionStatus::PastDue, PlanType::Free, vec![])
    } else {
        (
            SubscriptionStatus::Active,
            PlanType::Pro,
            vec![
                "audit_log".to_string(),
                "advanced_reporting".to_string(),
                "api_access".to_string(),
            ],
        )
    };

    // Subscription signature valid for 7 days (short TTL for security)
    let signature_valid_until = Utc::now()
        .checked_add_signed(Duration::days(7))
        .expect("valid timestamp")
        .to_rfc3339();

    // Build subscription info
    let subscription = SubscriptionInfo {
        tenant_id: tenant_id.clone(),
        id: None,
        status,
        plan,
        starts_at: Utc::now().to_rfc3339(),
        expires_at: Some(
            Utc::now()
                .checked_add_signed(Duration::days(365))
                .expect("valid timestamp")
                .to_rfc3339(),
        ),
        features,
        signature_valid_until,
        signature: String::new(),
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

/// 刷新 Binding (更新 last_verified_at 并重新签名)
///
/// 边缘服务器和客户端定期调用此 API 来刷新 binding，
/// 用于防止时钟篡改。
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
            return Json(serde_json::json!({
                "success": false,
                "error": format!("Tenant not found or CA error: {}", e)
            }));
        }
    };

    // 2. Verify the current binding signature (防止伪造请求)
    if let Err(e) = req.binding.verify_signature(&tenant_ca.cert_pem()) {
        return Json(serde_json::json!({
            "success": false,
            "error": format!("Invalid binding signature: {}", e)
        }));
    }

    // 3. Refresh and re-sign
    let refreshed = req.binding.refresh();
    let signed = match refreshed.sign(&tenant_ca.key_pem()) {
        Ok(b) => b,
        Err(e) => {
            return Json(serde_json::json!({
                "success": false,
                "error": format!("Failed to sign binding: {}", e)
            }));
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

pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/api/auth/login", post(login))
        .route("/api/server/activate", post(activate))
        .route("/api/cert/issue", post(issue_cert))
        .route("/api/binding/refresh", post(refresh_binding))
        .route("/api/credential/refresh", post(refresh_credential))
        .route("/pki/root_ca", get(get_root_ca))
        .route("/api/tenant/subscription", post(get_subscription_status))
        .with_state(state)
}
