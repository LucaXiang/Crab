//! Archive Verify API 模块 (归档哈希链验证)

mod handler;

use axum::{routing::get, Router};

use crate::core::ServerState;

pub fn router() -> Router<ServerState> {
    Router::new().nest("/api/archive/verify", routes())
}

fn routes() -> Router<ServerState> {
    // 归档验证：无需权限检查（基础操作）
    Router::new()
        .route("/order/{receipt_number}", get(handler::verify_order))
        .route("/daily/{date}", get(handler::verify_daily_chain))
}
