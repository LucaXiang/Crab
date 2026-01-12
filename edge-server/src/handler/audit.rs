//! Manual Audit Log Handler
//!
//! Provides endpoints for clients to create audit log entries manually.
//! Used for events like "open cash drawer", "print receipt", etc.

use crate::audit_log;
use crate::common::{AppError, AppResponse, ok};
use crate::server::CurrentUser;
use axum::{Extension, Json};
use serde::{Deserialize, Serialize};

/// Request to create a manual audit log
#[derive(Debug, Deserialize)]
pub struct CreateAuditLogRequest {
    /// Event category (e.g., "cash_drawer", "receipt", "manual")
    pub category: String,
    /// Event action (e.g., "open", "close", "print")
    pub action: String,
    /// Optional target entity ID (e.g., order_id, employee_id)
    pub target_id: Option<String>,
    /// Additional description or notes
    pub description: Option<String>,
}

/// Response for audit log creation
#[derive(Debug, Serialize)]
pub struct AuditLogResponse {
    pub success: bool,
    pub message: String,
}

/// Create a manual audit log entry
pub async fn create_audit_log(
    Extension(current_user): Extension<CurrentUser>,
    Json(request): Json<CreateAuditLogRequest>,
) -> Result<Json<AppResponse<AuditLogResponse>>, AppError> {
    // Validate category and action are not empty
    if request.category.trim().is_empty() {
        return Err(AppError::Validation("Category is required".to_string()));
    }
    if request.action.trim().is_empty() {
        return Err(AppError::Validation("Action is required".to_string()));
    }

    let description = request
        .description
        .unwrap_or_else(|| format!("{} {}", request.category, request.action));

    // Create the audit log
    audit_log!(
        request.category,
        request.action,
        request.target_id.as_deref().unwrap_or(""),
        description
    );

    tracing::info!(
        user_id = %current_user.id,
        category = %request.category,
        action = %request.action,
        target_id = ?request.target_id,
        "Manual audit log created"
    );

    let response = AuditLogResponse {
        success: true,
        message: "Audit log created successfully".to_string(),
    };

    Ok(ok(response))
}
