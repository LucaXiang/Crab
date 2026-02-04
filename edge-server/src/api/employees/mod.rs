//! Employee API Module

mod handler;

use axum::{Router, middleware, routing::get};

use crate::auth::require_admin;
use crate::core::ServerState;

/// Employee router
pub fn router() -> Router<ServerState> {
    Router::new().nest("/api/employees", routes())
}

fn routes() -> Router<ServerState> {
    // 读取路由：无需权限检查（查看员工列表是基础操作）
    let read_routes = Router::new()
        .route("/", get(handler::list))
        .route("/all", get(handler::list_with_inactive))
        .route("/{id}", get(handler::get_by_id));

    // 管理路由：仅管理员可用 (users:manage)
    let manage_routes = Router::new()
        .route("/", axum::routing::post(handler::create))
        .route("/{id}", axum::routing::put(handler::update).delete(handler::delete))
        .layer(middleware::from_fn(require_admin));

    read_routes.merge(manage_routes)
}
