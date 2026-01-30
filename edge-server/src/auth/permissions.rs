//! Permission Definitions
//!
//! Defines all available permissions for the RBAC system.

/// All available permissions in the system
pub const ALL_PERMISSIONS: &[&str] = &[
    // Product permissions
    "products:read",
    "products:write",
    "products:delete",
    "products:manage",
    // Category permissions
    "categories:read",
    "categories:manage",
    // Attribute permissions
    "attributes:read",
    "attributes:manage",
    // Order permissions
    "orders:read",
    "orders:write",
    "orders:void",
    "orders:restore",
    "orders:discount",
    "orders:refund",
    "orders:cancel_item",
    // User management permissions
    "users:read",
    "users:manage",
    // Role management permissions
    "roles:read",
    "roles:write",
    // Zone & Table permissions
    "zones:read",
    "zones:manage",
    "tables:read",
    "tables:manage",
    "tables:merge_bill",
    "tables:transfer",
    // Pricing permissions
    "pricing:read",
    "pricing:write",
    // Statistics permissions
    "statistics:read",
    // Printer permissions
    "printers:read",
    "printers:manage",
    // Receipt permissions
    "receipts:print",
    "receipts:reprint",
    // Settings & System permissions
    "settings:manage",
    "system:read",
    "system:write",
    // POS operations
    "pos:cash_drawer",
    // Admin permission (grants all access)
    "all",
];

/// Default role permissions
pub const DEFAULT_ADMIN_PERMISSIONS: &[&str] = &["all"];

pub const DEFAULT_USER_PERMISSIONS: &[&str] = &[
    "products:read",
    "categories:read",
    "attributes:read",
    "orders:read",
    "orders:write",
    "zones:read",
    "tables:read",
    "pricing:read",
    "statistics:read",
    "printers:read",
    "receipts:print",
];

pub const DEFAULT_MANAGER_PERMISSIONS: &[&str] = &[
    "products:read",
    "products:write",
    "products:manage",
    "categories:read",
    "categories:manage",
    "attributes:read",
    "attributes:manage",
    "orders:read",
    "orders:write",
    "orders:void",
    "orders:restore",
    "orders:discount",
    "orders:refund",
    "orders:cancel_item",
    "zones:read",
    "zones:manage",
    "tables:read",
    "tables:manage",
    "tables:merge_bill",
    "tables:transfer",
    "pricing:read",
    "pricing:write",
    "statistics:read",
    "printers:read",
    "printers:manage",
    "receipts:print",
    "receipts:reprint",
    "settings:manage",
    "pos:cash_drawer",
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
