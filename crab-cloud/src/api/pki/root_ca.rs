use crate::state::AppState;
use axum::Json;
use axum::extract::State;

pub async fn get_root_ca(State(state): State<AppState>) -> Json<serde_json::Value> {
    match state.ca_store.get_or_create_root_ca().await {
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
