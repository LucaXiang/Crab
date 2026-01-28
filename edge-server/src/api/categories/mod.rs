//! Category API 模块

mod handler;

use axum::{
    Router,
    middleware,
    routing::{get, post, put},
};

use crate::auth::require_permission;
use crate::core::ServerState;

pub fn router() -> Router<ServerState> {
    Router::new().nest("/api/categories", routes())
}

fn routes() -> Router<ServerState> {
    let read_routes = Router::new()
        .route("/", get(handler::list))
        .route("/{id}", get(handler::get_by_id))
        .route("/{id}/attributes", get(handler::list_category_attributes))
        .layer(middleware::from_fn(require_permission("categories:read")));

    let manage_routes = Router::new()
        .route("/", post(handler::create))
        // Batch sort order update
        .route("/sort-order", put(handler::batch_update_sort_order))
        .route("/{id}", put(handler::update).delete(handler::delete))
        // Category-Attribute binding routes
        .route(
            "/{id}/attributes/{attr_id}",
            post(handler::bind_category_attribute).delete(handler::unbind_category_attribute),
        )
        .layer(middleware::from_fn(require_permission("categories:manage")));

    read_routes.merge(manage_routes)
}
