//! Role Model

use serde::{Deserialize, Serialize};
use surrealdb::sql::Thing;
/// Role ID type
pub type RoleId = Thing;

/// Role model matching SurrealDB schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Role {
    /// Optional role ID
    #[serde(skip)]
    pub id: Option<RoleId>,
    /// Name of the role
    pub role_name: String,
    /// List of permissions
    pub permissions: Vec<String>,
    /// Whether this is a system role
    pub is_system: bool,
    /// Whether the role is active
    pub is_active: bool,
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
