//! Permission Definitions
//!
//! Defines all available permissions for the RBAC system.

/// All available permissions in the system
pub const ALL_PERMISSIONS: &[&str] = &[
    // Product permissions
    "products:read",
    "products:write",
    "products:delete",
    // Category permissions
    "categories:read",
    "categories:write",
    // Order permissions
    "orders:read",
    "orders:write",
    "orders:cancel",
    // User management permissions
    "users:read",
    "users:write",
    // Role management permissions
    "roles:read",
    "roles:write",
    // System permissions
    "system:read",
    "system:write",
    // Admin permission (grants all access)
    "all",
];

/// Default role permissions
pub const DEFAULT_ADMIN_PERMISSIONS: &[&str] = &["all"];

pub const DEFAULT_USER_PERMISSIONS: &[&str] = &["products:read", "categories:read"];

pub const DEFAULT_MANAGER_PERMISSIONS: &[&str] = &[
    "products:read",
    "products:write",
    "categories:read",
    "categories:write",
    "users:read",
];

/// Get permissions for a role name
pub fn get_default_permissions(role_name: &str) -> Vec<String> {
    match role_name {
        "admin" => DEFAULT_ADMIN_PERMISSIONS
            .iter()
            .map(|s| s.to_string())
            .collect(),
        "manager" => DEFAULT_MANAGER_PERMISSIONS
            .iter()
            .map(|s| s.to_string())
            .collect(),
        "user" => DEFAULT_USER_PERMISSIONS
            .iter()
            .map(|s| s.to_string())
            .collect(),
        _ => vec!["products:read".to_string()],
    }
}

/// Validate if a permission string is valid
pub fn is_valid_permission(permission: &str) -> bool {
    ALL_PERMISSIONS.contains(&permission) || permission.ends_with(":*")
}
