mod api;
mod config;
mod db;
mod state;

use config::Config;
use sqlx::postgres::PgPoolOptions;
use state::{AppState, CaStore};
use std::sync::Arc;
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), lambda_http::Error> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "crab_auth=info".into()),
        )
        .without_time() // CloudWatch adds timestamps
        .init();

    dotenvy::dotenv().ok();

    let config = Config::from_env();

    // Connect to PostgreSQL
    let pool = PgPoolOptions::new()
        .max_connections(2) // Lambda: one request at a time
        .acquire_timeout(std::time::Duration::from_secs(5))
        .idle_timeout(std::time::Duration::from_secs(600))
        .max_lifetime(std::time::Duration::from_secs(1800))
        .connect(&config.database_url)
        .await?;

    info!("Connected to PostgreSQL");

    // Run migrations (idempotent, fast on warm starts)
    sqlx::migrate!("./migrations").run(&pool).await?;

    // Initialize AWS SDK
    let aws_config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
    let sm_client = aws_sdk_secretsmanager::Client::new(&aws_config);
    let s3_client = aws_sdk_s3::Client::new(&aws_config);

    // Initialize CA store and verify Root CA is accessible
    let ca_store = CaStore::new(sm_client.clone());
    ca_store.get_or_create_root_ca().await?;
    info!("Root CA ready");

    let state = Arc::new(AppState {
        db: pool,
        ca_store,
        sm: sm_client,
        s3: s3_client,
        s3_bucket: config.s3_bucket,
        kms_key_id: config.kms_key_id,
    });

    let app = api::router(state);

    info!("crab-auth Lambda handler ready");
    lambda_http::run(app).await
}
