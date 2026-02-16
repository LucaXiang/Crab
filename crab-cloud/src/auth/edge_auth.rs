//! Edge-server authentication via SignedBinding
//!
//! Two-layer authentication:
//! 1. mTLS: Entity Cert verified by axum-server RustlsConfig (transport layer)
//! 2. SignedBinding: X-Signed-Binding header verified at application layer

use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};
use shared::activation::SignedBinding;

use crate::state::AppState;

/// Authenticated edge-server identity extracted from SignedBinding
#[derive(Debug, Clone)]
pub struct EdgeIdentity {
    pub entity_id: String,
    pub tenant_id: String,
    pub device_id: String,
}

/// Middleware that extracts and verifies SignedBinding from request headers
///
/// Expects `X-Signed-Binding` header containing a JSON-serialized SignedBinding.
/// Verifies the signature using the Tenant CA cert from Secrets Manager.
/// On success, injects `EdgeIdentity` into request extensions.
pub async fn edge_auth_middleware(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Result<Response, Response> {
    // Extract SignedBinding from header
    let binding_header = request
        .headers()
        .get("X-Signed-Binding")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| error_response(401, "Missing X-Signed-Binding header"))?;

    // Parse SignedBinding JSON
    let binding: SignedBinding = serde_json::from_str(binding_header)
        .map_err(|e| error_response(401, &format!("Invalid SignedBinding: {e}")))?;

    // Load Tenant CA cert for signature verification
    let tenant_ca_cert = state
        .ca_store
        .load_tenant_ca_cert(&binding.tenant_id)
        .await
        .map_err(|e| {
            tracing::error!(
                tenant_id = %binding.tenant_id,
                "Failed to load Tenant CA cert: {e}"
            );
            error_response(500, "Failed to verify credentials")
        })?;

    // Verify signature
    binding.verify_signature(&tenant_ca_cert).map_err(|e| {
        tracing::warn!(
            entity_id = %binding.entity_id,
            tenant_id = %binding.tenant_id,
            "SignedBinding verification failed: {e}"
        );
        error_response(401, "Invalid binding signature")
    })?;

    // Inject EdgeIdentity into request extensions
    let identity = EdgeIdentity {
        entity_id: binding.entity_id.clone(),
        tenant_id: binding.tenant_id.clone(),
        device_id: binding.device_id.clone(),
    };

    request.extensions_mut().insert(identity);

    Ok(next.run(request).await)
}

fn error_response(status: u16, message: &str) -> Response {
    let body = serde_json::json!({
        "error": message,
    });
    let status =
        http::StatusCode::from_u16(status).unwrap_or(http::StatusCode::INTERNAL_SERVER_ERROR);
    (status, axum::Json(body)).into_response()
}

use axum::response::IntoResponse;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_edge_identity_clone() {
        let identity = EdgeIdentity {
            entity_id: "edge-001".to_string(),
            tenant_id: "tenant-123".to_string(),
            device_id: "hw-abc".to_string(),
        };
        let cloned = identity.clone();
        assert_eq!(cloned.entity_id, "edge-001");
        assert_eq!(cloned.tenant_id, "tenant-123");
    }
}
