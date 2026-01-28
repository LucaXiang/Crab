//! Role API Module

mod handler;

use axum::{Router, middleware, routing::get};

use crate::auth::require_permission;
use crate::core::ServerState;

/// Role router - requires authentication and role permissions
pub fn router() -> Router<ServerState> {
    let read_routes = Router::new()
        .nest("/api/roles", roles_read_routes())
        .route("/api/permissions", get(handler::get_all_permissions))
        .layer(middleware::from_fn(require_permission("roles:read")));

    let write_routes = Router::new()
        .nest("/api/roles", roles_write_routes())
        .layer(middleware::from_fn(require_permission("roles:write")));

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
