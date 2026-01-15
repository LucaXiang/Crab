use crate::server::auth::JwtConfig;

/// Enhanced configuration for SaaS edge server
#[derive(Debug, Clone)]
pub struct Config {
    // Legacy fields for backward compatibility
    pub work_dir: String,
    pub http_port: u16,
    pub message_tcp_port: u16,
    pub jwt: JwtConfig,
    pub environment: String,

    // New SaaS-specific configuration
    pub auth_server_url: String,
    pub max_connections: u32,
    pub request_timeout_ms: u64,
    pub shutdown_timeout_ms: u64,
    pub enable_multi_tenant: bool,
    pub enable_resource_quota: bool,
    pub enable_audit_log: bool,
    pub enable_metrics: bool,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            work_dir: std::env::var("WORK_DIR").unwrap_or_else(|_| "/var/lib/crab/edge".into()),
            http_port: std::env::var("HTTP_PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(3000),
            message_tcp_port: std::env::var("MESSAGE_TCP_PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(8081),
            jwt: JwtConfig::default(),
            environment: std::env::var("ENVIRONMENT").unwrap_or_else(|_| "development".into()),

            // New SaaS features
            auth_server_url: std::env::var("AUTH_SERVER_URL")
                .unwrap_or_else(|_| "http://localhost:3001".into()),
            max_connections: std::env::var("MAX_CONNECTIONS")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(1000),
            request_timeout_ms: std::env::var("REQUEST_TIMEOUT_MS")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(30000),
            shutdown_timeout_ms: std::env::var("SHUTDOWN_TIMEOUT_MS")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(10000),
            enable_multi_tenant: std::env::var("ENABLE_MULTI_TENANT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(true),
            enable_resource_quota: std::env::var("ENABLE_RESOURCE_QUOTA")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(true),
            enable_audit_log: std::env::var("ENABLE_AUDIT_LOG")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(true),
            enable_metrics: std::env::var("ENABLE_METRICS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(true),
        }
    }

    /// Create a config with custom overrides
    pub fn with_overrides(
        work_dir: impl Into<String>,
        http_port: u16,
        message_tcp_port: u16,
    ) -> Self {
        let mut config = Self::from_env();
        config.work_dir = work_dir.into();
        config.http_port = http_port;
        config.message_tcp_port = message_tcp_port;
        config
    }

    pub fn is_production(&self) -> bool {
        self.environment == "production"
    }

    pub fn is_development(&self) -> bool {
        self.environment == "development"
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::from_env()
    }
}
