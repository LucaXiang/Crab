use crate::db::activations;
use crate::state::AppState;
use axum::Json;
use axum::extract::State;
use shared::activation::SignedBinding;
use std::sync::Arc;

#[derive(serde::Deserialize)]
pub struct RefreshBindingRequest {
    pub binding: SignedBinding,
}

pub async fn refresh_binding(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RefreshBindingRequest>,
) -> Json<serde_json::Value> {
    // 1. Load Tenant CA
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

    // 2. Verify binding signature
    if let Err(e) = req.binding.verify_signature(tenant_ca.cert_pem()) {
        tracing::warn!(
            entity_id = %req.binding.entity_id,
            error = %e,
            "Invalid binding signature"
        );
        return auth_failed();
    }

    // 3. Check activation status in PG
    let activation = match activations::find_by_entity(&state.db, &req.binding.entity_id).await {
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
            "error": format!("device_{}", activation.status)
        }));
    }

    // 4. Refresh and re-sign binding
    let refreshed = req.binding.refresh();
    let signed = match refreshed.sign(&tenant_ca.key_pem()) {
        Ok(b) => b,
        Err(e) => {
            tracing::error!(error = %e, "Failed to sign binding");
            return auth_failed();
        }
    };

    // 5. Update last_refreshed_at in PG
    if let Err(e) = activations::update_last_refreshed(&state.db, &signed.entity_id).await {
        tracing::warn!(error = %e, "Failed to update last_refreshed_at");
        // Non-fatal, continue
    }

    Json(serde_json::json!({
        "success": true,
        "binding": signed
    }))
}

fn auth_failed() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "success": false,
        "error": "Authentication failed"
    }))
}
