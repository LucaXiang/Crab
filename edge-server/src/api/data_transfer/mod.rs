//! Data Transfer API — catalog export/import via ZIP
//!
//! - GET /api/data-transfer/export → ZIP (categories, products, attributes, tags, images)
//! - POST /api/data-transfer/import → accepts ZIP, imports catalog data
//!
//! Public functions [`export_zip`] and [`import_zip`] are also available for
//! direct in-process calls (e.g. from Tauri Server mode).

mod handler;

use axum::{
    Router, middleware,
    routing::{get, post},
};

use crate::auth::require_permission;
use crate::core::ServerState;
use crate::utils::AppError;

pub fn router() -> Router<ServerState> {
    Router::new()
        .route("/api/data-transfer/export", get(handler::export))
        .route("/api/data-transfer/import", post(handler::import))
        .layer(middleware::from_fn(require_permission("menu:manage")))
}

/// Export catalog data as ZIP bytes (for direct in-process call)
pub async fn export_zip(state: &ServerState) -> Result<Vec<u8>, AppError> {
    handler::export_zip(state).await
}

/// Import catalog data from ZIP bytes (for direct in-process call)
pub async fn import_zip(state: &ServerState, zip_bytes: &[u8]) -> Result<(), AppError> {
    handler::import_zip(state, zip_bytes).await
}
