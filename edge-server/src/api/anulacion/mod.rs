//! Invoice Anulación API Module
//!
//! 发票作废 (RegistroFacturaBaja) — 创建作废、查询状态

mod handler;

use axum::{
    Router, middleware,
    routing::{get, post},
};

use crate::auth::require_permission;
use crate::core::ServerState;

/// Anulacion router
pub fn router() -> Router<ServerState> {
    Router::new().nest("/api/anulacion", routes())
}

fn routes() -> Router<ServerState> {
    // 读取路由：查看作废状态是基础操作
    let read_routes = Router::new()
        .route("/eligibility/{order_pk}", get(handler::check_eligibility))
        .route("/by-order/{order_pk}", get(handler::get_by_order));

    // 写入路由：需要 orders:void 权限
    let write_routes = Router::new()
        .route("/", post(handler::create))
        .layer(middleware::from_fn(require_permission("orders:void")));

    read_routes.merge(write_routes)
}
