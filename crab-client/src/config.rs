//! Client configuration

/// Client type for communication with edge server
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ClientType {
    /// HTTP client for REST API calls
    #[default]
    Http,
    /// Message client for event-based communication
    Message,
}

/// Client configuration for connecting to Edge Server
#[derive(Debug, Clone)]
pub struct ClientConfig {
    /// Server base URL (e.g., "http://localhost:8080")
    pub base_url: String,

    /// Client type - how to communicate with edge server
    pub client_type: ClientType,

    /// JWT token for authentication
    pub token: Option<String>,

    /// Request timeout in seconds
    pub timeout: u64,

    /// TLS Root CA certificate (PEM format) - for validating tenant CA
    pub tls_root_ca_cert: Option<String>,

    /// TLS Tenant CA certificate (PEM format) - for client verification
    pub tls_ca_cert: Option<String>,

    /// TLS Client certificate (PEM format)
    pub tls_client_cert: Option<String>,

    /// TLS Client key (PEM format)
    pub tls_client_key: Option<String>,

    /// Message TCP address (for Message client type)
    pub message_tcp_addr: Option<String>,
}

impl ClientConfig {
    /// Create a new client configuration with HTTP client
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            client_type: ClientType::Http,
            token: None,
            timeout: 30,
            tls_root_ca_cert: None,
            tls_ca_cert: None,
            tls_client_cert: None,
            tls_client_key: None,
            message_tcp_addr: None,
        }
    }

    /// Use HTTP client type
    pub fn http(base_url: impl Into<String>) -> Self {
        Self::new(base_url).with_client_type(ClientType::Http)
    }

    /// Use Message client type
    pub fn message(base_url: impl Into<String>, tcp_addr: impl Into<String>) -> Self {
        Self::new(base_url)
            .with_client_type(ClientType::Message)
            .with_message_tcp_addr(tcp_addr)
    }

    /// Set the client type
    pub fn with_client_type(mut self, client_type: ClientType) -> Self {
        self.client_type = client_type;
        self
    }

    /// Set the message TCP address
    pub fn with_message_tcp_addr(mut self, addr: impl Into<String>) -> Self {
        self.message_tcp_addr = Some(addr.into());
        self
    }

    /// Set the JWT token
    pub fn with_token(mut self, token: impl Into<String>) -> Self {
        self.token = Some(token.into());
        self
    }

    /// Set mTLS configuration with root CA validation
    pub fn with_tls(
        mut self,
        root_ca_cert: impl Into<String>,
        tenant_ca_cert: impl Into<String>,
        client_cert: impl Into<String>,
        client_key: impl Into<String>,
    ) -> Self {
        self.tls_root_ca_cert = Some(root_ca_cert.into());
        self.tls_ca_cert = Some(tenant_ca_cert.into());
        self.tls_client_cert = Some(client_cert.into());
        self.tls_client_key = Some(client_key.into());
        self
    }

    /// Set mTLS configuration with just tenant CA (for backward compatibility)
    pub fn with_tls_simple(
        mut self,
        ca_cert: impl Into<String>,
        client_cert: impl Into<String>,
        client_key: impl Into<String>,
    ) -> Self {
        self.tls_ca_cert = Some(ca_cert.into());
        self.tls_client_cert = Some(client_cert.into());
        self.tls_client_key = Some(client_key.into());
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

    /// Create a message client from this configuration
    pub async fn build_message_client(&self) -> Result<super::MessageClient, super::MessageError> {
        super::MessageClient::from_config(self).await
    }
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self::new("http://localhost:8080")
    }
}
