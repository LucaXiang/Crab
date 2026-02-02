//! Shared Types
//!
//! Common types used across the application

use serde::Deserialize;

/// Pagination query parameters
#[derive(Debug, Clone, Deserialize)]
pub struct PaginationParams {
    #[serde(default = "default_page")]
    pub page: u32,

    #[serde(default = "default_page_size")]
    pub page_size: u32,
}

fn default_page() -> u32 {
    1
}

fn default_page_size() -> u32 {
    20
}

impl Default for PaginationParams {
    fn default() -> Self {
        Self {
            page: default_page(),
            page_size: default_page_size(),
        }
    }
}

impl PaginationParams {
    /// Calculate offset for SQL queries
    pub fn offset(&self) -> u32 {
        (self.page - 1) * self.page_size
    }

    /// Get limit for SQL queries
    pub fn limit(&self) -> u32 {
        self.page_size
    }
}

