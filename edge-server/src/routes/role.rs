use axum::Router;
use axum::routing::get;

use crate::handler;
use crate::server::ServerState;
use crate::server::auth::require_admin;

/// Role router - requires authentication and admin access
pub fn router() -> Router<ServerState> {
    Router::new()
        .route("/api/role", get(handler::role::get))
        .route_layer(axum::middleware::from_fn(require_admin))
}
