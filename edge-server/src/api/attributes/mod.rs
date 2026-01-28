//! Attribute API 模块

mod handler;

use axum::{
    Router,
    middleware,
    routing::{get, post, put},
};

use crate::auth::require_permission;
use crate::core::ServerState;

pub fn router() -> Router<ServerState> {
    Router::new().nest("/api/attributes", routes())
}

fn routes() -> Router<ServerState> {
    let read_routes = Router::new()
        .route("/", get(handler::list))
        .route("/{id}", get(handler::get_by_id))
        .layer(middleware::from_fn(require_permission("attributes:read")));

    let manage_routes = Router::new()
        .route("/", post(handler::create))
        .route("/{id}", put(handler::update).delete(handler::delete))
        .route("/{id}/options", post(handler::add_option))
        .route(
            "/{id}/options/{idx}",
            put(handler::update_option).delete(handler::remove_option),
        )
        .layer(middleware::from_fn(require_permission("attributes:manage")));

    read_routes.merge(manage_routes)
}
