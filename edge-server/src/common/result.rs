//! Unified Result Types
//!
//! Provides type aliases for commonly used Result types across the application

use shared::error::ApiError;

/// Application-level Result type
///
/// Used in HTTP handlers and application logic
pub type AppResult<T> = Result<T, ApiError>;

// NOTE: Uncomment these when integrating with existing error types
//
// /// Service-level Result type
// ///
// /// Used in business logic services
// pub type ServiceResult<T> = Result<T, crate::error::ServiceError>;
//
// /// Repository-level Result type
// ///
// /// Used in data access layer
// pub type RepoResult<T> = Result<T, crate::repository::DbError>;
