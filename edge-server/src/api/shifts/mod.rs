//! Shift API 模块 (班次管理)

mod handler;

use axum::{
    Router,
    middleware,
    routing::{get, post},
};

use crate::auth::require_permission;
use crate::core::ServerState;

pub fn router() -> Router<ServerState> {
    Router::new().nest("/api/shifts", routes())
}

fn routes() -> Router<ServerState> {
    let read_routes = Router::new()
        .route("/", get(handler::list))
        .route("/current", get(handler::get_current))
        .route("/{id}", get(handler::get_by_id))
        .layer(middleware::from_fn(require_permission("system:read")));

    let write_routes = Router::new()
        .route("/", post(handler::create))
        .route("/recover", post(handler::recover_stale))
        .route("/{id}", axum::routing::put(handler::update))
        .route("/{id}/close", post(handler::close))
        .route("/{id}/force-close", post(handler::force_close))
        .route("/{id}/heartbeat", post(handler::heartbeat))
        .layer(middleware::from_fn(require_permission("system:write")));

    read_routes.merge(write_routes)
}
