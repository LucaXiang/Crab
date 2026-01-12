use crate::server::auth::JwtConfig;

#[derive(Debug, Clone)]
pub struct Config {
    pub work_dir: String,
    pub http_port: u16,
    pub jwt: JwtConfig,
    pub environment: String,
    pub message_tcp_port: u16,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            work_dir: std::env::var("WORK_DIR").expect("Please configure WORK_DIR!"),
            http_port: std::env::var("HTTP_PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(3000),
            jwt: JwtConfig::default(),
            environment: std::env::var("ENVIRONMENT").unwrap_or("development".into()),
            message_tcp_port: std::env::var("MESSAGE_TCP_PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(8081),
        }
    }

    pub fn is_production(&self) -> bool {
        self.environment == "production"
    }
}
