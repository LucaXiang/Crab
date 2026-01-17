use crate::state::AppState;
use axum::{
    Json, Router,
    extract::State,
    http::HeaderMap,
    routing::{get, post},
};
use chrono::{Duration, Utc};
use crab_cert::{CaProfile, CertProfile, CertificateAuthority};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
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

    // 5. Return bundles
    Json(serde_json::json!({
        "success": true,
        "cert": full_cert_chain, // Contains Leaf + Intermediate (for Server)
        "key": key,
        "tenant_ca_cert": tenant_ca.cert_pem(), // For verifying clients (Trust Anchor)
        "root_ca_cert": root_ca.cert_pem(),      // Global Trust Anchor (PEM format)
        "identity_token": identity_token         // Long-lived token for verification
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
) -> Json<serde_json::Value> {
    // 1. Authenticate
    let tenant_id = match state
        .user_store
        .authenticate(&req.username, &req.password)
        .await
    {
        Some(id) => id,
        None => {
            return Json(serde_json::json!({
                "success": false,
                "error": "Invalid credentials"
            }));
        }
    };

    // 2. Load Root CA
    let root_ca = match state.auth_storage.get_or_create_root_ca() {
        Ok(ca) => ca,
        Err(e) => return Json(serde_json::json!({ "success": false, "error": e.to_string() })),
    };

    // 3. Ensure Tenant CA exists
    let tenant_dir = state.auth_storage.get_tenant_dir(&tenant_id);
    let tenant_ca_name = "tenant_ca";
    let tenant_ca = if tenant_dir.join(format!("{}.crt", tenant_ca_name)).exists() {
        CertificateAuthority::load_from_file(
            &tenant_dir.join(format!("{}.crt", tenant_ca_name)),
            &tenant_dir.join(format!("{}.key", tenant_ca_name)),
        )
        .unwrap()
    } else {
        let profile = CaProfile::intermediate(&tenant_id, &format!("Tenant {}", tenant_id));
        let ca = CertificateAuthority::new_intermediate(profile, &root_ca).unwrap();
        ca.save(&tenant_dir, tenant_ca_name).unwrap();
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

    let (cert, key) = tenant_ca.issue_cert(&profile).unwrap();
    let full_cert_chain = format!("{}\n{}", cert, tenant_ca.cert_pem());

    // 6. Return response
    Json(serde_json::json!({
        "success": true,
        "server_id": server_id,
        "tenant_id": tenant_id,
        "cert": full_cert_chain,
        "key": key,
        "tenant_ca_cert": tenant_ca.cert_pem(),
        "root_ca_cert": root_ca.cert_pem(),
    }))
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
    State(_state): State<Arc<AppState>>,
    Json(payload): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let tenant_id = payload
        .get("tenant_id")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    // 模拟订阅逻辑：
    // 实际项目中应查询数据库 (e.g. Stripe/Paddle webhook data)
    let (status, plan, features) = if tenant_id == "expired_tenant" {
        ("past_due", "basic", vec![])
    } else {
        (
            "active",
            "pro",
            vec!["audit_log", "advanced_reporting", "api_access"],
        )
    };

    // 默认一年后过期
    let expires_at = Utc::now()
        .checked_add_signed(Duration::days(365))
        .expect("valid timestamp")
        .to_rfc3339();

    Json(serde_json::json!({
        "success": true,
        "subscription": {
            "tenant_id": tenant_id,
            "status": status,
            "plan": plan,
            "features": features,
            "expires_at": expires_at
        }
    }))
}

pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/api/auth/login", post(login))
        .route("/api/server/activate", post(activate))
        .route("/api/cert/issue", post(issue_cert))
        .route("/pki/root_ca", get(get_root_ca))
        .route("/api/tenant/subscription", post(get_subscription_status))
        .with_state(state)
}
