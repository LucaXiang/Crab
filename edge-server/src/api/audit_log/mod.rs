//! Audit Log API 模块 (审计日志查询)

mod handler;

use axum::{middleware, routing::get, Router};

use crate::auth::require_permission;
use crate::core::ServerState;

pub fn router() -> Router<ServerState> {
    Router::new().nest("/api/audit-log", routes())
}

fn routes() -> Router<ServerState> {
    Router::new()
        .route("/", get(handler::list))
        .route_layer(middleware::from_fn(require_permission("settings:manage")))
}
