//! Authentication and Authorization Module
//!
//! Handles JWT token generation/validation, permissions, and auth middleware

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
