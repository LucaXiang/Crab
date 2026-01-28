//! Print Config API 模块
//!
//! System default printer configuration management.

mod handler;

use axum::{Router, middleware, routing::get};

use crate::auth::require_permission;
use crate::core::ServerState;

pub fn router() -> Router<ServerState> {
    Router::new().nest("/api/print-config", routes())
}

fn routes() -> Router<ServerState> {
    let read_routes = Router::new()
        .route("/", get(handler::get))
        .layer(middleware::from_fn(require_permission("printers:read")));

    let manage_routes = Router::new()
        .route("/", axum::routing::put(handler::update))
        .layer(middleware::from_fn(require_permission("printers:manage")));

    read_routes.merge(manage_routes)
}
