//! Authentication Handlers
//!
//! Handles login, logout, and token management

use std::time::Duration;

use axum::{Extension, Json, extract::State};

use crate::audit_log;
use crate::auth::CurrentUser;
use crate::core::ServerState;
use crate::db::models::{Employee, Role};
use crate::{AppError, AppResponse};

// Re-use shared DTOs for API consistency
use shared::client::{LoginRequest, LoginResponse, UserInfo};

/// Fixed delay for authentication to prevent timing attacks
const AUTH_FIXED_DELAY_MS: u64 = 500;

/// Login handler
///
/// Authenticates user credentials and returns a JWT token
pub async fn login(
    State(state): State<ServerState>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<AppResponse<LoginResponse>>, AppError> {
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
                // Log failed login for audit
                audit_log!(&username, "login_failed", &username);
                tracing::warn!(username = %username, "Login failed - invalid credentials");
                return Err(AppError::invalid(
                    "Invalid username or password".to_string(),
                ));
            }

            e
        }
        None => {
            // User not found - log and return same error as wrong password
            audit_log!(&username, "login_failed", &username);
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
            &role.role_name,
            &role.permissions,
        )
        .map_err(|e| AppError::internal(format!("Failed to generate token: {}", e)))?;

    // Log successful login
    audit_log!(&user_id, "login", &req.username);

    tracing::info!(
        user_id = %user_id,
        username = %employee.username,
        role = %role.role_name,
        "User logged in successfully"
    );

    let response = LoginResponse {
        token,
        user: UserInfo {
            id: user_id,
            username: employee.username,
            role: role.role_name,
            permissions: role.permissions,
        },
    };

    Ok(crate::ok!(response))
}

/// Get current user info
pub async fn me(
    Extension(user): Extension<CurrentUser>,
) -> Result<Json<AppResponse<UserInfo>>, AppError> {
    let user_info = UserInfo {
        id: user.id,
        username: user.username,
        role: user.role,
        permissions: user.permissions,
    };

    Ok(crate::ok!(user_info))
}

/// Logout handler
pub async fn logout(
    Extension(user): Extension<CurrentUser>,
) -> Result<Json<AppResponse<()>>, AppError> {
    audit_log!(&user.id, "logout", &user.username);

    tracing::info!(
        user_id = %user.id,
        username = %user.username,
        "User logged out"
    );

    Ok(crate::ok!(()))
}
