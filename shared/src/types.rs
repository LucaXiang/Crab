//! Common types for the shared crate
//!
//! Utility types used across the framework

use serde::{Deserialize, Serialize};

/// Timestamp type (Unix milliseconds)
pub type Timestamp = i64;

/// Permission type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Permission(pub String);

impl Permission {
    /// Check if this permission grants access to the given resource action
    pub fn grants(&self, action: &str) -> bool {
        if self.0 == "*" {
            return true;
        }
        if self.0.ends_with(":*") {
            let prefix = &self.0[..self.0.len() - 2];
            return action.starts_with(prefix);
        }
        self.0 == action
    }
}

impl std::fmt::Display for Permission {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
