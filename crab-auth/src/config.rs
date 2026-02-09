/// crab-auth configuration from environment variables
pub struct Config {
    /// PostgreSQL connection string
    pub database_url: String,
    /// S3 bucket for .p12 certificates
    pub s3_bucket: String,
    /// KMS Key ID for S3 SSE-KMS encryption
    pub kms_key_id: Option<String>,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            database_url: std::env::var("DATABASE_URL").expect("DATABASE_URL must be set"),
            s3_bucket: std::env::var("P12_S3_BUCKET")
                .unwrap_or_else(|_| "crab-tenant-certificates".to_string()),
            kms_key_id: std::env::var("P12_KMS_KEY_ID").ok(),
        }
    }
}
