//! Dining Table API 模块

mod handler;

use axum::{
    Router,
    routing::get,
};

use crate::core::ServerState;

pub fn router() -> Router<ServerState> {
    Router::new()
        .nest("/api/tables", routes())
}

fn routes() -> Router<ServerState> {
    Router::new()
        .route("/", get(handler::list).post(handler::create))
        .route("/{id}", get(handler::get_by_id).put(handler::update).delete(handler::delete))
}
