//! Unified error system for Crab framework
//!
//! This module provides a comprehensive error handling system with:
//! - [`ErrorCode`]: Standardized error codes for all error types
//! - [`ErrorCategory`]: Classification of errors by domain
//! - [`AppError`]: Rich error type with codes, messages, and details
//! - [`ApiResponse`]: Unified API response format
//!
//! # Error Code Ranges
//!
//! - 0xxx: General errors
//! - 1xxx: Authentication errors
//! - 2xxx: Permission errors
//! - 3xxx: Tenant errors
//! - 4xxx: Order errors
//! - 5xxx: Payment errors
//! - 6xxx: Product errors
//! - 7xxx: Table errors
//! - 8xxx: Employee errors
//! - 9xxx: System errors
//!
//! # Example
//!
//! ```
//! use shared::error::{AppError, ErrorCode, ApiResponse};
//!
//! // Create a simple error
//! let err = AppError::new(ErrorCode::NotFound);
//!
//! // Create an error with custom message
//! let err = AppError::with_message(ErrorCode::ValidationFailed, "Invalid email format");
//!
//! // Create an error with details
//! let err = AppError::validation("Missing required field")
//!     .with_detail("field", "email");
//!
//! // Convert to API response
//! let response = ApiResponse::<()>::error(&err);
//! ```

mod category;
mod codes;
mod http;
mod types;

pub use category::ErrorCategory;
pub use codes::{ErrorCode, InvalidErrorCode};
pub use types::{ApiResponse, AppError, AppResult};
