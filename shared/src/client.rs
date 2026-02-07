//! Client-related types shared between server and client
//!
//! Common request/response types used in API communication.
//! These types are shared between edge-server and crab-client.

use serde::{Deserialize, Serialize};

// =============================================================================
// Auth API DTOs
// =============================================================================

/// Login request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

/// Login response data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginResponse {
    pub token: String,
    pub user: UserInfo,
}

/// User information returned after login
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    pub id: i64,
    pub username: String,
    pub display_name: String,
    pub role_id: i64,
    pub role_name: String,
    pub permissions: Vec<String>,
    pub is_system: bool,
    pub is_active: bool,
    pub created_at: i64,
}

/// Current user response (same as UserInfo)
pub type CurrentUserResponse = UserInfo;

/// Escalation request (supervisor authorization)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EscalateRequest {
    /// 授权人用户名
    pub username: String,
    /// 授权人密码
    pub password: String,
    /// 所需权限
    pub required_permission: String,
}

/// Escalation response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EscalateResponse {
    /// 授权人信息
    pub authorizer: UserInfo,
}
