//! 认证授权模块
//!
//! 提供 JWT 认证、权限管理和中间件：
//! - [`JwtService`] - JWT 令牌服务
//! - [`CurrentUser`] - 当前用户上下文
//! - [`require_auth`] - 认证中间件
//! - [`require_permission`] - 权限检查中间件

pub mod extractor;
pub mod jwt;
pub mod middleware;
pub mod permissions;

pub use jwt::{Claims, CurrentUser, JwtConfig, JwtError, JwtService};
pub use middleware::{CurrentUserExt, require_admin, require_auth, require_permission};
