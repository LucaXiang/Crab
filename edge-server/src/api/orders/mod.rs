//! Order API Module

mod handler;

use axum::{
    Router,
    routing::{get, post, put},
};

use crate::core::ServerState;

/// Order router
pub fn router() -> Router<ServerState> {
    Router::new().nest("/api/orders", routes())
}

fn routes() -> Router<ServerState> {
    Router::new()
        // List & query
        .route("/", get(handler::list))
        .route("/history", get(handler::fetch_order_list))
        .route("/last", get(handler::get_last))
        .route("/verify", get(handler::verify_chain))
        .route("/receipt/{receipt}", get(handler::get_by_receipt))
        // Single order operations
        .route("/{id}", get(handler::get_by_id))
        .route(
            "/{id}/items",
            post(handler::add_item).delete(handler::remove_item),
        )
        .route("/{id}/payments", post(handler::add_payment))
        .route("/{id}/totals", put(handler::update_totals))
        .route("/{id}/hash", put(handler::update_hash))
        .route(
            "/{id}/events",
            get(handler::get_events).post(handler::add_event),
        )
}
