//! Shift API 模块 (班次管理)

mod handler;

use axum::{
    Router,
    routing::{get, post},
};

use crate::core::ServerState;

pub fn router() -> Router<ServerState> {
    Router::new().nest("/api/shifts", routes())
}

fn routes() -> Router<ServerState> {
    Router::new()
        .route("/", get(handler::list).post(handler::create))
        .route("/current", get(handler::get_current))
        .route("/recover", post(handler::recover_stale))
        .route("/{id}", get(handler::get_by_id).put(handler::update))
        .route("/{id}/close", post(handler::close))
        .route("/{id}/force-close", post(handler::force_close))
        .route("/{id}/heartbeat", post(handler::heartbeat))
}
