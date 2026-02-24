//! Print Config API Handlers
//!
//! Manages system default printer configuration for kitchen and label printing.

use axum::Json;
use axum::extract::{Extension, State};
use serde::{Deserialize, Serialize};

use crate::audit::AuditAction;
use crate::audit_log;
use crate::auth::CurrentUser;
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
    let defaults = state.catalog_service.get_print_defaults();
    Ok(Json(PrintConfig {
        default_kitchen_printer: defaults.kitchen_destination,
        default_label_printer: defaults.label_destination,
    }))
}

/// PUT /api/print-config
///
/// Updates the system default printer configuration.
/// Pass `null` to clear a default.
pub async fn update(
    State(state): State<ServerState>,
    Extension(current_user): Extension<CurrentUser>,
    Json(config): Json<PrintConfig>,
) -> AppResult<Json<PrintConfig>> {
    // Persist to DB first
    crate::db::repository::print_config::update(
        &state.pool,
        config.default_kitchen_printer.as_deref(),
        config.default_label_printer.as_deref(),
    )
    .await
    .map_err(crate::utils::AppError::from)?;

    // Then update in-memory cache
    state.catalog_service.set_print_defaults(
        config.default_kitchen_printer.clone(),
        config.default_label_printer.clone(),
    );

    audit_log!(
        state.audit_service,
        AuditAction::PrintConfigChanged,
        "print_config",
        "default",
        operator_id = Some(current_user.id),
        operator_name = Some(current_user.display_name.clone()),
        details = serde_json::json!({
            "default_kitchen_printer": &config.default_kitchen_printer,
            "default_label_printer": &config.default_label_printer,
        })
    );

    state
        .broadcast_sync(
            shared::cloud::SyncResource::PrintConfig,
            "updated",
            "default",
            Some(&config),
        )
        .await;

    Ok(Json(config))
}
