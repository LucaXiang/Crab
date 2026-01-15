use axum::Router;
use axum::middleware as axum_middleware;
use http::{HeaderName, HeaderValue};
use tower_http::compression::CompressionLayer;
use tower_http::cors::CorsLayer;
use tower_http::request_id::{
    MakeRequestId, PropagateRequestIdLayer, RequestId, SetRequestIdLayer,
};
use tower_http::trace::TraceLayer;
use uuid::Uuid;

use crate::server::ServerState;
use crate::server::middleware;

pub mod audit;
pub mod auth;
pub mod health;
pub mod role;
pub mod upload;

pub mod router_ext;
pub use router_ext::{OneshotResult, OneshotRouter};

/// Custom request ID generator
#[derive(Clone)]
struct XRequestId;

impl MakeRequestId for XRequestId {
    fn make_request_id<B>(&mut self, _request: &http::Request<B>) -> Option<RequestId> {
        let id = Uuid::new_v4().to_string();
        Some(RequestId::new(HeaderValue::from_str(&id).unwrap()))
    }
}

/// Build a router with all routes registered (no middleware, no state)
pub fn build_router() -> Router<ServerState> {
    Router::new()
        // Audit API - authentication required
        .merge(audit::router())
        // Admin API - admin permission required
        .merge(role::router())
        // Auth API - authentication required
        .merge(auth::router())
        // Upload API - authentication required
        .merge(upload::router())
        // Health API - public route
        .merge(health::router())
}

/// Build a fully configured application with all middleware and state
///
/// This is used by both the HTTP server and oneshot calls
pub fn build_app(state: &ServerState) -> Router<ServerState> {
    build_router()
        // ========== Tower HTTP Middleware ==========
        // CORS - Handle cross-origin requests
        .layer(CorsLayer::permissive())
        // Compression - Gzip compress responses
        .layer(CompressionLayer::new())
        // Request logging - outermost, executed first
        .layer(axum_middleware::from_fn(middleware::logging_middleware))
        // Trace - Request tracing (logs at INFO level)
        .layer(TraceLayer::new_for_http())
        // ========== Application Middleware ==========
        // Request ID - Generate unique ID for each request
        .layer(SetRequestIdLayer::new(
            HeaderName::from_static("x-request-id"),
            XRequestId,
        ))
        // Propagate request ID to response
        .layer(PropagateRequestIdLayer::new(HeaderName::from_static(
            "x-request-id",
        )))
        // Get user context (JWT authentication) - executes before routes, injects CurrentUser
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            crate::server::auth::require_auth,
        ))
}
