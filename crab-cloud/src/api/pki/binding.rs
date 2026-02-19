use crate::db::activations;
use crate::state::AppState;
use axum::Json;
use axum::extract::State;
use shared::activation::SignedBinding;
use shared::error::ErrorCode;

#[derive(serde::Deserialize)]
pub struct RefreshBindingRequest {
    pub binding: SignedBinding,
}

pub async fn refresh_binding(
    State(state): State<AppState>,
    Json(req): Json<RefreshBindingRequest>,
) -> Json<serde_json::Value> {
    let tenant_ca = match state.ca_store.load_tenant_ca(&req.binding.tenant_id).await {
        Ok(ca) => ca,
        Err(e) => {
            tracing::warn!(
                tenant_id = %req.binding.tenant_id,
                error = %e,
                "Failed to load Tenant CA"
            );
            return auth_failed();
        }
    };

    if let Err(e) = req.binding.verify_signature(tenant_ca.cert_pem()) {
        tracing::warn!(
            entity_id = %req.binding.entity_id,
            error = %e,
            "Invalid binding signature"
        );
        return auth_failed();
    }

    let activation = match activations::find_by_entity(&state.pool, &req.binding.entity_id).await {
        Ok(Some(a)) => a,
        Ok(None) => {
            tracing::warn!(
                entity_id = %req.binding.entity_id,
                "Activation record not found"
            );
            return auth_failed();
        }
        Err(e) => {
            tracing::error!(error = %e, "Database error checking activation");
            return auth_failed();
        }
    };

    if activation.status != "active" {
        tracing::warn!(
            entity_id = %req.binding.entity_id,
            status = %activation.status,
            "Rejected refresh for non-active device"
        );
        return Json(serde_json::json!({
            "success": false,
            "error": format!("device_{}", activation.status),
            "error_code": ErrorCode::ActivationFailed
        }));
    }

    let refreshed = req.binding.refresh();
    let signed = match refreshed.sign(&tenant_ca.key_pem()) {
        Ok(b) => b,
        Err(e) => {
            tracing::error!(error = %e, "Failed to sign binding");
            return auth_failed();
        }
    };

    if let Err(e) = activations::update_last_refreshed(&state.pool, &signed.entity_id).await {
        tracing::warn!(error = %e, "Failed to update last_refreshed_at");
    }

    Json(serde_json::json!({
        "success": true,
        "binding": signed
    }))
}

fn auth_failed() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "success": false,
        "error": "Authentication failed",
        "error_code": ErrorCode::TenantCredentialsInvalid
    }))
}
