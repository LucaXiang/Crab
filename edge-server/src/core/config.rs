use std::path::{Path, PathBuf};

use crate::auth::JwtConfig;

/// 服务器配置 - 边缘节点的所有配置项
///
/// # 创建方式
///
/// - `Config::builder()` - Builder 模式 (推荐)
/// - `Config::from_env()` - 从环境变量加载 (仅用于 bin，需先调用 dotenv)
///
/// # 示例
///
/// ```ignore
/// use edge_server::Config;
///
/// // Builder 模式 (库使用)
/// let config = Config::builder()
///     .work_dir("/data/crab")
///     .http_port(8080)
///     .message_tcp_port(9000)
///     .build();
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
    /// 认证服务器 URL (用于边缘激活)
    pub auth_server_url: String,
    /// 最大并发连接数
    pub max_connections: u32,
    /// 请求超时时间 (毫秒)
    pub request_timeout_ms: u64,
    /// 关闭超时时间 (毫秒)
    pub shutdown_timeout_ms: u64,
}

/// Config Builder
#[derive(Debug, Clone, Default)]
pub struct ConfigBuilder {
    work_dir: Option<String>,
    http_port: Option<u16>,
    message_tcp_port: Option<u16>,
    jwt: Option<JwtConfig>,
    environment: Option<String>,
    auth_server_url: Option<String>,
    max_connections: Option<u32>,
    request_timeout_ms: Option<u64>,
    shutdown_timeout_ms: Option<u64>,
}

impl ConfigBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn work_dir(mut self, value: impl Into<String>) -> Self {
        self.work_dir = Some(value.into());
        self
    }

    pub fn http_port(mut self, value: u16) -> Self {
        self.http_port = Some(value);
        self
    }

    pub fn message_tcp_port(mut self, value: u16) -> Self {
        self.message_tcp_port = Some(value);
        self
    }

    pub fn jwt(mut self, value: JwtConfig) -> Self {
        self.jwt = Some(value);
        self
    }

    pub fn environment(mut self, value: impl Into<String>) -> Self {
        self.environment = Some(value.into());
        self
    }

    pub fn auth_server_url(mut self, value: impl Into<String>) -> Self {
        self.auth_server_url = Some(value.into());
        self
    }

    pub fn max_connections(mut self, value: u32) -> Self {
        self.max_connections = Some(value);
        self
    }

    pub fn request_timeout_ms(mut self, value: u64) -> Self {
        self.request_timeout_ms = Some(value);
        self
    }

    pub fn shutdown_timeout_ms(mut self, value: u64) -> Self {
        self.shutdown_timeout_ms = Some(value);
        self
    }

    /// 构建配置，使用默认值填充未设置的字段
    pub fn build(self) -> Config {
        Config {
            work_dir: self.work_dir.unwrap_or_else(|| "/var/lib/crab/edge".into()),
            http_port: self.http_port.unwrap_or(3000),
            message_tcp_port: self.message_tcp_port.unwrap_or(8081),
            jwt: self.jwt.unwrap_or_default(),
            environment: self.environment.unwrap_or_else(|| "development".into()),
            auth_server_url: self.auth_server_url.unwrap_or_else(|| "http://localhost:3001".into()),
            max_connections: self.max_connections.unwrap_or(1000),
            request_timeout_ms: self.request_timeout_ms.unwrap_or(30000),
            shutdown_timeout_ms: self.shutdown_timeout_ms.unwrap_or(10000),
        }
    }
}

impl Config {
    /// 创建 Builder
    pub fn builder() -> ConfigBuilder {
        ConfigBuilder::new()
    }

