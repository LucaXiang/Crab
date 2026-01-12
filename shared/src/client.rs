//! Client-related types shared between server and client
//!
//! Common response types used in API communication.

use serde::{Deserialize, Serialize};

// Re-export ApiResponse from response module
pub use crate::response::ApiResponse;

/// Login response data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginResponse {
    pub token: String,
    pub user: UserInfo,
}

/// User information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    pub id: String,
    pub username: String,
    pub role: String,
    #[serde(default)]
    pub permissions: Vec<String>,
}

/// Current user response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrentUserResponse {
    pub id: String,
    pub username: String,
    pub role: String,
    pub permissions: Vec<String>,
}
