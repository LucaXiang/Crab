//! Authentication Handlers
//!
//! Handles login, logout, and token management

use std::time::Duration;

use axum::{Extension, Json, extract::State};

use crate::audit::AuditAction;
use crate::auth::CurrentUser;
use crate::core::ServerState;
use crate::db::repository::{employee, role};
use crate::AppError;
use shared::models::Role;

// Re-use shared DTOs for API consistency
use shared::client::{EscalateRequest, EscalateResponse, LoginRequest, LoginResponse, UserInfo};

/// Fixed delay for authentication to prevent timing attacks
const AUTH_FIXED_DELAY_MS: u64 = 500;

/// Login handler
///
/// Authenticates user credentials and returns a JWT token
pub async fn login(
    State(state): State<ServerState>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, AppError> {
    let username = req.username.clone();

    // Query employee by username (with hash for password verification)
    let emp_with_hash = employee::find_by_username_with_hash(&state.pool, &username).await?;

    // Fixed delay to prevent timing attacks (before checking result)
    tokio::time::sleep(Duration::from_millis(AUTH_FIXED_DELAY_MS)).await;

    // Check authentication result - unified error message to prevent username enumeration
    let emp = match emp_with_hash {
        Some(e) => {
            // User found - check active status
            if !e.is_active {
                return Err(AppError::account_disabled());
            }

            // Verify password
            let password_valid = employee::verify_password(&req.password, &e.hash_pass)
                .map_err(|e| AppError::internal(format!("Password verification failed: {}", e)))?;

            if !password_valid {
                state.audit_service.log(
                    AuditAction::LoginFailed, "auth", format!("employee:{}", username),
                    None, None,
                    serde_json::json!({"reason": "invalid_credentials"}),
                ).await;
                tracing::warn!(username = %username, "Login failed - invalid credentials");
                return Err(AppError::invalid_credentials());
            }

            e
        }
        None => {
            state.audit_service.log(
                AuditAction::LoginFailed, "auth", format!("employee:{}", username),
                None, None,
                serde_json::json!({"reason": "user_not_found"}),
            ).await;
            tracing::warn!(username = %username, "Login failed - user not found");
            return Err(AppError::invalid_credentials());
        }
    };

    // Fetch role information
    let role: Role = role::find_by_id(&state.pool, emp.role_id)
        .await?
        .ok_or_else(|| AppError::new(shared::ErrorCode::RoleNotFound))?;

    if !role.is_active {
        return Err(AppError::role_disabled());
    }

    // Generate JWT token
    let jwt_service = state.get_jwt_service();
    let user_id = emp.id.to_string();

    let token = jwt_service
        .generate_token(
            &user_id,
            &emp.username,
            &emp.display_name,
            &emp.role_id.to_string(),
            &role.name,
            &role.permissions,
            emp.is_system,
        )
        .map_err(|e| AppError::internal(format!("Failed to generate token: {}", e)))?;

    // Log successful login
    state.audit_service.log(
        AuditAction::LoginSuccess, "auth", format!("employee:{}", user_id),
        Some(emp.id), Some(emp.display_name.clone()),
        serde_json::json!({"username": &emp.username}),
    ).await;

    tracing::info!(
        user_id = %user_id,
        username = %emp.username,
        role = %role.name,
        "User logged in successfully"
    );

    let response = LoginResponse {
        token,
        user: UserInfo {
            id: emp.id,
            username: emp.username,
            display_name: emp.display_name,
            role_id: emp.role_id,
            role_name: role.name,
            permissions: role.permissions,
            is_system: emp.is_system,
            is_active: emp.is_active,
            created_at: emp.created_at,
        },
    };

    Ok(Json(response))
}

/// Get current user info
pub async fn me(
    State(state): State<ServerState>,
    Extension(user): Extension<CurrentUser>,
) -> Result<Json<UserInfo>, AppError> {
    // Query fresh employee data from database for is_active and created_at
    let emp = employee::find_by_id(&state.pool, user.id)
        .await?
        .ok_or_else(|| AppError::new(shared::ErrorCode::EmployeeNotFound))?;

    let (is_active, created_at) = (emp.is_active, emp.created_at);

    let user_info = UserInfo {
        id: user.id,
        username: user.username,
        display_name: user.display_name,
        role_id: user.role_id,
        role_name: user.role_name,
        permissions: user.permissions,
        is_system: user.is_system,
        is_active,
        created_at,
    };

    Ok(Json(user_info))
}

/// Logout handler
pub async fn logout(
    State(state): State<ServerState>,
    Extension(user): Extension<CurrentUser>,
) -> Result<Json<()>, AppError> {
    state.audit_service.log(
        AuditAction::Logout, "auth", format!("employee:{}", user.id),
        Some(user.id), Some(user.display_name.clone()),
        serde_json::json!({"username": &user.username}),
    ).await;

    tracing::info!(
        user_id = %user.id,
        username = %user.username,
        "User logged out"
    );

    Ok(Json(()))
}

/// Escalate handler (supervisor authorization)
///
/// Validates supervisor credentials and checks permission.
/// Only logs on success for audit trail.
pub async fn escalate(
    State(state): State<ServerState>,
    Extension(current_user): Extension<CurrentUser>,
    Json(req): Json<EscalateRequest>,
) -> Result<Json<EscalateResponse>, AppError> {
    let username = req.username.clone();

    // Query employee by username (with hash for password verification)
    let emp_with_hash = employee::find_by_username_with_hash(&state.pool, &username).await?;

    // Fixed delay to prevent timing attacks
    tokio::time::sleep(Duration::from_millis(AUTH_FIXED_DELAY_MS)).await;

    // Check authentication result
    let emp = match emp_with_hash {
        Some(e) => {
            if !e.is_active {
                return Err(AppError::account_disabled());
            }

            let password_valid = employee::verify_password(&req.password, &e.hash_pass)
                .map_err(|e| AppError::internal(format!("Password verification failed: {}", e)))?;

            if !password_valid {
                tracing::warn!(
                    username = %username,
                    required_permission = %req.required_permission,
                    "Escalation failed - invalid credentials"
                );
                return Err(AppError::invalid_credentials());
            }

            e
        }
        None => {
            tracing::warn!(
                username = %username,
                required_permission = %req.required_permission,
                "Escalation failed - user not found"
            );
            return Err(AppError::invalid_credentials());
        }
    };

    // Fetch role information
    let role: Role = role::find_by_id(&state.pool, emp.role_id)
        .await?
        .ok_or_else(|| AppError::new(shared::ErrorCode::RoleNotFound))?;

    if !role.is_active {
        return Err(AppError::role_disabled());
    }

    // Check permission
    let has_permission = role.name == "admin"
        || role.permissions.iter().any(|p| p == "all")
        || role.permissions.iter().any(|p| p == &req.required_permission)
        || role.permissions.iter().any(|p| {
            // Wildcard match: "orders:*" matches "orders:void"
            if let Some(prefix) = p.strip_suffix(":*") {
                req.required_permission.starts_with(&format!("{}:", prefix))
            } else {
                false
            }
        });

    if !has_permission {
        tracing::warn!(
            authorizer = %username,
            required_permission = %req.required_permission,
            "Escalation failed - insufficient permission"
        );
        return Err(AppError::permission_denied("Insufficient permission")
            .with_detail("required_permission", req.required_permission.clone()));
    }

    let authorizer_id = emp.id;

    // Log successful escalation
    state.audit_service.log(
        AuditAction::EscalationSuccess,
        "auth",
        format!("employee:{}", authorizer_id),
        Some(authorizer_id),
        Some(emp.display_name.clone()),
        serde_json::json!({
            "authorizer_username": &emp.username,
            "required_permission": &req.required_permission,
            "requester_id": &current_user.id,
            "requester_name": &current_user.display_name,
        }),
    ).await;

    tracing::info!(
        authorizer_id = %authorizer_id,
        authorizer_username = %emp.username,
        required_permission = %req.required_permission,
        requester_id = %current_user.id,
        "Permission escalation successful"
    );

    let response = EscalateResponse {
        authorizer: UserInfo {
            id: emp.id,
            username: emp.username,
            display_name: emp.display_name,
            role_id: emp.role_id,
            role_name: role.name,
            permissions: role.permissions,
            is_system: emp.is_system,
            is_active: emp.is_active,
            created_at: emp.created_at,
        },
    };

    Ok(Json(response))
}
