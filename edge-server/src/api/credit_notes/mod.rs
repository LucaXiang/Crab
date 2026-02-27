//! Credit Notes API Module
//!
//! 退款凭证管理 — 创建退款、查询退款记录

mod handler;

use axum::{
    Router,
    routing::{get, post},
};

use crate::core::ServerState;

/// Credit notes router
pub fn router() -> Router<ServerState> {
    Router::new().nest("/api/credit-notes", routes())
}

fn routes() -> Router<ServerState> {
    Router::new()
        .route("/", post(handler::create))
        .route("/{id}", get(handler::get_by_id))
        .route("/{id}/receipt", get(handler::get_receipt))
        .route("/by-order/{order_pk}", get(handler::list_by_order))
        .route("/refundable/{order_pk}", get(handler::get_refundable_info))
}
