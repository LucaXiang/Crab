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
    Router::new()
        // Order history (archived orders)
        .route("/history", get(handler::fetch_order_list))
        // Order detail (archived)
        .route("/{id}", get(handler::get_by_id))
}
