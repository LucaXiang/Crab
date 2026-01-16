use crate::auth::JwtConfig;

/// 服务器配置 - 边缘节点的所有配置项
///
/// # 环境变量
///
/// 所有配置项都可以通过环境变量覆盖：
///
/// | 环境变量 | 默认值 | 说明 |
/// |----------|--------|------|
/// | WORK_DIR | /var/lib/crab/edge | 工作目录 |
/// | HTTP_PORT | 3000 | HTTP 服务端口 |
/// | MESSAGE_TCP_PORT | 8081 | TCP 消息总线端口 |
/// | AUTH_SERVER_URL | http://localhost:3001 | 认证服务器地址 |
/// | ENVIRONMENT | development | 运行环境 |
/// | MAX_CONNECTIONS | 1000 | 最大连接数 |
/// | REQUEST_TIMEOUT_MS | 30000 | 请求超时(毫秒) |
///
/// # 示例
///
/// ```ignore
/// WORK_DIR=/data/crab HTTP_PORT=8080 cargo run
/// ```
#[derive(Debug, Clone)]
pub struct Config {
    /// 工作目录，存储证书、日志等文件
    pub work_dir: String,
    /// HTTP API 服务端口
    pub http_port: u16,
    /// TCP 消息总线端口 (用于客户端直连)
    pub message_tcp_port: u16,
    /// JWT 认证配置
    pub jwt: JwtConfig,
    /// 运行环境: development | staging | production
    pub environment: String,

    // === 多租户特性配置 ===
    /// 认证服务器 URL (用于边缘激活)
    pub auth_server_url: String,
    /// 最大并发连接数
    pub max_connections: u32,
    /// 请求超时时间 (毫秒)
    pub request_timeout_ms: u64,
    /// 关闭超时时间 (毫秒)
    pub shutdown_timeout_ms: u64,
    /// 是否启用多租户支持
    pub enable_multi_tenant: bool,
    /// 是否启用资源配额
    pub enable_resource_quota: bool,
    /// 是否启用审计日志
    pub enable_audit_log: bool,
    /// 是否启用指标监控
    pub enable_metrics: bool,
}

impl Config {
    /// 从环境变量加载配置
    ///
    /// 如果环境变量未设置，使用默认值
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

    /// 使用自定义值覆盖部分配置
    ///
    /// 常用于测试场景
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

    /// 是否生产环境
    pub fn is_production(&self) -> bool {
        self.environment == "production"
    }

    /// 是否开发环境
    pub fn is_development(&self) -> bool {
        self.environment == "development"
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::from_env()
    }
}
