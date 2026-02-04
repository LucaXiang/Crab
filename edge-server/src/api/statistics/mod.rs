//! Statistics API 模块 (数据统计)

mod handler;

use axum::{
    Router,
    middleware,
    routing::get,
};

use crate::auth::require_permission;
use crate::core::ServerState;

pub fn router() -> Router<ServerState> {
    Router::new().nest("/api/statistics", routes())
}

fn routes() -> Router<ServerState> {
    // 报表查看：需要 reports:view 权限
    Router::new()
        .route("/", get(handler::get_statistics))
        .route("/sales-report", get(handler::get_sales_report))
        .layer(middleware::from_fn(require_permission("reports:view")))
}
