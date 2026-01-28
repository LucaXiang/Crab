//! Sync API 模块

mod handler;

use axum::{Router, middleware, routing::get};

use crate::auth::require_permission;
use crate::core::ServerState;

pub fn router() -> Router<ServerState> {
    Router::new().nest("/api/sync", routes())
}

fn routes() -> Router<ServerState> {
    Router::new()
        .route("/status", get(handler::get_sync_status))
        .layer(middleware::from_fn(require_permission("system:read")))
}
