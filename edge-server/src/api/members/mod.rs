//! Member API 模块

mod handler;

use axum::{middleware, routing::{get, post, put}, Router};

use crate::auth::require_permission;
use crate::core::ServerState;

pub fn router() -> Router<ServerState> {
    Router::new().nest("/api/members", routes())
}

fn routes() -> Router<ServerState> {
    // 读取路由：无需权限检查
    let read_routes = Router::new()
        .route("/", get(handler::list))
        .route("/search", get(handler::search))
        .route("/{id}", get(handler::get_by_id));

    // 管理路由：需要 members:manage 权限
    let manage_routes = Router::new()
        .route("/", post(handler::create))
        .route("/{id}", put(handler::update).delete(handler::delete))
        .layer(middleware::from_fn(require_permission("members:manage")));

    read_routes.merge(manage_routes)
}
