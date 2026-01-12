use axum::{Json, Router, routing::get};
use serde::Serialize;

use crate::server::ServerState;

/// Health check router - public routes (no auth required)
pub fn router() -> Router<ServerState> {
    Router::new()
        .route("/health", get(health))
        .route("/health/live", get(liveness))
        .route("/health/ready", get(readiness))
}

#[derive(Serialize)]
pub struct HealthResponse {
    status: &'static str,
    version: &'static str,
}

/// Basic health check
pub async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "healthy",
        version: env!("CARGO_PKG_VERSION"),
    })
}

/// Kubernetes liveness probe
pub async fn liveness() -> &'static str {
    "OK"
}

/// Kubernetes readiness probe - check dependencies
pub async fn readiness() -> Result<&'static str, &'static str> {
    // Simplified - actual DB check would need state
    Ok("OK")
}
