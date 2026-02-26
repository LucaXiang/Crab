//! Role Model

use serde::{Deserialize, Serialize};

/// Role entity (RBAC 角色)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "db", derive(sqlx::FromRow))]
pub struct Role {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    /// JSON array of permission strings (e.g. ["*"], ["orders:read", "products:write"])
    #[cfg_attr(feature = "db", sqlx(json))]
    pub permissions: Vec<String>,
    pub is_system: bool,
    pub is_active: bool,
}

/// Create role payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleCreate {
    pub name: String,
    pub description: Option<String>,
    pub permissions: Vec<String>,
}

/// Update role payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleUpdate {
    pub name: Option<String>,
    pub description: Option<String>,
    pub permissions: Option<Vec<String>>,
    pub is_active: Option<bool>,
}
