//! Audit Log Routes
//!
//! Provides endpoints for manual audit log entries.

use axum::{Router, routing::post};

use crate::handler;
use crate::server::ServerState;

/// Build audit router
pub fn router() -> Router<ServerState> {
    Router::new()
        // Create manual audit log - authentication required
        .route("/api/audit", post(handler::audit::create_audit_log))
}
