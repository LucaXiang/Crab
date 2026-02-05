//! Authentication Handlers
//!
//! Handles login, logout, and token management

use std::time::Duration;

use axum::{Extension, Json, extract::State};

use crate::audit::AuditAction;
use crate::auth::CurrentUser;
use crate::core::ServerState;
use crate::db::models::{Employee, Role};
use crate::AppError;

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
    let db = state.get_db();
    let username = req.username.clone();

    // Query employee by username
    let mut result = db
        .query("SELECT * FROM employee WHERE username = $username LIMIT 1")
        .bind(("username", username.clone()))
        .await
        .map_err(|e| AppError::database(format!("Query failed: {}", e)))?;

    let employee: Option<Employee> = result
        .take(0)
        .map_err(|e| AppError::database(format!("Failed to parse employee: {}", e)))?;

    // Fixed delay to prevent timing attacks (before checking result)
    tokio::time::sleep(Duration::from_millis(AUTH_FIXED_DELAY_MS)).await;

    // Check authentication result - unified error message to prevent username enumeration
    let employee = match employee {
        Some(e) => {
            // User found - check active status
            if !e.is_active {
                return Err(AppError::forbidden("Account has been disabled".to_string()));
            }

            // Verify password
            let password_valid = e
                .verify_password(&req.password)
                .map_err(|e| AppError::internal(format!("Password verification failed: {}", e)))?;

            if !password_valid {
                state.audit_service.log(
                    AuditAction::LoginFailed, "auth", format!("employee:{}", username),
                    None, None,
                    serde_json::json!({"reason": "invalid_credentials"}),
                ).await;
                tracing::warn!(username = %username, "Login failed - invalid credentials");
                return Err(AppError::invalid(
                    "Invalid username or password".to_string(),
                ));
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
            return Err(AppError::invalid(
                "Invalid username or password".to_string(),
            ));
        }
    };

    // Fetch role information
    let role_id = employee.role.clone();
    let mut role_result = db
        .query("SELECT * FROM $role_id")
        .bind(("role_id", role_id))
        .await
        .map_err(|e| AppError::database(format!("Failed to query role: {}", e)))?;

    let role: Option<Role> = role_result
        .take(0)
        .map_err(|e| AppError::database(format!("Failed to parse role: {}", e)))?;

    let role = role.ok_or_else(|| AppError::internal("Role not found".to_string()))?;

    if !role.is_active {
        return Err(AppError::forbidden("Role has been disabled".to_string()));
    }

    // Generate JWT token
    let jwt_service = state.get_jwt_service();
    let user_id = employee
        .id
        .as_ref()
        .map(|t| t.to_string())
        .unwrap_or_default();

    let token = jwt_service
        .generate_token(
            &user_id,
            &employee.username,
            &employee.display_name,
            &employee.role.to_string(),
            &role.name,
            &role.permissions,
            employee.is_system,
        )
        .map_err(|e| AppError::internal(format!("Failed to generate token: {}", e)))?;

    // Log successful login
    state.audit_service.log(
        AuditAction::LoginSuccess, "auth", format!("employee:{}", user_id),
        Some(user_id.clone()), Some(employee.display_name.clone()),
        serde_json::json!({"username": &employee.username}),
    ).await;

    tracing::info!(
        user_id = %user_id,
        username = %employee.username,
        role = %role.name,
        "User logged in successfully"
    );

    let response = LoginResponse {
        token,
        user: UserInfo {
            id: user_id,
            username: employee.username.clone(),
            display_name: employee.display_name.clone(),
            role_id: employee.role.to_string(),
            role_name: role.name,
            permissions: role.permissions,
            is_system: employee.is_system,
            is_active: employee.is_active,
            created_at: employee.created_at,
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
    let db = state.get_db();
    let id_part = user.id.strip_prefix("employee:").unwrap_or(&user.id).to_string();
    let mut result = db
        .query("SELECT is_active, created_at FROM type::thing('employee', $id)")
        .bind(("id", id_part))
        .await
        .map_err(|e| AppError::database(format!("Failed to query employee: {}", e)))?;

    let employee_data: Option<(bool, i64)> = result
        .take::<Option<Employee>>(0)
        .map_err(|e| AppError::database(format!("Failed to parse employee: {}", e)))?
        .map(|e| (e.is_active, e.created_at));

    let (is_active, created_at) = employee_data.unwrap_or((true, 0));

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
        Some(user.id.clone()), Some(user.display_name.clone()),
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
    let db = state.get_db();
    let username = req.username.clone();

    // Query employee by username
    let mut result = db
        .query("SELECT * FROM employee WHERE username = $username LIMIT 1")
        .bind(("username", username.clone()))
        .await
        .map_err(|e| AppError::database(format!("Query failed: {}", e)))?;

    let employee: Option<Employee> = result
        .take(0)
        .map_err(|e| AppError::database(format!("Failed to parse employee: {}", e)))?;

    // Fixed delay to prevent timing attacks
    tokio::time::sleep(Duration::from_millis(AUTH_FIXED_DELAY_MS)).await;

    // Check authentication result
    let employee = match employee {
        Some(e) => {
            if !e.is_active {
                return Err(AppError::forbidden("Account has been disabled".to_string()));
            }

            let password_valid = e
                .verify_password(&req.password)
                .map_err(|e| AppError::internal(format!("Password verification failed: {}", e)))?;

            if !password_valid {
                tracing::warn!(
                    username = %username,
                    required_permission = %req.required_permission,
                    "Escalation failed - invalid credentials"
                );
                return Err(AppError::invalid("Invalid username or password".to_string()));
            }

            e
        }
        None => {
            tracing::warn!(
                username = %username,
                required_permission = %req.required_permission,
                "Escalation failed - user not found"
            );
            return Err(AppError::invalid("Invalid username or password".to_string()));
        }
    };

    // Fetch role information
    let role_id = employee.role.clone();
    let mut role_result = db
        .query("SELECT * FROM $role_id")
        .bind(("role_id", role_id))
        .await
        .map_err(|e| AppError::database(format!("Failed to query role: {}", e)))?;

    let role: Option<Role> = role_result
        .take(0)
        .map_err(|e| AppError::database(format!("Failed to parse role: {}", e)))?;

    let role = role.ok_or_else(|| AppError::internal("Role not found".to_string()))?;

    if !role.is_active {
        return Err(AppError::forbidden("Role has been disabled".to_string()));
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
        return Err(AppError::forbidden(format!(
            "User does not have permission: {}",
            req.required_permission
        )));
    }

    let authorizer_id = employee
        .id
        .as_ref()
        .map(|t| t.to_string())
        .unwrap_or_default();

    // Log successful escalation
    state.audit_service.log(
        AuditAction::EscalationSuccess,
        "auth",
        format!("employee:{}", authorizer_id),
        Some(authorizer_id.clone()),
        Some(employee.display_name.clone()),
        serde_json::json!({
            "authorizer_username": &employee.username,
            "required_permission": &req.required_permission,
            "requester_id": &current_user.id,
            "requester_name": &current_user.display_name,
        }),
    ).await;

    tracing::info!(
        authorizer_id = %authorizer_id,
        authorizer_username = %employee.username,
        required_permission = %req.required_permission,
        requester_id = %current_user.id,
        "Permission escalation successful"
    );

    let response = EscalateResponse {
        authorizer: UserInfo {
            id: authorizer_id,
            username: employee.username,
            display_name: employee.display_name,
            role_id: employee.role.to_string(),
            role_name: role.name,
            permissions: role.permissions,
            is_system: employee.is_system,
            is_active: employee.is_active,
            created_at: employee.created_at,
        },
    };

    Ok(Json(response))
}
