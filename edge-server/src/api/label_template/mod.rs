//! Label Template API Module

mod handler;

use axum::{
    Router,
    routing::get,
};

use crate::core::ServerState;

/// Label template router
pub fn router() -> Router<ServerState> {
    Router::new().nest("/api/label-templates", routes())
}

fn routes() -> Router<ServerState> {
    Router::new()
        .route("/", get(handler::list).post(handler::create))
        .route("/all", get(handler::list_all))
        .route("/default", get(handler::get_default))
        .route("/{id}", get(handler::get_by_id).put(handler::update).delete(handler::delete))
}
