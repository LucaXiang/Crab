//! Credit Notes API Module
//!
//! 退款凭证管理 — 创建退款、查询退款记录

mod handler;

use axum::{
    Router, middleware,
    routing::{get, post},
};

use crate::auth::require_permission;
use crate::core::ServerState;

/// Credit notes router
pub fn router() -> Router<ServerState> {
    Router::new().nest("/api/credit-notes", routes())
}

fn routes() -> Router<ServerState> {
    // 读取路由：查看退款记录是基础操作
    let read_routes = Router::new()
        .route("/{id}", get(handler::get_by_id))
        .route("/{id}/receipt", get(handler::get_receipt))
        .route("/by-order/{order_pk}", get(handler::list_by_order))
        .route("/refundable/{order_pk}", get(handler::get_refundable_info));

    // 写入路由：需要 orders:refund 权限
    let write_routes = Router::new()
        .route("/", post(handler::create))
        .layer(middleware::from_fn(require_permission("orders:refund")));

    read_routes.merge(write_routes)
}
