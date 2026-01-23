//! Role Model

use super::serde_helpers;
use super::serde_thing;
use serde::{Deserialize, Serialize};
use surrealdb::sql::Thing;

/// Role ID type
pub type RoleId = Thing;

/// Role model matching SurrealDB schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Role {
    /// Optional role ID
    #[serde(default, with = "serde_thing::option")]
    pub id: Option<RoleId>,
    /// Name of the role
    pub role_name: String,
    /// List of permissions
    #[serde(default)]
    pub permissions: Vec<String>,
    /// Whether this is a system role
    #[serde(default, deserialize_with = "serde_helpers::bool_false")]
    pub is_system: bool,
    /// Whether the role is active
    #[serde(
        default = "default_true",
        deserialize_with = "serde_helpers::bool_true"
    )]
    pub is_active: bool,
}

fn default_true() -> bool {
    true
}

impl Role {
    /// Create a new role
    pub fn new(role_name: String, permissions: Vec<String>) -> Self {
        Self {
            id: None,
            role_name,
            permissions,
            is_system: false,
            is_active: true,
        }
    }
}

/// Create role request
#[derive(Debug, Deserialize)]
pub struct RoleCreate {
    pub role_name: String,
    #[serde(default)]
    pub permissions: Vec<String>,
}

/// Update role request
#[derive(Debug, Serialize, Deserialize)]
pub struct RoleUpdate {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permissions: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_active: Option<bool>,
}
