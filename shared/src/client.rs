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
    pub id: String,
    pub username: String,
    pub display_name: String,
    pub role_id: String,
    pub role_name: String,
    #[serde(default)]
    pub permissions: Vec<String>,
    #[serde(default)]
    pub is_system: bool,
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
