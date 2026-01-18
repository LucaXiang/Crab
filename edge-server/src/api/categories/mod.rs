//! Category API 模块

mod handler;

use axum::{
    Router,
    routing::{get, post, put},
};

use crate::core::ServerState;

pub fn router() -> Router<ServerState> {
    Router::new()
        .nest("/api/categories", routes())
}

fn routes() -> Router<ServerState> {
    Router::new()
        .route("/", get(handler::list).post(handler::create))
        // Batch sort order update (must be before /{id} to avoid path conflicts)
        .route("/sort-order", put(handler::batch_update_sort_order))
        .route("/{id}", get(handler::get_by_id).put(handler::update).delete(handler::delete))
        // Category-Attribute binding routes
        .route("/{id}/attributes", get(handler::list_category_attributes))
        .route("/{id}/attributes/{attr_id}", post(handler::bind_category_attribute).delete(handler::unbind_category_attribute))
}
