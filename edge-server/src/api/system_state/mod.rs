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
    let read_routes = Router::new()
        .route("/", get(handler::get))
        .route("/pending-sync", get(handler::get_pending_sync))
        .layer(middleware::from_fn(require_permission("system:read")));

    let write_routes = Router::new()
        .route("/", put(handler::update))
        .route("/genesis", post(handler::init_genesis))
        .route("/last-order", put(handler::update_last_order))
        .route("/sync-state", put(handler::update_sync_state))
        .layer(middleware::from_fn(require_permission("system:write")));

    read_routes.merge(write_routes)
}
