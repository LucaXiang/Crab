//! Authentication Routes

mod handler;

use axum::{Router, routing::get, routing::post};

use crate::core::ServerState;

/// Build authentication router
/// - /api/auth/login: public (no auth required)
/// - /api/auth/me, /api/auth/logout: protected (require auth)
pub fn router() -> Router<ServerState> {
    Router::new()
        // Public route - no auth middleware applied
        .route("/api/auth/login", post(handler::login))
        // Protected routes - require authentication
        .route("/api/auth/me", get(handler::me))
        .route("/api/auth/logout", post(handler::logout))
}
