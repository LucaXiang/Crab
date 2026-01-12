//! Client configuration

/// Client configuration for connecting to Edge Server
#[derive(Debug, Clone)]
pub struct ClientConfig {
    /// Server base URL (e.g., "http://localhost:8080")
    pub base_url: String,

    /// JWT token for authentication
    pub token: Option<String>,

    /// Request timeout in seconds
    pub timeout: u64,
}

impl ClientConfig {
    /// Create a new client configuration
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            token: None,
            timeout: 30,
        }
    }

    /// Set the JWT token
    pub fn with_token(mut self, token: impl Into<String>) -> Self {
        self.token = Some(token.into());
        self
    }

    /// Set the request timeout
    pub fn with_timeout(mut self, seconds: u64) -> Self {
        self.timeout = seconds;
        self
    }

    /// Create an HTTP client from this configuration
    pub fn build_http_client(&self) -> super::HttpClient {
        super::HttpClient::new(self)
    }
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self::new("http://localhost:8080")
    }
}
