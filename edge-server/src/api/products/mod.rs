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
    // 读取路由：无需权限检查（基础操作）
    let read_routes = Router::new()
        .route("/", get(handler::list))
        .route("/{id}", get(handler::get_by_id))
        .route("/{id}/full", get(handler::get_full))
        .route("/{id}/attributes", get(handler::list_product_attributes))
        .route("/by-category/{category_id}", get(handler::list_by_category));

    // 写入/删除路由：需要 menu:manage 权限
    let manage_routes = Router::new()
        .route("/", post(handler::create))
        .route("/sort-order", put(handler::batch_update_sort_order))
        .route("/{id}", put(handler::update))
        .route("/{id}/tags/{tag_id}", post(handler::add_product_tag))
        .route("/{id}", axum::routing::delete(handler::delete))
        .route("/{id}/tags/{tag_id}", axum::routing::delete(handler::remove_product_tag))
        .layer(middleware::from_fn(require_permission("menu:manage")));

    read_routes.merge(manage_routes)
}
