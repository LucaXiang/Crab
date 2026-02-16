//! Price Rule API 模块

mod handler;

use axum::{Router, middleware, routing::get};

use crate::auth::require_permission;
use crate::core::ServerState;

pub fn router() -> Router<ServerState> {
    Router::new().nest("/api/price-rules", routes())
}

fn routes() -> Router<ServerState> {
    // 读取路由：无需权限检查（查看价格规则是基础操作）
    let read_routes = Router::new()
        .route("/", get(handler::list))
        .route("/{id}", get(handler::get_by_id))
        .route("/by-scope/{scope}", get(handler::list_by_scope))
        .route("/for-product/{product_id}", get(handler::list_for_product));

    // 写入路由：需要 price_rules:manage 权限
    let write_routes = Router::new()
        .route("/", axum::routing::post(handler::create))
        .route(
            "/{id}",
            axum::routing::put(handler::update).delete(handler::delete),
        )
        .layer(middleware::from_fn(require_permission(
            "price_rules:manage",
        )));

    read_routes.merge(write_routes)
}
