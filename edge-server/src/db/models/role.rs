//! Role Model

use super::serde_helpers;
use serde::{Deserialize, Serialize};
use surrealdb::RecordId;

/// Role ID type
pub type RoleId = RecordId;

/// Role model matching SurrealDB schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Role {
    /// Optional role ID
    #[serde(default, with = "serde_helpers::option_record_id")]
    pub id: Option<RoleId>,
    /// Unique name of the role (e.g., "admin", "cashier")
    pub name: String,
    /// Display name for UI (e.g., "管理员", "收银员")
    #[serde(default)]
    pub display_name: String,
    /// Description of the role
    pub description: Option<String>,
    /// List of permissions
    #[serde(default)]
    pub permissions: Vec<String>,
    /// Whether this is a system role
    pub is_system: bool,
    /// Whether the role is active
    pub is_active: bool,
}

impl Role {
    /// Create a new role
    pub fn new(name: String, permissions: Vec<String>) -> Self {
        Self {
            id: None,
            name: name.clone(),
            display_name: name,
            description: None,
            permissions,
            is_system: false,
            is_active: true,
        }
    }
}

/// Create role request
#[derive(Debug, Deserialize)]
pub struct RoleCreate {
    pub name: String,
    #[serde(default)]
    pub display_name: Option<String>,
    pub description: Option<String>,
    #[serde(default)]
    pub permissions: Vec<String>,
}

/// Update role request
#[derive(Debug, Serialize, Deserialize)]
pub struct RoleUpdate {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permissions: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_active: Option<bool>,
}
