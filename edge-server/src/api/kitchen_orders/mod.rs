//! Kitchen Orders API Module
//!
//! Provides REST endpoints for kitchen order and label record management.

mod handler;

use axum::{Router, routing::get, routing::post};

use crate::core::ServerState;

pub fn router() -> Router<ServerState> {
    Router::new()
        .nest("/api/kitchen-orders", kitchen_routes())
        .nest("/api/label-records", label_routes())
}

fn kitchen_routes() -> Router<ServerState> {
    Router::new()
        .route("/", get(handler::list))
        .route("/{id}", get(handler::get_by_id))
        .route("/{id}/reprint", post(handler::reprint))
}

fn label_routes() -> Router<ServerState> {
    Router::new()
        .route("/", get(handler::list_labels))
        .route("/{id}", get(handler::get_label_by_id))
        .route("/{id}/reprint", post(handler::reprint_label))
}
