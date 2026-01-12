//! HTTP client for network-based API calls

use crate::{ApiResponse, ClientConfig, ClientError, ClientResult, LoginResponse, CurrentUserResponse};
use reqwest::{Client, StatusCode};
use serde::de::DeserializeOwned;

/// HTTP client for making network requests to Edge Server
#[derive(Debug, Clone)]
pub struct HttpClient {
    client: Client,
    base_url: String,
    token: Option<String>,
}

impl HttpClient {
    /// Create a new HTTP client from configuration
    pub fn new(config: &ClientConfig) -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout))
            .build()
            .expect("Failed to build HTTP client");

        Self {
            client,
            base_url: config.base_url.clone(),
            token: config.token.clone(),
        }
    }

    /// Set the authentication token
    pub fn with_token(mut self, token: impl Into<String>) -> Self {
        self.token = Some(token.into());
        self
    }

    /// Get the current token
    pub fn token(&self) -> Option<&str> {
        self.token.as_deref()
    }

    /// Build authorization header value
    fn auth_header(&self) -> Option<String> {
        self.token.as_ref().map(|t| format!("Bearer {}", t))
    }

    /// Make a GET request
    pub async fn get<T: DeserializeOwned>(&self, path: &str) -> ClientResult<T> {
        let url = format!("{}/{}", self.base_url.trim_end_matches('/'), path);
        let mut request = self.client.get(&url);

        if let Some(auth) = self.auth_header() {
            request = request.header(reqwest::header::AUTHORIZATION, auth);
        }

        let response = request.send().await?;
        Self::handle_response(response).await
    }

    /// Make a POST request with JSON body
    pub async fn post<T: DeserializeOwned, B: serde::Serialize>(&self, path: &str, body: &B) -> ClientResult<T> {
        let url = format!("{}/{}", self.base_url.trim_end_matches('/'), path);
        let mut request = self.client.post(&url).json(body);

        if let Some(auth) = self.auth_header() {
            request = request.header(reqwest::header::AUTHORIZATION, auth);
        }

        let response = request.send().await?;
        Self::handle_response(response).await
    }

    /// Make a POST request without body
    pub async fn post_empty<T: DeserializeOwned>(&self, path: &str) -> ClientResult<T> {
        let url = format!("{}/{}", self.base_url.trim_end_matches('/'), path);
        let mut request = self.client.post(&url);

        if let Some(auth) = self.auth_header() {
            request = request.header(reqwest::header::AUTHORIZATION, auth);
        }

        let response = request.send().await?;
        Self::handle_response(response).await
    }

    /// Handle the HTTP response
    async fn handle_response<T: DeserializeOwned>(response: reqwest::Response) -> ClientResult<T> {
        let status = response.status();

        if !status.is_success() {
            let text = response.text().await?;
            return match status {
                StatusCode::UNAUTHORIZED => Err(ClientError::Unauthorized),
                StatusCode::FORBIDDEN => Err(ClientError::Forbidden(text)),
                StatusCode::NOT_FOUND => Err(ClientError::NotFound(text)),
                StatusCode::BAD_REQUEST => Err(ClientError::Validation(text)),
                _ => Err(ClientError::Internal(text)),
            };
        }

        response.json().await.map_err(Into::into)
    }

    // ========== Auth API ==========

    /// Login with username and password
    pub async fn login(&self, username: &str, password: &str) -> ClientResult<LoginResponse> {
        #[derive(serde::Serialize)]
        struct LoginRequest {
            username: String,
            password: String,
        }

        let request = LoginRequest {
            username: username.to_string(),
            password: password.to_string(),
        };

        self.post::<ApiResponse<LoginResponse>, _>("/api/auth/login", &request)
            .await?
            .data
            .ok_or_else(|| ClientError::InvalidResponse("Missing login data".to_string()))
    }

    /// Get current user information
    pub async fn me(&self) -> ClientResult<CurrentUserResponse> {
        self.get::<ApiResponse<CurrentUserResponse>>("/api/auth/me")
            .await?
            .data
            .ok_or_else(|| ClientError::InvalidResponse("Missing user data".to_string()))
    }

    /// Logout
    pub async fn logout(&mut self) -> ClientResult<()> {
        self.post_empty::<ApiResponse<()>>("/api/auth/logout").await?;
        self.token = None;
        Ok(())
    }
}
