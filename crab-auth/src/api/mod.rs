mod activate;
mod binding;
mod p12;
mod pki;
mod subscription;

use crate::state::AppState;
use axum::extract::DefaultBodyLimit;
use axum::routing::{get, post};
use axum::{Json, Router};
use std::sync::Arc;
use std::time::Duration;
use tower_http::timeout::TimeoutLayer;

pub fn router(state: Arc<AppState>) -> Router {
    // P12 上传需要更大的 body 限制
    let p12_routes = Router::new()
        .route("/api/p12/upload", post(p12::upload_p12))
        .layer(DefaultBodyLimit::max(5 * 1024 * 1024)); // 5MB

    Router::new()
        .route("/api/server/activate", post(activate::activate))
        .route(
            "/api/tenant/subscription",
            post(subscription::get_subscription_status),
        )
        .route("/api/binding/refresh", post(binding::refresh_binding))
        .route("/pki/root_ca", get(pki::get_root_ca))
        .route("/health", get(health))
        .merge(p12_routes)
        .layer(DefaultBodyLimit::max(1024 * 1024)) // 1MB default
        .layer(TimeoutLayer::with_status_code(http::StatusCode::REQUEST_TIMEOUT, Duration::from_secs(30)))
        .with_state(state)
}

async fn health(
    axum::extract::State(state): axum::extract::State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    let db_ok = state.db.acquire().await.is_ok();
    let status = if db_ok { "ok" } else { "degraded" };
    Json(serde_json::json!({
        "status": status,
        "db": db_ok
    }))
}
