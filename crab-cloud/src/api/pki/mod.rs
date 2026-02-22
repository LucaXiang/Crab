mod activate;
mod activate_client;
mod binding;
mod deactivate;
mod deactivate_client;
mod p12;
mod refresh;
mod root_ca;
mod subscription;
mod verify;

use crate::state::AppState;
use axum::Router;
use axum::extract::DefaultBodyLimit;
use axum::routing::{get, post};

/// PKI 路由 (从 crab-auth 合并)
pub fn pki_router() -> Router<AppState> {
    Router::new()
        .route("/api/server/activate", post(activate::activate))
        .route(
            "/api/client/activate",
            post(activate_client::activate_client),
        )
        .route(
            "/api/server/deactivate",
            post(deactivate::deactivate_server),
        )
        .route(
            "/api/client/deactivate",
            post(deactivate_client::deactivate_client),
        )
        .route(
            "/api/tenant/subscription",
            post(subscription::get_subscription_status),
        )
        .route("/api/binding/refresh", post(binding::refresh_binding))
        .route("/api/tenant/refresh", post(refresh::refresh_token))
        .route("/pki/root_ca", get(root_ca::get_root_ca))
}

/// P12 上传路由 (独立 rate limit: 3 req/min per IP)
pub fn p12_upload_router() -> Router<AppState> {
    Router::new()
        .route("/api/p12/upload", post(p12::upload_p12))
        .layer(DefaultBodyLimit::max(5 * 1024 * 1024)) // 5MB for P12
}

/// 接受密码的 PKI 端点 (需要专用限速: 5 req/min per IP)
pub fn pki_auth_router() -> Router<AppState> {
    Router::new().route("/api/tenant/verify", post(verify::verify_tenant))
}
