mod api;
mod state;

use std::path::PathBuf;
use std::sync::Arc;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let work_dir = PathBuf::from("auth_storage");
    if !work_dir.exists() {
        std::fs::create_dir_all(&work_dir).unwrap();
    }

    let state = Arc::new(state::AppState {
        auth_storage: state::AuthStorage::new(work_dir.clone()),
        user_store: state::UserStore::new(),
        jwt_secret: "crab-auth-secret-key-2024".to_string(),
    });

    // Ensure Root CA exists on startup
    if let Err(e) = state.auth_storage.get_or_create_root_ca() {
        tracing::error!("Failed to initialize Root CA: {}", e);
        return;
    }

    let app = api::router(state);
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3001").await.unwrap();
    tracing::info!(
        "Crab Auth Server listening on {}",
        listener.local_addr().unwrap()
    );
    axum::serve(listener, app).await.unwrap();
}
