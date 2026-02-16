//! API routes for crab-cloud

pub mod health;
pub mod sync;

use crate::auth::edge_auth::edge_auth_middleware;
use crate::state::AppState;
use axum::{Router, middleware};

/// Create the combined router
pub fn create_router(state: AppState) -> Router {
    let edge = Router::new()
        .route("/api/edge/sync", axum::routing::post(sync::handle_sync))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            edge_auth_middleware,
        ));

    Router::new()
        .route("/health", axum::routing::get(health::health_check))
        .merge(edge)
        .with_state(state)
}
