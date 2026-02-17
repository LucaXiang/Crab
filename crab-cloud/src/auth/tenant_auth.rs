//! Tenant JWT authentication for management API

use axum::{
    extract::{Request, State},
    middleware::Next,
    response::{IntoResponse, Response},
};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

use crate::state::AppState;

/// JWT claims for tenant authentication
#[derive(Debug, Serialize, Deserialize)]
pub struct TenantClaims {
    /// Tenant ID
    pub sub: String,
    /// Tenant email
    pub email: String,
    /// Expiration (Unix timestamp seconds)
    pub exp: usize,
    /// Issued at (Unix timestamp seconds)
    pub iat: usize,
}

/// Authenticated tenant identity extracted from JWT
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct TenantIdentity {
    pub tenant_id: String,
    pub email: String,
}

const JWT_EXPIRY_HOURS: i64 = 24;

/// Create a JWT token for a tenant
pub fn create_token(
    tenant_id: &str,
    email: &str,
    secret: &str,
) -> Result<String, jsonwebtoken::errors::Error> {
    let now = chrono::Utc::now();
    let claims = TenantClaims {
        sub: tenant_id.to_string(),
        email: email.to_string(),
        exp: (now + chrono::Duration::hours(JWT_EXPIRY_HOURS)).timestamp() as usize,
        iat: now.timestamp() as usize,
    };

    jsonwebtoken::encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
}

/// Middleware that extracts and verifies tenant JWT from Authorization header
pub async fn tenant_auth_middleware(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Result<Response, Response> {
    let auth_header = request
        .headers()
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| error_response(401, "Missing Authorization header"))?;

    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or_else(|| error_response(401, "Invalid Authorization format"))?;

    let validation = Validation::default();
    let token_data = jsonwebtoken::decode::<TenantClaims>(
        token,
        &DecodingKey::from_secret(state.jwt_secret.as_bytes()),
        &validation,
    )
    .map_err(|e| {
        tracing::debug!("JWT validation failed: {e}");
        error_response(401, "Invalid or expired token")
    })?;

    let identity = TenantIdentity {
        tenant_id: token_data.claims.sub,
        email: token_data.claims.email,
    };

    request.extensions_mut().insert(identity);

    Ok(next.run(request).await)
}

fn error_response(status: u16, message: &str) -> Response {
    let body = serde_json::json!({ "error": message });
    let status =
        http::StatusCode::from_u16(status).unwrap_or(http::StatusCode::INTERNAL_SERVER_ERROR);
    (status, axum::Json(body)).into_response()
}
