//! Tag API 模块

mod handler;

use axum::{Router, middleware, routing::get};

use crate::auth::require_permission;
use crate::core::ServerState;

pub fn router() -> Router<ServerState> {
    Router::new().nest("/api/tags", routes())
}

fn routes() -> Router<ServerState> {
    let read_routes = Router::new()
        .route("/", get(handler::list))
        .route("/{id}", get(handler::get_by_id))
        .layer(middleware::from_fn(require_permission("products:read")));

    let write_routes = Router::new()
        .route("/", axum::routing::post(handler::create))
        .route("/{id}", axum::routing::put(handler::update))
        .layer(middleware::from_fn(require_permission("products:write")));

    let delete_routes = Router::new()
        .route("/{id}", axum::routing::delete(handler::delete))
        .layer(middleware::from_fn(require_permission("products:delete")));

    read_routes.merge(write_routes).merge(delete_routes)
}
