//! Authentication Handlers
//!
//! Handles login, logout, and token management

use axum::{extract::State, Extension, Json};
use serde::{Deserialize, Serialize};

use crate::audit_log;
use crate::common::{ok, AppError, AppResponse};
use crate::db::models::{Employee, Role};
use crate::server::{ServerState, CurrentUser};

/// Login request payload
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

/// Login response with JWT token
#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub user: UserInfo,
}

/// User information returned after login
#[derive(Debug, Serialize)]
pub struct UserInfo {
    pub id: String,
    pub username: String,
    pub role: String,
    pub permissions: Vec<String>,
}

/// Login handler
///
/// Authenticates user credentials and returns a JWT token
pub async fn login(
    State(state): State<ServerState>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<AppResponse<LoginResponse>>, AppError> {
    let db = state.get_db();

    // Query employee by username
    let username = req.username.clone();
    let mut result = db
        .query("SELECT * FROM employee WHERE username = $username LIMIT 1")
        .bind(("username", username))
        .await
        .map_err(|e| AppError::database(format!("Query failed: {}", e)))?;

    let employee: Option<Employee> = result
        .take(0)
        .map_err(|e| AppError::database(format!("Failed to parse employee: {}", e)))?;

    let employee = employee.ok_or_else(|| {
        AppError::validation("Invalid username or password".to_string())
    })?;

    // Check if user is active
    if !employee.is_active {
        return Err(AppError::forbidden("Account has been disabled".to_string()));
    }

    // Verify password using argon2
    let password_valid = employee
        .verify_password(&req.password)
        .map_err(|e| AppError::internal(format!("Password verification failed: {}", e)))?;

    if !password_valid {
        return Err(AppError::validation("Invalid username or password".to_string()));
    }

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

    let role = role.ok_or_else(|| {
        AppError::internal("Role not found".to_string())
    })?;

    if !role.is_active {
        return Err(AppError::forbidden("Role has been disabled".to_string()));
    }

    // Generate JWT token
    let jwt_service = state.get_jwt_service();
    let user_id = employee.id
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

    // Log successful login (audit log)
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

    Ok(ok(response))
}

/// Get current user info
///
/// Returns the current authenticated user's information
pub async fn me(
    Extension(user): Extension<CurrentUser>,
) -> Result<Json<AppResponse<UserInfo>>, AppError> {
    let user_info = UserInfo {
        id: user.id,
        username: user.username,
        role: user.role,
        permissions: user.permissions,
    };

    Ok(ok(user_info))
}

/// Logout handler (client-side token invalidation)
///
/// Since JWTs are stateless, logout is typically handled client-side
/// by removing the token. This endpoint is mainly for audit logging.
pub async fn logout(
    Extension(user): Extension<CurrentUser>,
) -> Result<Json<AppResponse<()>>, AppError> {
    // Log logout event
    audit_log!(&user.id, "logout", &user.username);

    tracing::info!(
        user_id = %user.id,
        username = %user.username,
        "User logged out"
    );

    Ok(ok(()))
}
