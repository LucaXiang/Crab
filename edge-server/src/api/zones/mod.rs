//! Zone API 模块

mod handler;

use axum::{Router, middleware, routing::get};

use crate::auth::require_permission;
use crate::core::ServerState;

pub fn router() -> Router<ServerState> {
    Router::new().nest("/api/zones", routes())
}

fn routes() -> Router<ServerState> {
    // 读取路由：无需权限检查
    let read_routes = Router::new()
        .route("/", get(handler::list))
        .route("/{id}", get(handler::get_by_id))
        .route("/{id}/tables", get(handler::list_tables));

    // 管理路由：需要 tables:manage 权限
    let manage_routes = Router::new()
        .route("/", axum::routing::post(handler::create))
        .route(
            "/{id}",
            axum::routing::put(handler::update).delete(handler::delete),
        )
        .layer(middleware::from_fn(require_permission("tables:manage")));

    read_routes.merge(manage_routes)
}
