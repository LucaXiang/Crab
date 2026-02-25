//! Tenant management API endpoints — split into sub-modules by domain

mod account;
mod analytics;
mod audit;
mod auth;
mod billing;
mod command;
mod order;
mod store;

use shared::error::{AppError, ErrorCode};

use crate::db::tenant_queries;
use crate::state::AppState;

pub type ApiResult<T> = Result<axum::Json<T>, AppError>;

/// Verify that a store belongs to the given tenant.
pub async fn verify_store(
    state: &AppState,
    store_id: i64,
    tenant_id: &str,
) -> Result<(), AppError> {
    tenant_queries::verify_store_ownership(&state.pool, store_id, tenant_id)
        .await
        .map_err(|e| {
            tracing::error!("Store verification error: {e}");
            AppError::new(ErrorCode::InternalError)
        })?
        .ok_or_else(|| {
            AppError::with_message(ErrorCode::NotFound, "Store not found or access denied")
        })?;
    Ok(())
}

// Re-export all handlers for route registration
pub use auth::{forgot_password, login, reset_password};

pub use account::{
    change_email, change_password, confirm_email_change, get_profile, update_profile,
};

pub use store::{list_stores, update_store};

pub use analytics::{
    get_report_detail, get_stats, get_store_overview, get_store_red_flags, get_tenant_overview,
};

pub use order::{get_order_detail, list_orders};

pub use command::{create_command, list_commands};

pub use billing::{billing_portal, create_checkout};

pub use audit::audit_log;
