//! 认证与授权模块
//!
//! 处理 JWT 令牌生成/验证、权限和认证中间件

mod jwt;
mod permissions;
mod auth_middleware;
mod extractor;

pub use jwt::{Claims, CurrentUser, JwtConfig, JwtError, JwtService};
pub use permissions::{
    get_default_permissions, is_valid_permission, ALL_PERMISSIONS,
    DEFAULT_ADMIN_PERMISSIONS, DEFAULT_MANAGER_PERMISSIONS, DEFAULT_USER_PERMISSIONS,
};
pub use auth_middleware::{require_admin, require_auth, require_permission, CurrentUserExt};
