mod handler;

use axum::Router;
use axum::routing::get;

use crate::auth::require_admin;
use crate::core::ServerState;

/// Role router - requires authentication and admin access
pub fn router() -> Router<ServerState> {
    Router::new()
        .route("/api/role", get(handler::get))
        .layer(axum::middleware::from_fn(require_admin))
}
