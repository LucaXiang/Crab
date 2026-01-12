//! Oneshot client backend for CrabClient
//!
//! Makes in-process HTTP calls directly to the router without network overhead.
//! Useful when client and server are in the same process.

use async_trait::async_trait;
use axum::body::to_bytes;
use http::Request;
use serde::Serialize;
use serde::de::DeserializeOwned;
use tokio::sync::broadcast;

use super::{ApiResponse, CrabClient, CurrentUserResponse, LoginResponse};
use crate::common::AppError;
use crate::message::BusMessage;
use crate::server::ServerState;

/// Oneshot client backend for in-process calls
///
/// This client makes direct calls through the router using Tower's oneshot service.
/// It bypasses the network stack entirely, making it ideal for same-process communication.
#[derive(Debug, Clone)]
pub struct Oneshot {
    state: ServerState,
    token: Option<String>,
}

impl Oneshot {
    /// Create a new oneshot client with the given server state
    pub fn new(state: ServerState) -> Self {
        Self { state, token: None }
    }

    /// Set authentication token
    pub fn with_token(mut self, token: impl Into<String>) -> Self {
        self.token = Some(token.into());
        self
    }

    fn auth_header(&self) -> Option<String> {
        self.token.as_ref().map(|t| format!("Bearer {}", t))
    }

    /// Subscribe to message bus notifications
    pub fn subscribe_messages(&self) -> broadcast::Receiver<BusMessage> {
        self.state.message_bus().subscribe()
    }

    /// Get a message client for bidirectional communication
    pub fn message_client(&self) -> crate::client::MessageClient {
        let bus = self.state.message_bus();
        crate::client::MessageClient::memory(bus)
    }

    /// Execute a GET request and parse the response
    pub async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T, AppError> {
        let mut request = Request::builder().uri(path).method(http::Method::GET);

        if let Some(auth) = self.auth_header() {
            request = request.header(http::header::AUTHORIZATION, auth);
        }

        let request = request
            .body(Vec::new().into())
            .map_err(|e| AppError::Internal(e.to_string()))?;

        self.execute(request).await
    }

    /// Execute a POST request with body and parse the response
    pub async fn post<T: DeserializeOwned, B: Serialize + Sync>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T, AppError> {
        let body_bytes = serde_json::to_vec(body).map_err(|e| AppError::Internal(e.to_string()))?;

        let mut request = Request::builder()
            .uri(path)
            .method(http::Method::POST)
            .header(http::header::CONTENT_TYPE, "application/json");

        if let Some(auth) = self.auth_header() {
            request = request.header(http::header::AUTHORIZATION, auth);
        }

        let request = request
            .body(body_bytes.into())
            .map_err(|e| AppError::Internal(e.to_string()))?;

        self.execute(request).await
    }

    /// Execute a POST request without body and parse the response
    pub async fn post_empty<T: DeserializeOwned>(&self, path: &str) -> Result<T, AppError> {
        let mut request = Request::builder().uri(path).method(http::Method::POST);

        if let Some(auth) = self.auth_header() {
            request = request.header(http::header::AUTHORIZATION, auth);
        }

        let request = request
            .body(Vec::new().into())
            .map_err(|e| AppError::Internal(e.to_string()))?;

        self.execute(request).await
    }

    /// Execute a request and parse the response
    async fn execute<T: DeserializeOwned>(
        &self,
        request: Request<axum::body::Body>,
    ) -> Result<T, AppError> {
        let response = self
            .state
            .oneshot(request)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;
        let status = response.status();
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

        if !status.is_success() {
            let text = String::from_utf8_lossy(&body).to_string();
            return match status {
                http::StatusCode::UNAUTHORIZED => Err(AppError::Unauthorized),
                http::StatusCode::FORBIDDEN => Err(AppError::Forbidden(text)),
                http::StatusCode::NOT_FOUND => Err(AppError::NotFound(text)),
                http::StatusCode::BAD_REQUEST => Err(AppError::Validation(text)),
                _ => Err(AppError::Internal(text)),
            };
        }

        serde_json::from_slice(&body).map_err(|e| AppError::Internal(e.to_string()))
    }
}

#[async_trait]
impl CrabClient for Oneshot {
    async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T, AppError> {
        Oneshot::get(self, path).await
    }

    async fn post<T: DeserializeOwned, B: Serialize + Sync>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T, AppError> {
        Oneshot::post(self, path, body).await
    }

    async fn post_empty<T: DeserializeOwned>(&self, path: &str) -> Result<T, AppError> {
        Oneshot::post_empty(self, path).await
    }

    async fn login(&mut self, username: &str, password: &str) -> Result<LoginResponse, AppError> {
        #[derive(Serialize)]
        struct LoginRequest {
            username: String,
            password: String,
        }

        let request = LoginRequest {
            username: username.to_string(),
            password: password.to_string(),
        };

        let response =
            Oneshot::post::<ApiResponse<LoginResponse>, _>(self, "/api/auth/login", &request)
                .await?
                .data
                .ok_or_else(|| AppError::Internal("Missing login data".to_string()))?;

        // Store the token for subsequent requests
        self.token = Some(response.token.clone());

        Ok(response)
    }

    async fn me(&self) -> Result<CurrentUserResponse, AppError> {
        Oneshot::get::<ApiResponse<CurrentUserResponse>>(self, "/api/auth/me")
            .await?
            .data
            .ok_or_else(|| AppError::Internal("Missing user data".to_string()))
    }

    async fn logout(&mut self) -> Result<(), AppError> {
        Oneshot::post_empty::<ApiResponse<()>>(self, "/api/auth/logout").await?;
        // Clear the token after logout
        self.token = None;
        Ok(())
    }

    fn subscribe(&self) -> Result<broadcast::Receiver<BusMessage>, AppError> {
        Ok(self.subscribe_messages())
    }
}
