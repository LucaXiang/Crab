//! System State API Module

mod handler;

use axum::{
    Router,
    middleware,
    routing::{get, post, put},
};

use crate::auth::require_permission;
use crate::core::ServerState;

/// System state router
pub fn router() -> Router<ServerState> {
    Router::new().nest("/api/system-state", routes())
}

fn routes() -> Router<ServerState> {
    // 读取路由：无需权限检查（查看系统状态是基础操作）
    let read_routes = Router::new()
        .route("/", get(handler::get));

    // 写入路由：需要 settings:manage 权限
    let write_routes = Router::new()
        .route("/", put(handler::update))
        .route("/genesis", post(handler::init_genesis))
        .route("/last-order", put(handler::update_last_order))
        .route("/sync-state", put(handler::update_sync_state))
        .layer(middleware::from_fn(require_permission("settings:manage")));

    read_routes.merge(write_routes)
}
