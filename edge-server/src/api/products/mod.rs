//! Product API 模块

mod handler;

use axum::{
    Router,
    routing::{get, post},
};

use crate::core::ServerState;

pub fn router() -> Router<ServerState> {
    Router::new().nest("/api/products", product_routes())
}

fn product_routes() -> Router<ServerState> {
    Router::new()
        .route("/", get(handler::list).post(handler::create))
        .route(
            "/{id}",
            get(handler::get_by_id)
                .put(handler::update)
                .delete(handler::delete),
        )
        .route("/{id}/full", get(handler::get_full))
        .route("/{id}/attributes", get(handler::list_product_attributes))
        .route(
            "/{id}/tags/{tag_id}",
            post(handler::add_product_tag).delete(handler::remove_product_tag),
        )
        .route("/by-category/{category_id}", get(handler::list_by_category))
}
