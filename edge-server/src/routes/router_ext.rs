//! Router extension for oneshot calls
//!
//! Provides the ability to call the Router directly without going through
//! the network stack.

use http::Response;
use tower::Service;

use crate::server::ServerState;
use anyhow::Result;
use axum::Router;
use axum::body::Body;
use http::Request;

/// Result type for oneshot API calls
pub type OneshotResult = Result<Response<Body>>;

/// Extension trait for Router to support oneshot calls
///
/// This trait provides the `oneshot` method that allows processing
/// HTTP requests directly without going through the network stack.
#[async_trait::async_trait]
pub trait OneshotRouter {
    /// Process a request using oneshot pattern
    ///
    /// # Example
    ///
    /// ```ignore
    /// use http::Request;
    ///
    /// let state = ServerState::initialize(&config).await;
    /// let request = Request::builder()
    ///     .uri("/health")
    ///     .body(Body::empty())?;
    ///
    /// let response = state.oneshot(request).await?;
    /// ```
    async fn oneshot(&mut self, state: &ServerState, request: Request<Body>) -> OneshotResult;
}

#[async_trait::async_trait]
impl OneshotRouter for Router<ServerState> {
    async fn oneshot(&mut self, state: &ServerState, request: Request<Body>) -> OneshotResult {
        // Clone router and apply state, then call as Service
        let mut svc = self.clone().with_state(state.clone());
        let response = svc.call(request).await?;
        Ok(response)
    }
}
