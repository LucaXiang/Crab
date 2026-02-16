//! Sync API 模块

mod handler;

use axum::{Router, routing::get};

use crate::core::ServerState;

pub fn router() -> Router<ServerState> {
    Router::new().nest("/api/sync", routes())
}

fn routes() -> Router<ServerState> {
    // 同步状态：无需权限检查（基础操作）
    Router::new().route("/status", get(handler::get_sync_status))
}
