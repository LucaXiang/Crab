//! Product API 模块

mod handler;

use axum::{
    Router,
    middleware,
    routing::{get, post, put},
};

use crate::auth::require_permission;
use crate::core::ServerState;

pub fn router() -> Router<ServerState> {
    Router::new().nest("/api/products", product_routes())
}

fn product_routes() -> Router<ServerState> {
    let read_routes = Router::new()
        .route("/", get(handler::list))
        .route("/{id}", get(handler::get_by_id))
        .route("/{id}/full", get(handler::get_full))
        .route("/{id}/attributes", get(handler::list_product_attributes))
        .route("/by-category/{category_id}", get(handler::list_by_category))
        .layer(middleware::from_fn(require_permission("products:read")));

    let write_routes = Router::new()
        .route("/", post(handler::create))
        .route("/sort-order", put(handler::batch_update_sort_order))
        .route("/{id}", put(handler::update))
        .route("/{id}/tags/{tag_id}", post(handler::add_product_tag))
        .layer(middleware::from_fn(require_permission("products:write")));

    let delete_routes = Router::new()
        .route("/{id}", axum::routing::delete(handler::delete))
        .route("/{id}/tags/{tag_id}", axum::routing::delete(handler::remove_product_tag))
        .layer(middleware::from_fn(require_permission("products:delete")));

    read_routes.merge(write_routes).merge(delete_routes)
}
