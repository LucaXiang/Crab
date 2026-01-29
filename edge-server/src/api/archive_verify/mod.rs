//! Archive Verify API 模块 (归档哈希链验证)

mod handler;

use axum::{middleware, routing::get, Router};

use crate::auth::require_permission;
use crate::core::ServerState;

pub fn router() -> Router<ServerState> {
    Router::new().nest("/api/archive/verify", routes())
}

fn routes() -> Router<ServerState> {
    Router::new()
        .route("/order/{receipt_number}", get(handler::verify_order))
        .route("/daily/{date}", get(handler::verify_daily_chain))
        .layer(middleware::from_fn(require_permission("orders:read")))
}
