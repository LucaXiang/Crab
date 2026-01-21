//! Price Rule API 模块

mod handler;

use axum::{Router, routing::get};

use crate::core::ServerState;

pub fn router() -> Router<ServerState> {
    Router::new().nest("/api/price-rules", routes())
}

fn routes() -> Router<ServerState> {
    Router::new()
        .route("/", get(handler::list).post(handler::create))
        .route(
            "/{id}",
            get(handler::get_by_id)
                .put(handler::update)
                .delete(handler::delete),
        )
        .route("/by-scope/{scope}", get(handler::list_by_scope))
        .route("/for-product/{product_id}", get(handler::list_for_product))
}
