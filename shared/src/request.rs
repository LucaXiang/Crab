//! Request types for the shared crate
//!
//! Common request types used across the framework

/// Pagination query parameters
#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct PaginationQuery {
    /// Page number (1-based, default: 1)
    #[serde(default = "default_page")]
    pub page: u32,

    /// Items per page (default: 20, max: 100)
    #[serde(default = "default_per_page")]
    pub per_page: u32,
}

fn default_page() -> u32 {
    1
}

fn default_per_page() -> u32 {
    20
}

impl PaginationQuery {
    /// Get the offset for database queries
    pub fn offset(&self) -> u64 {
        (self.page.saturating_sub(1)) as u64 * self.per_page as u64
    }

    /// Get the limit (clamped to max 100)
    pub fn limit(&self) -> u32 {
        std::cmp::min(self.per_page, 100)
    }
}

/// Ordering query parameters
#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct OrderingQuery {
    /// Sort field (default: created_at)
    #[serde(default = "default_sort_field")]
    pub sort_by: String,

    /// Sort order (asc or desc, default: desc)
    #[serde(default = "default_sort_order")]
    pub order: String,
}

fn default_sort_field() -> String {
    "created_at".to_string()
}

fn default_sort_order() -> String {
    "desc".to_string()
}

impl OrderingQuery {
    /// Get sort direction (true for desc, false for asc)
    pub fn is_descending(&self) -> bool {
        self.order.to_lowercase() == "desc"
    }
}

/// Combined pagination and ordering query
#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct ListQuery {
    #[serde(flatten)]
    pub pagination: PaginationQuery,

    #[serde(flatten)]
    pub ordering: OrderingQuery,

    /// Search keyword
    #[serde(default)]
    pub search: Option<String>,
}
