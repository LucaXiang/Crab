//! Order API Module
//!
//! Read-only access to archived orders. All mutations go through OrderManager.

mod handler;

use axum::{
    Router,
    routing::get,
};

use crate::core::ServerState;

/// Order router
pub fn router() -> Router<ServerState> {
    Router::new().nest("/api/orders", routes())
}

fn routes() -> Router<ServerState> {
    // 订单历史查询：无需权限检查（基础操作）
    Router::new()
        // Order history (archived orders)
        .route("/history", get(handler::fetch_order_list))
        // Member spending history
        .route("/member/{member_id}/history", get(handler::fetch_member_history))
        // Order detail (archived)
        .route("/{id}", get(handler::get_by_id))
}
