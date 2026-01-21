//! Employee API Module

mod handler;

use axum::{Router, routing::get};

use crate::core::ServerState;

/// Employee router
pub fn router() -> Router<ServerState> {
    Router::new().nest("/api/employees", routes())
}

fn routes() -> Router<ServerState> {
    Router::new()
        .route("/", get(handler::list).post(handler::create))
        .route("/all", get(handler::list_with_inactive))
        .route(
            "/{id}",
            get(handler::get_by_id)
                .put(handler::update)
                .delete(handler::delete),
        )
}
