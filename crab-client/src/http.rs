//! HTTP client for network-based API calls

use crate::{
    ApiResponse, ClientConfig, ClientError, ClientResult, CurrentUserResponse, LoginResponse,
};
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
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            "x-client-version",
            reqwest::header::HeaderValue::from_static(env!("CARGO_PKG_VERSION")),
        );

        let mut builder = Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout))
            .user_agent(concat!("crab-client/", env!("CARGO_PKG_VERSION")))
            .default_headers(headers);

        // Configure mTLS if certificates are provided
        if let Some(ca_cert_pem) = &config.tls_ca_cert {
            // 1. Load Root CA
            let mut root_store = rustls::RootCertStore::empty();
            let mut reader = std::io::BufReader::new(ca_cert_pem.as_bytes());
            for c in rustls_pemfile::certs(&mut reader).flatten() {
                root_store.add(c).ok();
            }

            // 2. Create SkipHostnameVerifier (ignores hostname mismatch, but enforces CA signature)
            let verifier = std::sync::Arc::new(crab_cert::SkipHostnameVerifier::new(root_store));

            // 3. Prepare Client Config Builder
            let config_builder = rustls::ClientConfig::builder()
                .dangerous()
                .with_custom_certificate_verifier(verifier);

            // 4. Load Client Cert/Key if present
            let tls_config = if let (Some(cert_pem), Some(key_pem)) =
                (&config.tls_client_cert, &config.tls_client_key)
            {
                let certs =
                    rustls_pemfile::certs(&mut std::io::BufReader::new(cert_pem.as_bytes()))
                        .filter_map(|r| r.ok())
                        .collect::<Vec<_>>();

                let mut key_reader = std::io::BufReader::new(key_pem.as_bytes());
                let key = rustls_pemfile::private_key(&mut key_reader)
                    .ok()
                    .flatten()
                    .expect("Failed to parse client key");

                config_builder
                    .with_client_auth_cert(certs, key)
                    .expect("Failed to set client auth")
            } else {
                config_builder.with_no_client_auth()
            };

            builder = builder.use_preconfigured_tls(tls_config);
        }

        let client = builder.build().expect("Failed to build HTTP client");

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
    pub async fn post<T: DeserializeOwned, B: serde::Serialize>(
        &self,
        path: &str,
        body: &B,
    ) -> ClientResult<T> {
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
        self.post_empty::<ApiResponse<()>>("/api/auth/logout")
            .await?;
        self.token = None;
        Ok(())
    }
}
