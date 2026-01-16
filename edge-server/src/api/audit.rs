//! Audit Log Routes
//!
//! Provides endpoints for manual audit log entries.

use axum::{Json, Router, extract::Extension, routing::post};
use serde::{Deserialize, Serialize};

use crate::audit_log;
use crate::auth::CurrentUser;
use crate::{AppError, AppResponse};

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

/// Build audit router
pub fn router() -> Router<crate::ServerState> {
    Router::new()
        // Create manual audit log - authentication required
        .route("/api/audit", post(create_audit_log))
}

/// Create a manual audit log entry
pub async fn create_audit_log(
    Extension(current_user): Extension<CurrentUser>,
    Json(request): Json<CreateAuditLogRequest>,
) -> Result<Json<AppResponse<AuditLogResponse>>, AppError> {
    // Validate category and action are not empty
    if request.category.trim().is_empty() {
        return Err(AppError::validation("Category is required".to_string()));
    }
    if request.action.trim().is_empty() {
        return Err(AppError::validation("Action is required".to_string()));
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

    Ok(crate::ok!(response))
}
