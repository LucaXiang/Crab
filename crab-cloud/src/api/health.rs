//! Health check endpoint

use axum::Json;

pub async fn health_check() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
        "service": "crab-cloud",
        "version": env!("CARGO_PKG_VERSION"),
        "git_hash": option_env!("GIT_HASH").unwrap_or("dev"),
    }))
}
