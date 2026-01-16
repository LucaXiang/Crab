//! Unified Result Types
//!
//! Provides type aliases for commonly used Result types across the application

use crate::AppError;

/// Application-level Result type
///
/// Used in HTTP handlers and application logic
pub type AppResult<T> = Result<T, AppError>;
