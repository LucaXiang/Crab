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

#[derive(Serialize)]
pub struct LoginResponse {
    token: String,
    message: String,
}

async fn login(
    State(state): State<Arc<AppState>>,
    Json(req): Json<LoginRequest>,
) -> Json<serde_json::Value> {
    if state
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
            "token": token
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
    let root_ca = match state.cert_store.get_or_create_root_ca() {
        Ok(ca) => ca,
        Err(e) => return Json(serde_json::json!({ "success": false, "error": e.to_string() })),
    };

    // 2. Ensure Tenant CA exists (or create it signed by Root)
    let tenant_dir = state.cert_store.get_tenant_dir(&req.tenant_id);
    let tenant_ca_name = "tenant_ca";

    let tenant_ca = if tenant_dir.join(format!("{}.crt", tenant_ca_name)).exists() {
        CertificateAuthority::load_from_file(
            &tenant_dir.join(format!("{}.crt", tenant_ca_name)),
            &tenant_dir.join(format!("{}.key", tenant_ca_name)),
        )
        .unwrap()
    } else {
        // 2. Create Tenant CA if needed
        let profile = CaProfile::intermediate(
            &format!("Tenant CA {}", req.tenant_id),
            &format!("Tenant {}", req.tenant_id),
        );
        let ca = CertificateAuthority::new_intermediate(profile, &root_ca).unwrap();
        ca.save(&tenant_dir, tenant_ca_name).unwrap();
        ca
    };

    // 3. Issue Leaf Cert
    let profile = if req.is_server {
        let mut p = CertProfile::new_server(
            &req.common_name,
            vec![req.common_name.clone(), "localhost".to_string()],
        );
        // Allow Server cert to also be used for Client Auth (Dual Use)
        // This enables the server to also act as a client (e.g. for debugging or mesh)
        p.is_client = true;
        p
    } else {
        CertProfile::new_client(&req.common_name, Some(req.tenant_id.clone()), None, None)
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
        "root_ca_cert": root_ca.cert_pem(),      // Global Trust Anchor
        "identity_token": identity_token         // Long-lived token for verification
    }))
}

async fn get_root_ca(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    match state.cert_store.get_or_create_root_ca() {
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

pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/api/auth/login", post(login))
        .route("/api/cert/issue", post(issue_cert))
        .route("/pki/root_ca", get(get_root_ca))
        .with_state(state)
}
