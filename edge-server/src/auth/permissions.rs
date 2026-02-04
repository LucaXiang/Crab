//! Permission Definitions
//!
//! Simplified RBAC permission system.
//!
//! ## 设计原则
//! - 基础操作（查看菜单、订单、基础POS操作）无需权限，登录即可使用
//! - 模块化权限：按功能模块授权
//! - 敏感操作：单独控制高风险操作
//! - 用户管理：仅 admin 角色可用（is_system 保护）

/// 可配置权限列表（12 项）
/// 不包含 "all" 和 "users:manage"，这些是系统级权限
pub const ALL_PERMISSIONS: &[&str] = &[
    // === 模块化权限 (6) ===
    "menu:manage",         // 菜单管理（商品/分类/属性/标签 增删改查）
    "tables:manage",       // 桌台管理（区域/餐桌 增删改查）
    "shifts:manage",       // 班次管理
    "reports:view",        // 报表查看
    "price_rules:manage",  // 价格规则管理
    "settings:manage",     // 系统设置

    // === 敏感操作 (6) ===
    "orders:void",         // 作废订单
    "orders:discount",     // 应用折扣/附加费
    "orders:comp",         // 赠送菜品
    "orders:refund",       // 退款
    "orders:modify_price", // 修改价格
    "cash_drawer:open",    // 打开钱箱
];

/// Admin 专属权限（不在可配置列表中）
pub const ADMIN_ONLY_PERMISSIONS: &[&str] = &[
    "users:manage", // 用户管理
    "all",          // 超级权限
];

/// Default role permissions
pub const DEFAULT_ADMIN_PERMISSIONS: &[&str] = &["all"];

/// 经理角色默认权限（全部可配置权限）
pub const DEFAULT_MANAGER_PERMISSIONS: &[&str] = &[
    // 模块化
    "menu:manage",
    "tables:manage",
    "shifts:manage",
    "reports:view",
    "price_rules:manage",
    "settings:manage",
    // 敏感操作
    "orders:void",
    "orders:discount",
    "orders:comp",
    "orders:refund",
    "orders:modify_price",
    "cash_drawer:open",
];

/// 普通员工默认权限（仅查看报表）
pub const DEFAULT_USER_PERMISSIONS: &[&str] = &[
    "reports:view",
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
        _ => vec![],
    }
}

/// Validate if a permission string is valid
pub fn is_valid_permission(permission: &str) -> bool {
    ALL_PERMISSIONS.contains(&permission)
        || ADMIN_ONLY_PERMISSIONS.contains(&permission)
        || permission.ends_with(":*")
}
