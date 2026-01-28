//! Employee API Module

mod handler;

use axum::{Router, middleware, routing::get};

use crate::auth::require_permission;
use crate::core::ServerState;

/// Employee router
pub fn router() -> Router<ServerState> {
    Router::new().nest("/api/employees", routes())
}

fn routes() -> Router<ServerState> {
    let read_routes = Router::new()
        .route("/", get(handler::list))
        .route("/all", get(handler::list_with_inactive))
        .route("/{id}", get(handler::get_by_id))
        .layer(middleware::from_fn(require_permission("users:read")));

    let manage_routes = Router::new()
        .route("/", axum::routing::post(handler::create))
        .route("/{id}", axum::routing::put(handler::update).delete(handler::delete))
        .layer(middleware::from_fn(require_permission("users:manage")));

    read_routes.merge(manage_routes)
}
