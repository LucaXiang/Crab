//! Employee Model

use serde::{Deserialize, Serialize};

/// Employee entity (without password hash)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "db", derive(sqlx::FromRow))]
pub struct Employee {
    pub id: i64,
    pub username: String,
    pub name: String,
    pub role_id: i64,
    pub is_system: bool,
    pub is_active: bool,
    pub created_at: i64,
}

/// Create employee payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmployeeCreate {
    pub username: String,
    pub password: String,
    pub name: Option<String>,
    pub role_id: i64,
}

/// Update employee payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmployeeUpdate {
    pub username: Option<String>,
    pub password: Option<String>,
    pub name: Option<String>,
    pub role_id: Option<i64>,
    pub is_active: Option<bool>,
}
