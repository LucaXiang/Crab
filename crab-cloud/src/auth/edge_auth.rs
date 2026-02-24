//! Edge-server authentication via SignedBinding
//!
//! Two-layer authentication:
//! 1. mTLS: Entity Cert verified by axum-server RustlsConfig (transport layer)
//! 2. SignedBinding: X-Signed-Binding header verified at application layer

use axum::{
    extract::{Request, State},
    middleware::Next,
    response::{IntoResponse, Response},
};
use shared::activation::SignedBinding;
use shared::error::{AppError, ErrorCode};

use crate::state::AppState;

/// Authenticated edge-server identity extracted from SignedBinding
#[derive(Debug, Clone)]
pub struct EdgeIdentity {
    pub entity_id: String,
    pub tenant_id: String,
    pub device_id: String,
    pub entity_type: shared::activation::EntityType,
}

/// Middleware that extracts and verifies SignedBinding from request headers
///
/// Expects `X-Signed-Binding` header containing a JSON-serialized SignedBinding.
/// Verifies the signature using the Tenant CA cert from PostgreSQL.
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
        .ok_or_else(|| AppError::new(ErrorCode::NotAuthenticated).into_response())?;

    // Parse SignedBinding JSON
    let binding: SignedBinding = serde_json::from_str(binding_header)
        .map_err(|_| AppError::new(ErrorCode::NotAuthenticated).into_response())?;

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
            AppError::new(ErrorCode::InternalError).into_response()
        })?;

    // Verify signature
    binding.verify_signature(&tenant_ca_cert).map_err(|e| {
        tracing::warn!(
            entity_id = %binding.entity_id,
            tenant_id = %binding.tenant_id,
            "SignedBinding verification failed: {e}"
        );
        AppError::new(ErrorCode::NotAuthenticated).into_response()
    })?;

    // Inject EdgeIdentity into request extensions
    let identity = EdgeIdentity {
        entity_id: binding.entity_id.clone(),
        tenant_id: binding.tenant_id.clone(),
        device_id: binding.device_id.clone(),
        entity_type: binding.entity_type,
    };

    request.extensions_mut().insert(identity);

    Ok(next.run(request).await)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_edge_identity_clone() {
        let identity = EdgeIdentity {
            entity_id: "edge-001".to_string(),
            tenant_id: "tenant-123".to_string(),
            device_id: "hw-abc".to_string(),
            entity_type: shared::activation::EntityType::Server,
        };
        let cloned = identity.clone();
        assert_eq!(cloned.entity_id, "edge-001");
        assert_eq!(cloned.tenant_id, "tenant-123");
    }
}
