//! Invoice Upgrade API Module
//!
//! F2 → F3 Sustitutiva — 升级简化发票为完整发票

mod handler;

use axum::{
    Router, middleware,
    routing::{get, post},
};

use crate::auth::require_permission;
use crate::core::ServerState;

/// Upgrade router
pub fn router() -> Router<ServerState> {
    Router::new().nest("/api/invoices/upgrade", routes())
}

fn routes() -> Router<ServerState> {
    // 读取路由：查看升级资格是基础操作
    let read_routes =
        Router::new().route("/eligibility/{order_pk}", get(handler::check_eligibility));

    // 写入路由：需要 settings:manage 权限
    let write_routes = Router::new()
        .route("/", post(handler::create))
        .layer(middleware::from_fn(require_permission("settings:manage")));

    read_routes.merge(write_routes)
}
