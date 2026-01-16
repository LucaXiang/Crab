//! 认证与授权模块
//!
//! 处理 JWT 令牌生成/验证、权限和认证中间件

mod auth_middleware;
mod extractor;
mod jwt;
mod permissions;

pub use auth_middleware::{CurrentUserExt, require_admin, require_auth, require_permission};
pub use jwt::{Claims, CurrentUser, JwtConfig, JwtError, JwtService};
pub use permissions::{
    ALL_PERMISSIONS, DEFAULT_ADMIN_PERMISSIONS, DEFAULT_MANAGER_PERMISSIONS,
    DEFAULT_USER_PERMISSIONS, get_default_permissions, is_valid_permission,
};
