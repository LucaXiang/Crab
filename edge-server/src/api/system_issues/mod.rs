//! System Issues API 模块 (系统问题查询与回应)

mod handler;

use axum::{routing::{get, post}, Router};

use crate::core::ServerState;

pub fn router() -> Router<ServerState> {
    Router::new().nest("/api/system-issues", routes())
}

fn routes() -> Router<ServerState> {
    // 任何已登录用户都可以查看和回应系统问题
    Router::new()
        .route("/pending", get(handler::pending))
        .route("/resolve", post(handler::resolve))
}
