//! Print Config API 模块
//!
//! System default printer configuration management.

mod handler;

use axum::{Router, routing::get};

use crate::core::ServerState;

pub fn router() -> Router<ServerState> {
    Router::new().nest("/api/print-config", routes())
}

fn routes() -> Router<ServerState> {
    Router::new().route("/", get(handler::get).put(handler::update))
}
