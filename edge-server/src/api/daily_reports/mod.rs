//! Daily Report API 模块 (日结报告)

mod handler;

use axum::{
    Router, middleware,
    routing::{get, post},
};

use crate::auth::require_permission;
use crate::core::ServerState;

pub fn router() -> Router<ServerState> {
    Router::new().nest("/api/daily-reports", routes())
}

fn routes() -> Router<ServerState> {
    // 日报查看：需要 reports:view 权限
    Router::new()
        .route("/", get(handler::list))
        .route("/generate", post(handler::generate))
        .route("/{id}", get(handler::get_by_id))
        .route("/date/{date}", get(handler::get_by_date))
        .layer(middleware::from_fn(require_permission("reports:view")))
}
