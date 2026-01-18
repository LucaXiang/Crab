//! Product API 模块

mod handler;

use axum::{
    Router,
    routing::{get, post},
};

use crate::core::ServerState;

pub fn router() -> Router<ServerState> {
    Router::new()
        .nest("/api/products", product_routes())
        .nest("/api/specs", spec_routes())
}

fn product_routes() -> Router<ServerState> {
    Router::new()
        .route("/", get(handler::list).post(handler::create))
        .route("/{id}", get(handler::get_by_id).put(handler::update).delete(handler::delete))
        .route("/{id}/full", get(handler::get_full))
        .route("/{id}/specs", get(handler::get_specs))
        .route("/by-category/{category_id}", get(handler::list_by_category))
}

fn spec_routes() -> Router<ServerState> {
    Router::new()
        .route("/", post(handler::create_spec))
        .route("/{id}", get(handler::get_spec).put(handler::update_spec).delete(handler::delete_spec))
        .route("/{id}/tags/{tag_id}", post(handler::add_tag).delete(handler::remove_tag))
}
