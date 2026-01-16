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
            // Optional: Validate certificate chain if root CA is provided
            if let Some(root_ca_pem) = &config.tls_root_ca_cert {
                tracing::info!("Validating certificate chain: Root CA -> Tenant CA");
                match crab_cert::verify_chain_against_root(ca_cert_pem, root_ca_pem) {
                    Ok(_) => tracing::info!("✅ Certificate chain validation passed"),
                    Err(e) => {
                        tracing::warn!("⚠️ Certificate chain validation failed: {}", e);
                        // Continue anyway for backward compatibility
                    }
                }
            }

            let mut ca_reader = std::io::Cursor::new(ca_cert_pem);
            let ca_certs: Vec<rustls::pki_types::CertificateDer> =
                rustls_pemfile::certs(&mut ca_reader)
                    .collect::<Result<Vec<_>, _>>()
                    .unwrap_or_else(|e| {
                        tracing::warn!("Failed to parse CA certificates: {}", e);
                        Vec::new()
                    });

            let mut root_store = rustls::RootCertStore::empty();
            for cert in ca_certs {
                root_store.add(cert).unwrap_or_else(|e| {
                    tracing::warn!("Failed to add CA certificate: {}", e);
                });
            }

            let verifier = std::sync::Arc::new(crab_cert::SkipHostnameVerifier::new(root_store));

            let config_builder = rustls::ClientConfig::builder()
                .dangerous()
                .with_custom_certificate_verifier(verifier);

            let tls_config = if let (Some(cert_pem), Some(key_pem)) =
                (&config.tls_client_cert, &config.tls_client_key)
            {
                let mut cert_reader = std::io::Cursor::new(cert_pem);
                let certs: Vec<rustls::pki_types::CertificateDer> =
                    rustls_pemfile::certs(&mut cert_reader)
                        .collect::<Result<Vec<_>, _>>()
                        .unwrap_or_else(|e| {
                            tracing::warn!("Failed to parse client certificates: {}", e);
                            Vec::new()
                        });

                let mut key_reader = std::io::Cursor::new(key_pem);
                let key = rustls_pemfile::private_key(&mut key_reader)
                    .unwrap_or_else(|e| {
                        tracing::warn!("Failed to parse client key: {}", e);
                        panic!("Failed to parse client key: {}", e);
                    })
                    .expect("Client key must be present");

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
