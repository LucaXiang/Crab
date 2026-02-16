//! Store Info API Module

mod handler;

use axum::{Router, middleware, routing::get};

use crate::auth::require_permission;
use crate::core::ServerState;

/// Store info router
pub fn router() -> Router<ServerState> {
    Router::new().nest("/api/store-info", routes())
}

fn routes() -> Router<ServerState> {
    // 读取路由：无需权限检查
    let read_routes = Router::new().route("/", get(handler::get));

    // 写入路由：需要 settings:manage 权限
    let write_routes = Router::new()
        .route("/", axum::routing::put(handler::update))
        .layer(middleware::from_fn(require_permission("settings:manage")));

    read_routes.merge(write_routes)
}
