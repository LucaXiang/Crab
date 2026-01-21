//! Authentication Routes

mod handler;

use axum::{Router, routing::get, routing::post};

use crate::core::ServerState;

/// Build authentication router
/// - /api/auth/login: public (no auth required)
/// - /api/auth/me, /api/auth/logout: public (auth middleware handled at Router level)
pub fn router() -> Router<ServerState> {
    Router::new()
        // Public route - no auth middleware applied
        .route("/api/auth/login", post(handler::login))
        // Protected routes - require authentication (handled by global require_auth middleware)
        .route("/api/auth/me", get(handler::me))
        .route("/api/auth/logout", post(handler::logout))
}