    /// 从环境变量加载配置
    ///
    /// **注意**: 此方法仅应在 bin crate 中使用，在调用前应先加载 .env 文件。
    /// 库代码应使用 `Config::builder()` 显式构建配置。
    ///
    /// # 环境变量
    ///
    /// | 变量 | 默认值 | 说明 |
    /// |------|--------|------|
    /// | WORK_DIR | /var/lib/crab/edge | 工作目录 |
    /// | HTTP_PORT | 3000 | HTTP 端口 |
    /// | MESSAGE_TCP_PORT | 8081 | TCP 消息端口 |
    /// | ENVIRONMENT | development | 运行环境 |
    /// | AUTH_SERVER_URL | http://localhost:3001 | 认证服务器 |
    pub fn from_env() -> Self {
        Self::builder()
            .work_dir(std::env::var("WORK_DIR").unwrap_or_else(|_| "/var/lib/crab/edge".into()))
            .http_port(
                std::env::var("HTTP_PORT")
                    .ok()
                    .and_then(|p| p.parse().ok())
                    .unwrap_or(3000),
            )
            .message_tcp_port(
                std::env::var("MESSAGE_TCP_PORT")
                    .ok()
                    .and_then(|p| p.parse().ok())
                    .unwrap_or(8081),
            )
            .environment(std::env::var("ENVIRONMENT").unwrap_or_else(|_| "development".into()))
            .auth_server_url(
                std::env::var("AUTH_SERVER_URL")
                    .unwrap_or_else(|_| "http://localhost:3001".into()),
            )
            .max_connections(
                std::env::var("MAX_CONNECTIONS")
                    .ok()
                    .and_then(|p| p.parse().ok())
                    .unwrap_or(1000),
            )
            .request_timeout_ms(
                std::env::var("REQUEST_TIMEOUT_MS")
                    .ok()
                    .and_then(|p| p.parse().ok())
                    .unwrap_or(30000),
            )
            .shutdown_timeout_ms(
                std::env::var("SHUTDOWN_TIMEOUT_MS")
                    .ok()
                    .and_then(|p| p.parse().ok())
                    .unwrap_or(10000),
            )
            .build()
    }

    /// 使用自定义值覆盖部分配置 (测试用)
    pub fn with_overrides(
        work_dir: impl Into<String>,
        http_port: u16,
        message_tcp_port: u16,
    ) -> Self {
        Self::builder()
            .work_dir(work_dir)
            .http_port(http_port)
            .message_tcp_port(message_tcp_port)
            .build()
    }

    /// 是否生产环境
    pub fn is_production(&self) -> bool {
        self.environment == "production"
    }

    /// 是否开发环境
    pub fn is_development(&self) -> bool {
        self.environment == "development"
    }

    /// 确保工作目录结构存在
    ///
    /// 创建标准化的目录结构:
    /// - `certs/` - 证书目录
    /// - `database/` - 数据库目录
    /// - `logs/` - 日志目录
    /// - `auth_storage/` - 认证存储目录
    pub fn ensure_work_dir_structure(&self) -> std::io::Result<()> {
        let base = PathBuf::from(&self.work_dir);
        std::fs::create_dir_all(base.join("certs"))?;
        std::fs::create_dir_all(base.join("database"))?;
        std::fs::create_dir_all(base.join("logs"))?;
        std::fs::create_dir_all(base.join("auth_storage"))?;
        Ok(())
    }

    /// 获取证书目录路径
    pub fn certs_dir(&self) -> PathBuf {
        PathBuf::from(&self.work_dir).join("certs")
    }

    /// 获取数据库目录路径
    pub fn database_dir(&self) -> PathBuf {
        PathBuf::from(&self.work_dir).join("database")
    }

    /// 获取日志目录路径
    pub fn logs_dir(&self) -> PathBuf {
        PathBuf::from(&self.work_dir).join("logs")
    }

    /// 获取认证存储目录路径
    pub fn auth_storage_dir(&self) -> PathBuf {
        PathBuf::from(&self.work_dir).join("auth_storage")
    }
}

/// 检查并迁移旧目录结构
///
/// 如果 `work_dir/crab.db` 存在且 `work_dir/database/crab.db` 不存在，
/// 则将数据库迁移到新位置。
pub fn migrate_legacy_structure(work_dir: &Path) -> std::io::Result<()> {
    let legacy_db = work_dir.join("crab.db");
    let new_db_dir = work_dir.join("database");

    if legacy_db.exists() && !new_db_dir.join("crab.db").exists() {
        tracing::info!("Migrating legacy database location...");
        std::fs::create_dir_all(&new_db_dir)?;
        std::fs::rename(&legacy_db, new_db_dir.join("crab.db"))?;
        tracing::info!("Database migration complete");
    }

    Ok(())
}

impl Default for Config {
    fn default() -> Self {
        Self::builder().build()
    }
}
