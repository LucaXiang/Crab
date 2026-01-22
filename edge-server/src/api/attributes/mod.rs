//! Attribute API 模块

mod handler;

use axum::{
    Router,
    routing::{get, post, put},
};

use crate::core::ServerState;

pub fn router() -> Router<ServerState> {
    Router::new().nest("/api/attributes", routes())
}

fn routes() -> Router<ServerState> {
    Router::new()
        .route("/", get(handler::list).post(handler::create))
        .route(
            "/{id}",
            get(handler::get_by_id)
                .put(handler::update)
                .delete(handler::delete),
        )
        .route("/{id}/options", post(handler::add_option))
        .route(
            "/{id}/options/{idx}",
            put(handler::update_option).delete(handler::remove_option),
        )
}
