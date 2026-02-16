//! Marketing Group API 模块

mod handler;

use axum::{
    Router, middleware,
    routing::{get, post, put},
};

use crate::auth::require_permission;
use crate::core::ServerState;

pub fn router() -> Router<ServerState> {
    Router::new().nest("/api/marketing-groups", routes())
}

fn routes() -> Router<ServerState> {
    // 读取路由：无需权限检查
    let read_routes = Router::new()
        .route("/", get(handler::list))
        .route("/{id}", get(handler::get_by_id));

    // 管理路由：需要 marketing_groups:manage 权限
    let manage_routes = Router::new()
        .route("/", post(handler::create))
        .route("/{id}", put(handler::update).delete(handler::delete))
        // 嵌套资源: 折扣规则
        .route("/{id}/discount-rules", post(handler::create_rule))
        .route(
            "/{id}/discount-rules/{rule_id}",
            put(handler::update_rule).delete(handler::delete_rule),
        )
        // 嵌套资源: 集章活动
        .route("/{id}/stamp-activities", post(handler::create_activity))
        .route(
            "/{id}/stamp-activities/{activity_id}",
            put(handler::update_activity).delete(handler::delete_activity),
        )
        .layer(middleware::from_fn(require_permission(
            "marketing_groups:manage",
        )));

    read_routes.merge(manage_routes)
}
