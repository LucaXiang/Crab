//! HasAttribute API 模块 - 产品属性绑定

mod handler;

use axum::{
    Router,
    routing::{get, post},
};

use crate::core::ServerState;

pub fn router() -> Router<ServerState> {
    Router::new()
        .nest("/api/has-attribute", routes())
}

fn routes() -> Router<ServerState> {
    Router::new()
        .route("/", post(handler::create))
        .route("/{id}", get(handler::get_by_id).put(handler::update).delete(handler::delete))
        .route("/product/{product_id}", get(handler::list_by_product))
}
