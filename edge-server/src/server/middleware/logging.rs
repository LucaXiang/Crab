//! Request Logging Middleware
//!
//! Logs all incoming HTTP requests with timing, user info, and status codes

use axum::{
    extract::{MatchedPath, Request},
    middleware::Next,
    response::Response,
};
use std::time::Instant;
use tracing::{info, warn};

/// Request logging middleware
///
/// Logs request start and completion with the following info:
/// - Request ID (x-request-id from tower-http)
/// - HTTP method and path
/// - User agent
/// - Authenticated user (if available)
/// - Response status code
/// - Request latency in milliseconds
pub async fn logging_middleware(
    req: Request,
    next: Next,
) -> Response {
    let start = Instant::now();

    // Get request ID from header, or generate one if not present
    let request_id = req
        .headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    let method = req.method().clone();
    let uri = req.uri().clone();
    let path = req
        .extensions()
        .get::<MatchedPath>()
        .map(|p| p.as_str().to_string())
        .unwrap_or_else(|| uri.path().to_string());

    let user_agent = req
        .headers()
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown");

    // Extract user information if authenticated
    let user_info = req
        .extensions()
        .get::<crate::server::auth::CurrentUser>()
        .map(|u| format!("{}({})", u.username, u.id));

    info!(
        request_id = %request_id,
        method = %method,
        path = %path,
        user_agent = %user_agent,
        user = ?user_info,
        "Request started"
    );

    let response = next.run(req).await;

    let latency = start.elapsed();
    let status = response.status();

    // Log with different levels based on status code
    if status.is_server_error() {
        warn!(
            request_id = %request_id,
            method = %method,
            path = %path,
            status = %status.as_u16(),
            latency_ms = %latency.as_millis(),
            user = ?user_info,
            "Request completed with server error"
        );
    } else if status.is_client_error() {
        warn!(
            request_id = %request_id,
            method = %method,
            path = %path,
            status = %status.as_u16(),
            latency_ms = %latency.as_millis(),
            user = ?user_info,
            "Request completed with client error"
        );
    } else {
        info!(
            request_id = %request_id,
            method = %method,
            path = %path,
            status = %status.as_u16(),
            latency_ms = %latency.as_millis(),
            user = ?user_info,
            "Request completed successfully"
        );
    }

    response
}
