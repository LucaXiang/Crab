//! Print Config API Handlers
//!
//! Manages system default printer configuration for kitchen and label printing.

use axum::extract::State;
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::core::ServerState;
use crate::utils::AppResult;

/// System print configuration response/request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrintConfig {
    /// Default kitchen printer destination ID (None = no default)
    pub default_kitchen_printer: Option<String>,
    /// Default label printer destination ID (None = no default)
    pub default_label_printer: Option<String>,
}

/// GET /api/print-config
///
/// Returns the current system default printer configuration.
pub async fn get(State(state): State<ServerState>) -> AppResult<Json<PrintConfig>> {
    let cache = state.kitchen_print_service.config_cache();
    let defaults = cache.get_defaults().await;
    Ok(Json(PrintConfig {
        default_kitchen_printer: defaults.0,
        default_label_printer: defaults.1,
    }))
}

/// PUT /api/print-config
///
/// Updates the system default printer configuration.
/// Pass `null` to clear a default.
pub async fn update(
    State(state): State<ServerState>,
    Json(config): Json<PrintConfig>,
) -> AppResult<Json<PrintConfig>> {
    let cache = state.kitchen_print_service.config_cache();
    cache
        .set_defaults(
            config.default_kitchen_printer.clone(),
            config.default_label_printer.clone(),
        )
        .await;

    tracing::info!(
        default_kitchen = ?config.default_kitchen_printer,
        default_label = ?config.default_label_printer,
        "System default print config updated"
    );

    Ok(Json(config))
}
