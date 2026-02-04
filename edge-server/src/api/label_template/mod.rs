//! Label Template API Module

mod handler;

use axum::{
    Router,
    middleware,
    routing::get,
};

use crate::auth::require_permission;
use crate::core::ServerState;

/// Label template router
pub fn router() -> Router<ServerState> {
    Router::new().nest("/api/label-templates", routes())
}

fn routes() -> Router<ServerState> {
    // 读取路由：无需权限检查（查看标签模板是基础操作）
    let read_routes = Router::new()
        .route("/", get(handler::list))
        .route("/all", get(handler::list_all))
        .route("/default", get(handler::get_default))
        .route("/{id}", get(handler::get_by_id));

    // 写入路由：需要 settings:manage 权限
    let write_routes = Router::new()
        .route("/", axum::routing::post(handler::create))
        .route("/{id}", axum::routing::put(handler::update).delete(handler::delete))
        .layer(middleware::from_fn(require_permission("settings:manage")));

    read_routes.merge(write_routes)
}
