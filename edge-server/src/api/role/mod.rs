//! Role API Module

mod handler;

use axum::Router;
use axum::routing::get;

use crate::auth::require_admin;
use crate::core::ServerState;

/// Role router - requires authentication and admin access
pub fn router() -> Router<ServerState> {
    Router::new()
        .nest("/api/roles", roles_routes())
        .route("/api/permissions", get(handler::get_all_permissions))
        .layer(axum::middleware::from_fn(require_admin))
}

fn roles_routes() -> Router<ServerState> {
    Router::new()
        .route("/", get(handler::list).post(handler::create))
        .route(
            "/{id}",
            get(handler::get_by_id)
                .put(handler::update)
                .delete(handler::delete),
        )
        .route(
            "/{id}/permissions",
            get(handler::get_role_permissions).put(handler::update_role_permissions),
        )
}
