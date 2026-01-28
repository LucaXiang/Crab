//! Store Info API Module

mod handler;

use axum::{
    Router,
    middleware,
    routing::get,
};

use crate::auth::require_permission;
use crate::core::ServerState;

/// Store info router
pub fn router() -> Router<ServerState> {
    Router::new().nest("/api/store-info", routes())
}

fn routes() -> Router<ServerState> {
    let read_routes = Router::new()
        .route("/", get(handler::get))
        .layer(middleware::from_fn(require_permission("system:read")));

    let write_routes = Router::new()
        .route("/", axum::routing::put(handler::update))
        .layer(middleware::from_fn(require_permission("system:write")));

    read_routes.merge(write_routes)
}
