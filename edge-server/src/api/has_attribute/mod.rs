//! AttributeBinding API 模块 - 产品属性绑定

mod handler;

use axum::{
    Router,
    middleware,
    routing::{get, post},
};

use crate::auth::require_permission;
use crate::core::ServerState;

pub fn router() -> Router<ServerState> {
    Router::new().nest("/api/has-attribute", routes())
}

fn routes() -> Router<ServerState> {
    let read_routes = Router::new()
        .route("/{id}", get(handler::get_by_id))
        .route("/product/{product_id}", get(handler::list_by_product))
        .layer(middleware::from_fn(require_permission("attributes:read")));

    let manage_routes = Router::new()
        .route("/", post(handler::create))
        .route("/{id}", axum::routing::put(handler::update).delete(handler::delete))
        .layer(middleware::from_fn(require_permission("attributes:manage")));

    read_routes.merge(manage_routes)
}
