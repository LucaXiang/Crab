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
use shared::message::SyncChangeType;

/// System print configuration response/request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrintConfig {
    /// Global kitchen printing toggle
    pub kitchen_enabled: bool,
    /// Default kitchen printer destination ID (None = no default)
    pub default_kitchen_printer: Option<String>,
    /// Global label printing toggle
    pub label_enabled: bool,
    /// Default label printer destination ID (None = no default)
    pub default_label_printer: Option<String>,
}

/// GET /api/print-config
///
/// Returns the current system default printer configuration.
pub async fn get(State(state): State<ServerState>) -> AppResult<Json<PrintConfig>> {
    let defaults = state.catalog_service.get_print_defaults();
    Ok(Json(PrintConfig {
        kitchen_enabled: defaults.kitchen_enabled,
        default_kitchen_printer: defaults.kitchen_destination,
        label_enabled: defaults.label_enabled,
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
        config.kitchen_enabled,
        config.default_kitchen_printer.as_deref(),
        config.label_enabled,
        config.default_label_printer.as_deref(),
    )
    .await
    .map_err(crate::utils::AppError::from)?;

    // Then update in-memory cache
    state.catalog_service.set_print_defaults(
        config.kitchen_enabled,
        config.default_kitchen_printer.clone(),
        config.label_enabled,
        config.default_label_printer.clone(),
    );

    audit_log!(
        state.audit_service,
        AuditAction::PrintConfigChanged,
        "print_config",
        "default",
        operator_id = Some(current_user.id),
        operator_name = Some(current_user.name.clone()),
        details = serde_json::json!({
            "kitchen_enabled": config.kitchen_enabled,
            "default_kitchen_printer": &config.default_kitchen_printer,
            "label_enabled": config.label_enabled,
            "default_label_printer": &config.default_label_printer,
        })
    );

    state
        .broadcast_sync(
            shared::cloud::SyncResource::PrintConfig,
            SyncChangeType::Updated,
            0,
            Some(&config),
            false,
        )
        .await;

    Ok(Json(config))
}
