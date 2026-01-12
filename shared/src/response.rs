//! API Response types
//!
//! Standardized API response structures for the entire framework

use serde::{Serialize, Deserialize};

/// Standard API response code
pub const API_CODE_SUCCESS: &str = "E0000";

/// Unified API response structure
///
/// All API responses follow this format:
/// ```json
/// {
///     "code": "E0000",
///     "message": "Success",
///     "data": { ... }
/// }
/// ```
#[derive(Debug, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    /// Response code (E0000 = success, others = error codes)
    pub code: String,
    /// Human-readable message
    pub message: String,
    /// Response data (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    /// Request trace ID for debugging (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<String>,
}

impl<T> ApiResponse<T> {
    /// Create a successful response
    pub fn ok(data: T) -> Self {
        Self {
            code: API_CODE_SUCCESS.to_string(),
            message: "Success".to_string(),
            data: Some(data),
            trace_id: None,
        }
    }

    /// Create a successful response with custom message
    pub fn ok_with_message(data: T, message: impl Into<String>) -> Self {
        Self {
            code: API_CODE_SUCCESS.to_string(),
            message: message.into(),
            data: Some(data),
            trace_id: None,
        }
    }

    /// Create an error response
    pub fn error(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            data: None,
            trace_id: None,
        }
    }

    /// Create an error response with data
    pub fn error_with_data(code: impl Into<String>, message: impl Into<String>, data: T) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            data: Some(data),
            trace_id: None,
        }
    }

    /// Add trace ID to response
    pub fn with_trace_id(mut self, trace_id: impl Into<String>) -> Self {
        self.trace_id = Some(trace_id.into());
        self
    }
}

impl<T> Default for ApiResponse<T> where T: Default {
    fn default() -> Self {
        Self {
            code: API_CODE_SUCCESS.to_string(),
            message: "Success".to_string(),
            data: Some(T::default()),
            trace_id: None,
        }
    }
}

/// Pagination metadata
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Pagination {
    /// Current page number (1-based)
    pub page: u32,
    /// Items per page
    pub per_page: u32,
    /// Total number of items
    pub total: u64,
    /// Total number of pages
    pub total_pages: u32,
}

impl Pagination {
    /// Create a new pagination
    pub fn new(page: u32, per_page: u32, total: u64) -> Self {
        let total_pages = if per_page == 0 {
            0
        } else {
            ((total as f64) / (per_page as f64)).ceil() as u32
        };
        Self {
            page,
            per_page,
            total,
            total_pages,
        }
    }
}

/// Paginated response wrapper
#[derive(Debug, Serialize)]
pub struct PaginatedResponse<T> {
    /// List of items
    pub items: Vec<T>,
    /// Pagination metadata
    pub pagination: Pagination,
}

impl<T> PaginatedResponse<T> {
    /// Create a new paginated response
    pub fn new(items: Vec<T>, page: u32, per_page: u32, total: u64) -> Self {
        Self {
            items,
            pagination: Pagination::new(page, per_page, total),
        }
    }
}

/// Empty response (unit type)
#[derive(Debug, Clone, Copy)]
pub struct Empty;

impl Serialize for Empty {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_none()
    }
}
