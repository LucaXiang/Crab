//! Chain Entries API Module
//!
//! 统一 hash 链时间线查询 (ORDER + CREDIT_NOTE)，供 POS History 使用。

mod handler;

use axum::{Router, routing::get};

use crate::core::ServerState;

pub fn router() -> Router<ServerState> {
    Router::new().nest("/api/chain-entries", routes())
}

fn routes() -> Router<ServerState> {
    Router::new()
        .route("/", get(handler::list))
        .route("/credit-note/{id}", get(handler::get_credit_note_detail))
        .route("/anulacion/{id}", get(handler::get_anulacion_detail))
        .route("/upgrade/{id}", get(handler::get_upgrade_detail))
}
