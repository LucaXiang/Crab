//! API routes for crab-cloud

pub mod health;
pub mod register;
pub mod stripe_webhook;
pub mod sync;

use crate::auth::edge_auth::edge_auth_middleware;
use crate::state::AppState;
use axum::routing::{get, post};
use axum::{Router, middleware};

/// Create the combined router
pub fn create_router(state: AppState) -> Router {
    // Edge-server sync (mTLS authenticated)
    let edge = Router::new()
        .route("/api/edge/sync", post(sync::handle_sync))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            edge_auth_middleware,
        ));

    // Public registration (no auth)
    let registration = Router::new()
        .route("/api/register", post(register::register))
        .route("/api/verify-email", post(register::verify_email))
        .route("/api/resend-code", post(register::resend_code));

    // Stripe webhook (signature-verified, raw body)
    let webhook = Router::new().route("/stripe/webhook", post(stripe_webhook::handle_webhook));

    Router::new()
        .route("/health", get(health::health_check))
        .merge(registration)
        .merge(webhook)
        .merge(edge)
        .with_state(state)
}
