//! Role API Module

mod handler;

use axum::{Router, middleware, routing::get};

use crate::auth::require_admin;
use crate::core::ServerState;

/// Role router - role management is admin-only
pub fn router() -> Router<ServerState> {
    // 读取路由：无需权限检查（查看角色列表）
    let read_routes = Router::new()
        .nest("/api/roles", roles_read_routes())
        .route("/api/permissions", get(handler::get_all_permissions));

    // 写入路由：仅管理员可用 (users:manage)
    let write_routes = Router::new()
        .nest("/api/roles", roles_write_routes())
        .layer(middleware::from_fn(require_admin));

    read_routes.merge(write_routes)
}

fn roles_read_routes() -> Router<ServerState> {
    Router::new()
        .route("/", get(handler::list))
        .route("/{id}", get(handler::get_by_id))
        .route("/{id}/permissions", get(handler::get_role_permissions))
}

fn roles_write_routes() -> Router<ServerState> {
    Router::new()
        .route("/", axum::routing::post(handler::create))
        .route("/{id}", axum::routing::put(handler::update).delete(handler::delete))
        .route("/{id}/permissions", axum::routing::put(handler::update_role_permissions))
}
