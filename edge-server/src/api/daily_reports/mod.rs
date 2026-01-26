//! Daily Report API 模块 (日结报告)

mod handler;

use axum::{
    Router,
    routing::{get, post},
};

use crate::core::ServerState;

pub fn router() -> Router<ServerState> {
    Router::new().nest("/api/daily-reports", routes())
}

fn routes() -> Router<ServerState> {
    Router::new()
        .route("/", get(handler::list))
        .route("/generate", post(handler::generate))
        .route("/{id}", get(handler::get_by_id).delete(handler::delete))
        .route("/date/{date}", get(handler::get_by_date))
}
