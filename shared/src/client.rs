//! Client-related types shared between server and client
//!
//! Common request/response types used in API communication.
//! These types are shared between edge-server and crab-client.

use serde::{Deserialize, Serialize};

// Re-export ApiResponse from response module
pub use crate::response::ApiResponse;

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
