//! Store Info API Module

mod handler;

use axum::{
    Router,
    routing::get,
};

use crate::core::ServerState;

/// Store info router
pub fn router() -> Router<ServerState> {
    Router::new().nest("/api/store-info", routes())
}

fn routes() -> Router<ServerState> {
    Router::new()
        .route("/", get(handler::get).put(handler::update))
}
