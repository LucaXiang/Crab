//! Audit Log API 模块 (审计日志查询、验证、启动异常确认)

mod handler;

use axum::{middleware, routing::{get, post}, Router};

use crate::auth::require_permission;
use crate::core::ServerState;

pub fn router() -> Router<ServerState> {
    Router::new().nest("/api/audit-log", routes())
}

fn routes() -> Router<ServerState> {
    Router::new()
        // 管理员权限：日志查询和链验证
        .route("/", get(handler::list))
        .route("/verify", get(handler::verify_chain))
        .route_layer(middleware::from_fn(require_permission("settings:manage")))
        // 普通认证：启动异常确认（任何登录用户都需要回应）
        .route("/pending-startup", get(handler::pending_startup))
        .route("/acknowledge-startup", post(handler::acknowledge_startup))
}
