//! Common utilities and shared infrastructure
//!
//! This module contains core infrastructure used across the application:
//! - Configuration management
//! - Logging setup
//! - Error handling
//! - Result types
//! - Shared types

// pub mod config;
pub mod error;
pub mod logger;
pub mod result;
pub mod types;

// Re-export commonly used items
// pub use config::AppConfig;
pub use error::{ok, ok_with_message, AppError, AppResponse};
pub use logger::{init_logger, init_logger_with_file};
pub use result::AppResult;
