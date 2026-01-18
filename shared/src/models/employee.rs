//! Employee Model

use serde::{Deserialize, Serialize};

/// Employee response (without password)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmployeeResponse {
    pub id: String,
    pub username: String,
    /// Role reference (String ID)
    pub role: String,
    pub is_active: bool,
}

/// Create employee payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmployeeCreate {
    pub username: String,
    pub password: String,
    /// Role reference (String ID)
    pub role: String,
}

/// Update employee payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmployeeUpdate {
    pub username: Option<String>,
    pub password: Option<String>,
    /// Role reference (String ID)
    pub role: Option<String>,
    pub is_active: Option<bool>,
}
