//! Employee Model

use serde::{Deserialize, Serialize};

/// Employee entity (without password)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Employee {
    pub id: Option<String>,
    pub username: String,
    /// Display name (e.g., "张三")
    pub display_name: String,
    /// Role reference (String ID)
    pub role: String,
    /// System-created employee (e.g., built-in admin)
    #[serde(default)]
    pub is_system: bool,
    pub is_active: bool,
}

/// Create employee payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmployeeCreate {
    pub username: String,
    pub password: String,
    pub display_name: Option<String>,
    /// Role reference (String ID)
    pub role: String,
}

/// Update employee payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmployeeUpdate {
    pub username: Option<String>,
    pub password: Option<String>,
    pub display_name: Option<String>,
    /// Role reference (String ID)
    pub role: Option<String>,
    pub is_active: Option<bool>,
}
