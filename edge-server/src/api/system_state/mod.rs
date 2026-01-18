//! System State API Module

mod handler;

use axum::{
    routing::{get, post, put},
    Router,
};

use crate::core::ServerState;

/// System state router
pub fn router() -> Router<ServerState> {
    Router::new().nest("/api/system-state", routes())
}

fn routes() -> Router<ServerState> {
    Router::new()
        .route("/", get(handler::get).put(handler::update))
        .route("/genesis", post(handler::init_genesis))
        .route("/last-order", put(handler::update_last_order))
        .route("/sync-state", put(handler::update_sync_state))
        .route("/pending-sync", get(handler::get_pending_sync))
}
