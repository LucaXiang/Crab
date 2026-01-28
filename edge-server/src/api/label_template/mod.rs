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
    let read_routes = Router::new()
        .route("/", get(handler::list))
        .route("/all", get(handler::list_all))
        .route("/default", get(handler::get_default))
        .route("/{id}", get(handler::get_by_id))
        .layer(middleware::from_fn(require_permission("system:read")));

    let write_routes = Router::new()
        .route("/", axum::routing::post(handler::create))
        .route("/{id}", axum::routing::put(handler::update).delete(handler::delete))
        .layer(middleware::from_fn(require_permission("system:write")));

    read_routes.merge(write_routes)
}
