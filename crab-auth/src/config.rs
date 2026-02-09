use std::path::PathBuf;

/// crab-auth 配置，从环境变量读取
pub struct Config {
    /// PostgreSQL 连接字符串
    pub database_url: String,
    /// CA 证书存储路径
    pub auth_storage_path: PathBuf,
    /// 服务端口
    pub port: u16,
    /// S3 存储桶名 (.p12 证书)
    pub s3_bucket: String,
    /// KMS Key ID (用于 S3 SSE-KMS 加密)
    pub kms_key_id: Option<String>,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            database_url: std::env::var("DATABASE_URL").expect("DATABASE_URL must be set"),
            auth_storage_path: std::env::var("AUTH_STORAGE_PATH")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from("auth_storage")),
            port: std::env::var("PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(3001),
            s3_bucket: std::env::var("P12_S3_BUCKET")
                .unwrap_or_else(|_| "crab-tenant-certificates".to_string()),
            kms_key_id: std::env::var("P12_KMS_KEY_ID").ok(),
        }
    }
}
