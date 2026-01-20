# Unified Error System Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement a unified, maintainable, i18n-ready error system across edge-server, tauri, and react frontend.

**Architecture:** Single source of truth in `shared/src/error/`, auto-generate TypeScript types via build.rs, numeric error codes (0-9999) with category-based HTTP status mapping.

**Tech Stack:** Rust (thiserror, serde), TypeScript, i18next

---

## Phase 1: Shared Crate Error Module

### Task 1.1: Create Error Code Enum

**Files:**
- Create: `shared/src/error/mod.rs`
- Create: `shared/src/error/codes.rs`

**Step 1: Create the error module directory structure**

```bash
mkdir -p shared/src/error
```

**Step 2: Create `shared/src/error/codes.rs`**

```rust
//! Unified error codes for Crab framework
//!
//! Error code ranges:
//! - 0xxx: General/Validation
//! - 1xxx: Authentication
//! - 2xxx: Authorization
//! - 3xxx: Tenant/Activation
//! - 4xxx: Order
//! - 5xxx: Payment
//! - 6xxx: Product/Category/Spec
//! - 7xxx: Table/Zone
//! - 8xxx: Employee/Role
//! - 9xxx: System/Infrastructure

use serde::{Deserialize, Serialize};

/// Unified error code enum
///
/// The numeric value is the error code (e.g., `NotAuthenticated = 1001` means code 1001)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(into = "u16", try_from = "u16")]
#[repr(u16)]
pub enum ErrorCode {
    // ===== 0xxx: General =====
    /// Success (special case)
    Success = 0,
    /// Unknown error
    Unknown = 1,
    /// Validation failed
    ValidationFailed = 2,
    /// Resource not found
    NotFound = 3,
    /// Resource already exists
    AlreadyExists = 4,
    /// Invalid request
    InvalidRequest = 5,
    /// Invalid format
    InvalidFormat = 6,
    /// Required field missing
    RequiredField = 7,
    /// Value out of range
    ValueOutOfRange = 8,

    // ===== 1xxx: Authentication =====
    /// Not authenticated
    NotAuthenticated = 1001,
    /// Invalid credentials
    InvalidCredentials = 1002,
    /// Token expired
    TokenExpired = 1003,
    /// Token invalid
    TokenInvalid = 1004,
    /// Session expired
    SessionExpired = 1005,
    /// Account locked
    AccountLocked = 1006,
    /// Account disabled
    AccountDisabled = 1007,

    // ===== 2xxx: Authorization =====
    /// Permission denied
    PermissionDenied = 2001,
    /// Role required
    RoleRequired = 2002,
    /// Admin required
    AdminRequired = 2003,
    /// Cannot modify admin
    CannotModifyAdmin = 2004,
    /// Cannot delete admin
    CannotDeleteAdmin = 2005,

    // ===== 3xxx: Tenant/Activation =====
    /// Tenant not selected
    TenantNotSelected = 3001,
    /// Tenant not found
    TenantNotFound = 3002,
    /// Activation failed
    ActivationFailed = 3003,
    /// Certificate invalid
    CertificateInvalid = 3004,
    /// License expired
    LicenseExpired = 3005,

    // ===== 4xxx: Order =====
    /// Order not found
    OrderNotFound = 4001,
    /// Order already paid
    OrderAlreadyPaid = 4002,
    /// Order already completed
    OrderAlreadyCompleted = 4003,
    /// Order already voided
    OrderAlreadyVoided = 4004,
    /// Order has payments
    OrderHasPayments = 4005,
    /// Order item not found
    OrderItemNotFound = 4006,
    /// Order is empty
    OrderEmpty = 4007,

    // ===== 5xxx: Payment =====
    /// Payment failed
    PaymentFailed = 5001,
    /// Insufficient payment amount
    PaymentInsufficientAmount = 5002,
    /// Invalid payment method
    PaymentInvalidMethod = 5003,
    /// Payment already refunded
    PaymentAlreadyRefunded = 5004,
    /// Refund exceeds payment amount
    PaymentRefundExceedsAmount = 5005,

    // ===== 6xxx: Product/Category/Spec =====
    /// Product not found
    ProductNotFound = 6001,
    /// Invalid product price
    ProductInvalidPrice = 6002,
    /// Product out of stock
    ProductOutOfStock = 6003,
    /// Category not found
    CategoryNotFound = 6101,
    /// Category has products
    CategoryHasProducts = 6102,
    /// Category name exists
    CategoryNameExists = 6103,
    /// Specification not found
    SpecNotFound = 6201,
    /// Attribute not found
    AttributeNotFound = 6301,
    /// Attribute bind failed
    AttributeBindFailed = 6302,

    // ===== 7xxx: Table/Zone =====
    /// Table not found
    TableNotFound = 7001,
    /// Table occupied
    TableOccupied = 7002,
    /// Table already empty
    TableAlreadyEmpty = 7003,
    /// Zone not found
    ZoneNotFound = 7101,
    /// Zone has tables
    ZoneHasTables = 7102,
    /// Zone name exists
    ZoneNameExists = 7103,

    // ===== 8xxx: Employee/Role =====
    /// Employee not found
    EmployeeNotFound = 8001,
    /// Employee username exists
    EmployeeUsernameExists = 8002,
    /// Cannot delete self
    EmployeeCannotDeleteSelf = 8003,
    /// Role not found
    RoleNotFound = 8101,
    /// Role name exists
    RoleNameExists = 8102,
    /// Role in use
    RoleInUse = 8103,

    // ===== 9xxx: System =====
    /// Internal error
    InternalError = 9001,
    /// Database error
    DatabaseError = 9002,
    /// Network error
    NetworkError = 9003,
    /// Timeout error
    TimeoutError = 9004,
    /// Configuration error
    ConfigError = 9005,
    /// Bridge not initialized
    BridgeNotInitialized = 9101,
    /// Bridge not connected
    BridgeNotConnected = 9102,
    /// Bridge connection failed
    BridgeConnectionFailed = 9103,
    /// Printer not available
    PrinterNotAvailable = 9201,
    /// Print failed
    PrintFailed = 9202,
}

impl ErrorCode {
    /// Get the numeric code
    #[inline]
    pub fn code(&self) -> u16 {
        *self as u16
    }

    /// Check if this is a success code
    #[inline]
    pub fn is_success(&self) -> bool {
        *self == Self::Success
    }

    /// Get developer-facing message (English)
    pub fn message(&self) -> &'static str {
        match self {
            // General
            Self::Success => "Success",
            Self::Unknown => "Unknown error",
            Self::ValidationFailed => "Validation failed",
            Self::NotFound => "Resource not found",
            Self::AlreadyExists => "Resource already exists",
            Self::InvalidRequest => "Invalid request",
            Self::InvalidFormat => "Invalid format",
            Self::RequiredField => "Required field missing",
            Self::ValueOutOfRange => "Value out of range",
            // Auth
            Self::NotAuthenticated => "Authentication required",
            Self::InvalidCredentials => "Invalid credentials",
            Self::TokenExpired => "Token expired",
            Self::TokenInvalid => "Invalid token",
            Self::SessionExpired => "Session expired",
            Self::AccountLocked => "Account locked",
            Self::AccountDisabled => "Account disabled",
            // Permission
            Self::PermissionDenied => "Permission denied",
            Self::RoleRequired => "Role required",
            Self::AdminRequired => "Admin required",
            Self::CannotModifyAdmin => "Cannot modify admin",
            Self::CannotDeleteAdmin => "Cannot delete admin",
            // Tenant
            Self::TenantNotSelected => "Tenant not selected",
            Self::TenantNotFound => "Tenant not found",
            Self::ActivationFailed => "Activation failed",
            Self::CertificateInvalid => "Certificate invalid",
            Self::LicenseExpired => "License expired",
            // Order
            Self::OrderNotFound => "Order not found",
            Self::OrderAlreadyPaid => "Order already paid",
            Self::OrderAlreadyCompleted => "Order already completed",
            Self::OrderAlreadyVoided => "Order already voided",
            Self::OrderHasPayments => "Order has payments",
            Self::OrderItemNotFound => "Order item not found",
            Self::OrderEmpty => "Order is empty",
            // Payment
            Self::PaymentFailed => "Payment failed",
            Self::PaymentInsufficientAmount => "Insufficient payment amount",
            Self::PaymentInvalidMethod => "Invalid payment method",
            Self::PaymentAlreadyRefunded => "Payment already refunded",
            Self::PaymentRefundExceedsAmount => "Refund exceeds payment amount",
            // Product
            Self::ProductNotFound => "Product not found",
            Self::ProductInvalidPrice => "Invalid product price",
            Self::ProductOutOfStock => "Product out of stock",
            Self::CategoryNotFound => "Category not found",
            Self::CategoryHasProducts => "Category has products",
            Self::CategoryNameExists => "Category name exists",
            Self::SpecNotFound => "Specification not found",
            Self::AttributeNotFound => "Attribute not found",
            Self::AttributeBindFailed => "Attribute bind failed",
            // Table
            Self::TableNotFound => "Table not found",
            Self::TableOccupied => "Table occupied",
            Self::TableAlreadyEmpty => "Table already empty",
            Self::ZoneNotFound => "Zone not found",
            Self::ZoneHasTables => "Zone has tables",
            Self::ZoneNameExists => "Zone name exists",
            // Employee
            Self::EmployeeNotFound => "Employee not found",
            Self::EmployeeUsernameExists => "Username already exists",
            Self::EmployeeCannotDeleteSelf => "Cannot delete self",
            Self::RoleNotFound => "Role not found",
            Self::RoleNameExists => "Role name exists",
            Self::RoleInUse => "Role in use",
            // System
            Self::InternalError => "Internal server error",
            Self::DatabaseError => "Database error",
            Self::NetworkError => "Network error",
            Self::TimeoutError => "Request timeout",
            Self::ConfigError => "Configuration error",
            Self::BridgeNotInitialized => "Bridge not initialized",
            Self::BridgeNotConnected => "Bridge not connected",
            Self::BridgeConnectionFailed => "Bridge connection failed",
            Self::PrinterNotAvailable => "Printer not available",
            Self::PrintFailed => "Print failed",
        }
    }
}

impl From<ErrorCode> for u16 {
    fn from(code: ErrorCode) -> Self {
        code as u16
    }
}

impl TryFrom<u16> for ErrorCode {
    type Error = u16;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Success),
            1 => Ok(Self::Unknown),
            2 => Ok(Self::ValidationFailed),
            3 => Ok(Self::NotFound),
            4 => Ok(Self::AlreadyExists),
            5 => Ok(Self::InvalidRequest),
            6 => Ok(Self::InvalidFormat),
            7 => Ok(Self::RequiredField),
            8 => Ok(Self::ValueOutOfRange),
            1001 => Ok(Self::NotAuthenticated),
            1002 => Ok(Self::InvalidCredentials),
            1003 => Ok(Self::TokenExpired),
            1004 => Ok(Self::TokenInvalid),
            1005 => Ok(Self::SessionExpired),
            1006 => Ok(Self::AccountLocked),
            1007 => Ok(Self::AccountDisabled),
            2001 => Ok(Self::PermissionDenied),
            2002 => Ok(Self::RoleRequired),
            2003 => Ok(Self::AdminRequired),
            2004 => Ok(Self::CannotModifyAdmin),
            2005 => Ok(Self::CannotDeleteAdmin),
            3001 => Ok(Self::TenantNotSelected),
            3002 => Ok(Self::TenantNotFound),
            3003 => Ok(Self::ActivationFailed),
            3004 => Ok(Self::CertificateInvalid),
            3005 => Ok(Self::LicenseExpired),
            4001 => Ok(Self::OrderNotFound),
            4002 => Ok(Self::OrderAlreadyPaid),
            4003 => Ok(Self::OrderAlreadyCompleted),
            4004 => Ok(Self::OrderAlreadyVoided),
            4005 => Ok(Self::OrderHasPayments),
            4006 => Ok(Self::OrderItemNotFound),
            4007 => Ok(Self::OrderEmpty),
            5001 => Ok(Self::PaymentFailed),
            5002 => Ok(Self::PaymentInsufficientAmount),
            5003 => Ok(Self::PaymentInvalidMethod),
            5004 => Ok(Self::PaymentAlreadyRefunded),
            5005 => Ok(Self::PaymentRefundExceedsAmount),
            6001 => Ok(Self::ProductNotFound),
            6002 => Ok(Self::ProductInvalidPrice),
            6003 => Ok(Self::ProductOutOfStock),
            6101 => Ok(Self::CategoryNotFound),
            6102 => Ok(Self::CategoryHasProducts),
            6103 => Ok(Self::CategoryNameExists),
            6201 => Ok(Self::SpecNotFound),
            6301 => Ok(Self::AttributeNotFound),
            6302 => Ok(Self::AttributeBindFailed),
            7001 => Ok(Self::TableNotFound),
            7002 => Ok(Self::TableOccupied),
            7003 => Ok(Self::TableAlreadyEmpty),
            7101 => Ok(Self::ZoneNotFound),
            7102 => Ok(Self::ZoneHasTables),
            7103 => Ok(Self::ZoneNameExists),
            8001 => Ok(Self::EmployeeNotFound),
            8002 => Ok(Self::EmployeeUsernameExists),
            8003 => Ok(Self::EmployeeCannotDeleteSelf),
            8101 => Ok(Self::RoleNotFound),
            8102 => Ok(Self::RoleNameExists),
            8103 => Ok(Self::RoleInUse),
            9001 => Ok(Self::InternalError),
            9002 => Ok(Self::DatabaseError),
            9003 => Ok(Self::NetworkError),
            9004 => Ok(Self::TimeoutError),
            9005 => Ok(Self::ConfigError),
            9101 => Ok(Self::BridgeNotInitialized),
            9102 => Ok(Self::BridgeNotConnected),
            9103 => Ok(Self::BridgeConnectionFailed),
            9201 => Ok(Self::PrinterNotAvailable),
            9202 => Ok(Self::PrintFailed),
            _ => Err(value),
        }
    }
}

impl std::fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.code())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_code_values() {
        assert_eq!(ErrorCode::Success.code(), 0);
        assert_eq!(ErrorCode::NotAuthenticated.code(), 1001);
        assert_eq!(ErrorCode::OrderNotFound.code(), 4001);
        assert_eq!(ErrorCode::InternalError.code(), 9001);
    }

    #[test]
    fn test_error_code_try_from() {
        assert_eq!(ErrorCode::try_from(0), Ok(ErrorCode::Success));
        assert_eq!(ErrorCode::try_from(1001), Ok(ErrorCode::NotAuthenticated));
        assert_eq!(ErrorCode::try_from(9999), Err(9999));
    }

    #[test]
    fn test_error_code_serialize() {
        let code = ErrorCode::NotAuthenticated;
        let json = serde_json::to_string(&code).unwrap();
        assert_eq!(json, "1001");
    }

    #[test]
    fn test_error_code_deserialize() {
        let code: ErrorCode = serde_json::from_str("4001").unwrap();
        assert_eq!(code, ErrorCode::OrderNotFound);
    }
}
```

**Step 3: Verify compilation**

Run: `cargo check -p shared`
Expected: Compilation succeeds

**Step 4: Run tests**

Run: `cargo test -p shared error::codes`
Expected: All tests pass

**Step 5: Commit**

```bash
git add shared/src/error/codes.rs
git commit -m "feat(shared): add unified error code enum"
```

---

### Task 1.2: Create Error Category Module

**Files:**
- Create: `shared/src/error/category.rs`

**Step 1: Create `shared/src/error/category.rs`**

```rust
//! Error category classification

use super::codes::ErrorCode;
use serde::{Deserialize, Serialize};

/// Error category for grouping related errors
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCategory {
    /// General/Validation errors (0xxx)
    General,
    /// Authentication errors (1xxx)
    Auth,
    /// Authorization errors (2xxx)
    Permission,
    /// Tenant/Activation errors (3xxx)
    Tenant,
    /// Order errors (4xxx)
    Order,
    /// Payment errors (5xxx)
    Payment,
    /// Product/Category/Spec errors (6xxx)
    Product,
    /// Table/Zone errors (7xxx)
    Table,
    /// Employee/Role errors (8xxx)
    Employee,
    /// System/Infrastructure errors (9xxx)
    System,
}

impl ErrorCategory {
    /// Get category from error code
    pub fn from_code(code: u16) -> Self {
        match code {
            0..1000 => Self::General,
            1000..2000 => Self::Auth,
            2000..3000 => Self::Permission,
            3000..4000 => Self::Tenant,
            4000..5000 => Self::Order,
            5000..6000 => Self::Payment,
            6000..7000 => Self::Product,
            7000..8000 => Self::Table,
            8000..9000 => Self::Employee,
            _ => Self::System,
        }
    }

    /// Get category name
    pub fn name(&self) -> &'static str {
        match self {
            Self::General => "general",
            Self::Auth => "auth",
            Self::Permission => "permission",
            Self::Tenant => "tenant",
            Self::Order => "order",
            Self::Payment => "payment",
            Self::Product => "product",
            Self::Table => "table",
            Self::Employee => "employee",
            Self::System => "system",
        }
    }
}

impl ErrorCode {
    /// Get the category for this error code
    pub fn category(&self) -> ErrorCategory {
        ErrorCategory::from_code(self.code())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_category_from_code() {
        assert_eq!(ErrorCategory::from_code(0), ErrorCategory::General);
        assert_eq!(ErrorCategory::from_code(1001), ErrorCategory::Auth);
        assert_eq!(ErrorCategory::from_code(2001), ErrorCategory::Permission);
        assert_eq!(ErrorCategory::from_code(4001), ErrorCategory::Order);
        assert_eq!(ErrorCategory::from_code(9001), ErrorCategory::System);
    }

    #[test]
    fn test_error_code_category() {
        assert_eq!(ErrorCode::Success.category(), ErrorCategory::General);
        assert_eq!(ErrorCode::NotAuthenticated.category(), ErrorCategory::Auth);
        assert_eq!(ErrorCode::PermissionDenied.category(), ErrorCategory::Permission);
        assert_eq!(ErrorCode::OrderNotFound.category(), ErrorCategory::Order);
        assert_eq!(ErrorCode::InternalError.category(), ErrorCategory::System);
    }
}
```

**Step 2: Verify compilation**

Run: `cargo check -p shared`
Expected: Compilation succeeds

**Step 3: Run tests**

Run: `cargo test -p shared error::category`
Expected: All tests pass

**Step 4: Commit**

```bash
git add shared/src/error/category.rs
git commit -m "feat(shared): add error category classification"
```

---

### Task 1.3: Create HTTP Status Mapping

**Files:**
- Create: `shared/src/error/http.rs`

**Step 1: Create `shared/src/error/http.rs`**

```rust
//! HTTP status code mapping for error codes

use super::codes::ErrorCode;
use super::category::ErrorCategory;
use http::StatusCode;

impl ErrorCode {
    /// Get the HTTP status code for this error
    pub fn http_status(&self) -> StatusCode {
        match self {
            // Success
            Self::Success => StatusCode::OK,

            // General - mostly 400, with specific overrides
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::AlreadyExists => StatusCode::CONFLICT,
            Self::Unknown
            | Self::ValidationFailed
            | Self::InvalidRequest
            | Self::InvalidFormat
            | Self::RequiredField
            | Self::ValueOutOfRange => StatusCode::BAD_REQUEST,

            // Auth - 401
            Self::NotAuthenticated
            | Self::InvalidCredentials
            | Self::TokenExpired
            | Self::TokenInvalid
            | Self::SessionExpired
            | Self::AccountLocked
            | Self::AccountDisabled => StatusCode::UNAUTHORIZED,

            // Permission - 403
            Self::PermissionDenied
            | Self::RoleRequired
            | Self::AdminRequired
            | Self::CannotModifyAdmin
            | Self::CannotDeleteAdmin => StatusCode::FORBIDDEN,

            // Tenant - 403
            Self::TenantNotSelected
            | Self::TenantNotFound
            | Self::ActivationFailed
            | Self::CertificateInvalid
            | Self::LicenseExpired => StatusCode::FORBIDDEN,

            // Order - mostly 400, with specific overrides
            Self::OrderNotFound | Self::OrderItemNotFound => StatusCode::NOT_FOUND,
            Self::OrderAlreadyPaid | Self::OrderAlreadyCompleted | Self::OrderAlreadyVoided => {
                StatusCode::CONFLICT
            }
            Self::OrderHasPayments | Self::OrderEmpty => StatusCode::BAD_REQUEST,

            // Payment - mostly 400, with 402 for payment required
            Self::PaymentInsufficientAmount => StatusCode::PAYMENT_REQUIRED,
            Self::PaymentAlreadyRefunded => StatusCode::CONFLICT,
            Self::PaymentFailed
            | Self::PaymentInvalidMethod
            | Self::PaymentRefundExceedsAmount => StatusCode::BAD_REQUEST,

            // Product - mostly 400, with specific overrides
            Self::ProductNotFound
            | Self::CategoryNotFound
            | Self::SpecNotFound
            | Self::AttributeNotFound => StatusCode::NOT_FOUND,
            Self::CategoryNameExists | Self::CategoryHasProducts => StatusCode::CONFLICT,
            Self::ProductInvalidPrice | Self::ProductOutOfStock | Self::AttributeBindFailed => {
                StatusCode::BAD_REQUEST
            }

            // Table - mostly 400, with specific overrides
            Self::TableNotFound | Self::ZoneNotFound => StatusCode::NOT_FOUND,
            Self::ZoneNameExists | Self::ZoneHasTables => StatusCode::CONFLICT,
            Self::TableOccupied | Self::TableAlreadyEmpty => StatusCode::BAD_REQUEST,

            // Employee - mostly 400, with specific overrides
            Self::EmployeeNotFound | Self::RoleNotFound => StatusCode::NOT_FOUND,
            Self::EmployeeUsernameExists | Self::RoleNameExists | Self::RoleInUse => {
                StatusCode::CONFLICT
            }
            Self::EmployeeCannotDeleteSelf => StatusCode::BAD_REQUEST,

            // System - 500
            Self::InternalError
            | Self::DatabaseError
            | Self::NetworkError
            | Self::TimeoutError
            | Self::ConfigError
            | Self::BridgeNotInitialized
            | Self::BridgeNotConnected
            | Self::BridgeConnectionFailed
            | Self::PrinterNotAvailable
            | Self::PrintFailed => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    /// Get default HTTP status for a category (fallback)
    pub fn default_status_for_category(category: ErrorCategory) -> StatusCode {
        match category {
            ErrorCategory::General => StatusCode::BAD_REQUEST,
            ErrorCategory::Auth => StatusCode::UNAUTHORIZED,
            ErrorCategory::Permission => StatusCode::FORBIDDEN,
            ErrorCategory::Tenant => StatusCode::FORBIDDEN,
            ErrorCategory::Order => StatusCode::BAD_REQUEST,
            ErrorCategory::Payment => StatusCode::BAD_REQUEST,
            ErrorCategory::Product => StatusCode::BAD_REQUEST,
            ErrorCategory::Table => StatusCode::BAD_REQUEST,
            ErrorCategory::Employee => StatusCode::BAD_REQUEST,
            ErrorCategory::System => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_http_status_success() {
        assert_eq!(ErrorCode::Success.http_status(), StatusCode::OK);
    }

    #[test]
    fn test_http_status_auth() {
        assert_eq!(ErrorCode::NotAuthenticated.http_status(), StatusCode::UNAUTHORIZED);
        assert_eq!(ErrorCode::TokenExpired.http_status(), StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn test_http_status_permission() {
        assert_eq!(ErrorCode::PermissionDenied.http_status(), StatusCode::FORBIDDEN);
    }

    #[test]
    fn test_http_status_not_found() {
        assert_eq!(ErrorCode::NotFound.http_status(), StatusCode::NOT_FOUND);
        assert_eq!(ErrorCode::OrderNotFound.http_status(), StatusCode::NOT_FOUND);
        assert_eq!(ErrorCode::ProductNotFound.http_status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn test_http_status_conflict() {
        assert_eq!(ErrorCode::AlreadyExists.http_status(), StatusCode::CONFLICT);
        assert_eq!(ErrorCode::OrderAlreadyPaid.http_status(), StatusCode::CONFLICT);
    }

    #[test]
    fn test_http_status_system() {
        assert_eq!(ErrorCode::InternalError.http_status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(ErrorCode::DatabaseError.http_status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn test_http_status_payment() {
        assert_eq!(ErrorCode::PaymentInsufficientAmount.http_status(), StatusCode::PAYMENT_REQUIRED);
    }
}
```

**Step 2: Verify compilation**

Run: `cargo check -p shared`
Expected: Compilation succeeds

**Step 3: Run tests**

Run: `cargo test -p shared error::http`
Expected: All tests pass

**Step 4: Commit**

```bash
git add shared/src/error/http.rs
git commit -m "feat(shared): add HTTP status code mapping"
```

---

### Task 1.4: Create AppError and ApiResponse Types

**Files:**
- Create: `shared/src/error/types.rs`

**Step 1: Create `shared/src/error/types.rs`**

```rust
//! Unified error types and API response structures

use super::codes::ErrorCode;
use http::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use thiserror::Error;

/// Unified application error
#[derive(Debug, Clone, Error)]
#[error("{message}")]
pub struct AppError {
    /// Error code
    pub code: ErrorCode,
    /// Developer message (English)
    pub message: String,
    /// Optional context details for i18n interpolation
    pub details: Option<HashMap<String, Value>>,
}

impl AppError {
    /// Create a new error
    pub fn new(code: ErrorCode) -> Self {
        Self {
            message: code.message().to_string(),
            code,
            details: None,
        }
    }

    /// Create error with custom message
    pub fn with_message(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            details: None,
        }
    }

    /// Add details for i18n interpolation
    pub fn with_details(mut self, details: HashMap<String, Value>) -> Self {
        self.details = Some(details);
        self
    }

    /// Add a single detail
    pub fn with_detail(mut self, key: impl Into<String>, value: impl Into<Value>) -> Self {
        self.details
            .get_or_insert_with(HashMap::new)
            .insert(key.into(), value.into());
        self
    }

    /// Get HTTP status code
    pub fn http_status(&self) -> StatusCode {
        self.code.http_status()
    }

    // ===== Convenience constructors =====

    /// Unknown error
    pub fn unknown(message: impl Into<String>) -> Self {
        Self::with_message(ErrorCode::Unknown, message)
    }

    /// Validation error
    pub fn validation(message: impl Into<String>) -> Self {
        Self::with_message(ErrorCode::ValidationFailed, message)
    }

    /// Not found error
    pub fn not_found(resource: impl Into<String>) -> Self {
        let resource = resource.into();
        Self::with_message(ErrorCode::NotFound, format!("{} not found", resource))
            .with_detail("resource", resource)
    }

    /// Already exists error
    pub fn already_exists(resource: impl Into<String>) -> Self {
        let resource = resource.into();
        Self::with_message(ErrorCode::AlreadyExists, format!("{} already exists", resource))
            .with_detail("resource", resource)
    }

    /// Invalid request error
    pub fn invalid_request(message: impl Into<String>) -> Self {
        Self::with_message(ErrorCode::InvalidRequest, message)
    }

    /// Not authenticated error
    pub fn not_authenticated() -> Self {
        Self::new(ErrorCode::NotAuthenticated)
    }

    /// Invalid credentials error
    pub fn invalid_credentials() -> Self {
        Self::new(ErrorCode::InvalidCredentials)
    }

    /// Token expired error
    pub fn token_expired() -> Self {
        Self::new(ErrorCode::TokenExpired)
    }

    /// Token invalid error
    pub fn token_invalid(message: impl Into<String>) -> Self {
        Self::with_message(ErrorCode::TokenInvalid, message)
    }

    /// Permission denied error
    pub fn permission_denied(message: impl Into<String>) -> Self {
        Self::with_message(ErrorCode::PermissionDenied, message)
    }

    /// Internal error
    pub fn internal(message: impl Into<String>) -> Self {
        Self::with_message(ErrorCode::InternalError, message)
    }

    /// Database error
    pub fn database(message: impl Into<String>) -> Self {
        Self::with_message(ErrorCode::DatabaseError, message)
    }

    /// Order not found
    pub fn order_not_found(order_id: impl Into<String>) -> Self {
        let order_id = order_id.into();
        Self::with_message(ErrorCode::OrderNotFound, format!("Order {} not found", order_id))
            .with_detail("order_id", order_id)
    }

    /// Product not found
    pub fn product_not_found(product_id: impl Into<String>) -> Self {
        let product_id = product_id.into();
        Self::with_message(ErrorCode::ProductNotFound, format!("Product {} not found", product_id))
            .with_detail("product_id", product_id)
    }
}

/// Unified API response structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    /// Error code (0 = success, null = success)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<u16>,
    /// Message
    pub message: String,
    /// Response data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    /// Context details for i18n interpolation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<HashMap<String, Value>>,
}

impl<T> ApiResponse<T> {
    /// Create success response
    pub fn success(data: T) -> Self {
        Self {
            code: Some(0),
            message: "OK".to_string(),
            data: Some(data),
            details: None,
        }
    }

    /// Create success response with message
    pub fn success_with_message(data: T, message: impl Into<String>) -> Self {
        Self {
            code: Some(0),
            message: message.into(),
            data: Some(data),
            details: None,
        }
    }
}

impl<T: Default> Default for ApiResponse<T> {
    fn default() -> Self {
        Self::success(T::default())
    }
}

impl ApiResponse<()> {
    /// Create success response without data
    pub fn ok() -> Self {
        Self {
            code: Some(0),
            message: "OK".to_string(),
            data: None,
            details: None,
        }
    }

    /// Create error response
    pub fn error(err: &AppError) -> Self {
        Self {
            code: Some(err.code.code()),
            message: err.message.clone(),
            data: None,
            details: err.details.clone(),
        }
    }

    /// Create error response from code and message
    pub fn error_with_code(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code: Some(code.code()),
            message: message.into(),
            data: None,
            details: None,
        }
    }
}

impl<T> From<AppError> for ApiResponse<T> {
    fn from(err: AppError) -> Self {
        Self {
            code: Some(err.code.code()),
            message: err.message,
            data: None,
            details: err.details,
        }
    }
}

/// Result type alias
pub type AppResult<T> = Result<T, AppError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_error_new() {
        let err = AppError::new(ErrorCode::NotAuthenticated);
        assert_eq!(err.code, ErrorCode::NotAuthenticated);
        assert_eq!(err.message, "Authentication required");
    }

    #[test]
    fn test_app_error_with_details() {
        let err = AppError::order_not_found("order-123");
        assert_eq!(err.code, ErrorCode::OrderNotFound);
        assert!(err.details.is_some());
        assert_eq!(
            err.details.as_ref().unwrap().get("order_id"),
            Some(&Value::String("order-123".to_string()))
        );
    }

    #[test]
    fn test_api_response_success() {
        let resp = ApiResponse::success("data");
        assert_eq!(resp.code, Some(0));
        assert_eq!(resp.data, Some("data"));
    }

    #[test]
    fn test_api_response_error() {
        let err = AppError::not_found("Order");
        let resp: ApiResponse<()> = err.into();
        assert_eq!(resp.code, Some(3));
        assert!(resp.message.contains("not found"));
    }

    #[test]
    fn test_api_response_serialize() {
        let resp = ApiResponse::success(42);
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"code\":0"));
        assert!(json.contains("\"data\":42"));
    }
}
```

**Step 2: Verify compilation**

Run: `cargo check -p shared`
Expected: Compilation succeeds

**Step 3: Run tests**

Run: `cargo test -p shared error::types`
Expected: All tests pass

**Step 4: Commit**

```bash
git add shared/src/error/types.rs
git commit -m "feat(shared): add AppError and ApiResponse types"
```

---

### Task 1.5: Create Error Module and Export

**Files:**
- Create: `shared/src/error/mod.rs`
- Modify: `shared/src/lib.rs`

**Step 1: Create `shared/src/error/mod.rs`**

```rust
//! Unified error system for Crab framework
//!
//! This module provides:
//! - `ErrorCode`: Numeric error codes (0-9999)
//! - `ErrorCategory`: Category classification
//! - `AppError`: Application error type
//! - `ApiResponse`: Unified API response structure
//!
//! # Error Code Ranges
//!
//! | Range | Category | Description |
//! |-------|----------|-------------|
//! | 0xxx  | General  | Validation, not found, etc |
//! | 1xxx  | Auth     | Authentication errors |
//! | 2xxx  | Permission | Authorization errors |
//! | 3xxx  | Tenant   | Tenant/activation errors |
//! | 4xxx  | Order    | Order-related errors |
//! | 5xxx  | Payment  | Payment-related errors |
//! | 6xxx  | Product  | Product/category/spec errors |
//! | 7xxx  | Table    | Table/zone errors |
//! | 8xxx  | Employee | Employee/role errors |
//! | 9xxx  | System   | System/infrastructure errors |

mod category;
mod codes;
mod http;
mod types;

pub use category::ErrorCategory;
pub use codes::ErrorCode;
pub use types::{ApiResponse, AppError, AppResult};
```

**Step 2: Update `shared/src/lib.rs` to export error module**

Add the following to `shared/src/lib.rs` (after existing module declarations):

```rust
pub mod error;

// Re-export error types for convenience
pub use error::{ApiResponse as UnifiedApiResponse, AppError, AppResult, ErrorCategory, ErrorCode};
```

**Step 3: Verify compilation**

Run: `cargo check -p shared`
Expected: Compilation succeeds

**Step 4: Run all error module tests**

Run: `cargo test -p shared error::`
Expected: All tests pass

**Step 5: Commit**

```bash
git add shared/src/error/mod.rs shared/src/lib.rs
git commit -m "feat(shared): export unified error module"
```

---

### Task 1.6: Add Axum IntoResponse Implementation

**Files:**
- Modify: `shared/src/error/types.rs`

**Step 1: Add axum feature and IntoResponse impl to `shared/src/error/types.rs`**

Add at the end of the file, before tests:

```rust
// ===== Axum Integration =====

impl axum::response::IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        use axum::Json;

        let status = self.http_status();
        let body = ApiResponse::<()>::error(&self);

        // Log internal errors
        if self.code.category() == super::category::ErrorCategory::System {
            tracing::error!(
                code = %self.code,
                message = %self.message,
                "System error occurred"
            );
        }

        (status, Json(body)).into_response()
    }
}

impl<T: Serialize> axum::response::IntoResponse for ApiResponse<T> {
    fn into_response(self) -> axum::response::Response {
        use axum::Json;

        let status = if self.code == Some(0) || self.code.is_none() {
            StatusCode::OK
        } else {
            ErrorCode::try_from(self.code.unwrap_or(1))
                .map(|c| c.http_status())
                .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)
        };

        (status, Json(self)).into_response()
    }
}
```

**Step 2: Add tracing to shared/Cargo.toml dependencies**

Check if tracing is already a workspace dependency. If not, add:

```toml
tracing.workspace = true
```

**Step 3: Verify compilation**

Run: `cargo check -p shared`
Expected: Compilation succeeds

**Step 4: Commit**

```bash
git add shared/src/error/types.rs shared/Cargo.toml
git commit -m "feat(shared): add Axum IntoResponse for AppError"
```

---

### Task 1.7: Create TypeScript Code Generator

**Files:**
- Create: `shared/build.rs`
- Modify: `shared/Cargo.toml`

**Step 1: Create `shared/build.rs`**

```rust
//! Build script to generate TypeScript error codes

use std::env;
use std::fs;
use std::path::Path;

fn main() {
    // Only generate in release mode or when explicitly requested
    if env::var("GENERATE_TS").is_err() && env::var("PROFILE").unwrap_or_default() != "release" {
        return;
    }

    generate_typescript_error_codes();
}

fn generate_typescript_error_codes() {
    let ts_content = r#"// This file is auto-generated by shared/build.rs
// DO NOT EDIT MANUALLY

/**
 * Unified error codes for Crab framework
 *
 * Ranges:
 * - 0xxx: General/Validation
 * - 1xxx: Authentication
 * - 2xxx: Authorization
 * - 3xxx: Tenant/Activation
 * - 4xxx: Order
 * - 5xxx: Payment
 * - 6xxx: Product/Category/Spec
 * - 7xxx: Table/Zone
 * - 8xxx: Employee/Role
 * - 9xxx: System/Infrastructure
 */
export const ErrorCode = {
  // ===== 0xxx: General =====
  Success: 0,
  Unknown: 1,
  ValidationFailed: 2,
  NotFound: 3,
  AlreadyExists: 4,
  InvalidRequest: 5,
  InvalidFormat: 6,
  RequiredField: 7,
  ValueOutOfRange: 8,

  // ===== 1xxx: Authentication =====
  NotAuthenticated: 1001,
  InvalidCredentials: 1002,
  TokenExpired: 1003,
  TokenInvalid: 1004,
  SessionExpired: 1005,
  AccountLocked: 1006,
  AccountDisabled: 1007,

  // ===== 2xxx: Authorization =====
  PermissionDenied: 2001,
  RoleRequired: 2002,
  AdminRequired: 2003,
  CannotModifyAdmin: 2004,
  CannotDeleteAdmin: 2005,

  // ===== 3xxx: Tenant/Activation =====
  TenantNotSelected: 3001,
  TenantNotFound: 3002,
  ActivationFailed: 3003,
  CertificateInvalid: 3004,
  LicenseExpired: 3005,

  // ===== 4xxx: Order =====
  OrderNotFound: 4001,
  OrderAlreadyPaid: 4002,
  OrderAlreadyCompleted: 4003,
  OrderAlreadyVoided: 4004,
  OrderHasPayments: 4005,
  OrderItemNotFound: 4006,
  OrderEmpty: 4007,

  // ===== 5xxx: Payment =====
  PaymentFailed: 5001,
  PaymentInsufficientAmount: 5002,
  PaymentInvalidMethod: 5003,
  PaymentAlreadyRefunded: 5004,
  PaymentRefundExceedsAmount: 5005,

  // ===== 6xxx: Product/Category/Spec =====
  ProductNotFound: 6001,
  ProductInvalidPrice: 6002,
  ProductOutOfStock: 6003,
  CategoryNotFound: 6101,
  CategoryHasProducts: 6102,
  CategoryNameExists: 6103,
  SpecNotFound: 6201,
  AttributeNotFound: 6301,
  AttributeBindFailed: 6302,

  // ===== 7xxx: Table/Zone =====
  TableNotFound: 7001,
  TableOccupied: 7002,
  TableAlreadyEmpty: 7003,
  ZoneNotFound: 7101,
  ZoneHasTables: 7102,
  ZoneNameExists: 7103,

  // ===== 8xxx: Employee/Role =====
  EmployeeNotFound: 8001,
  EmployeeUsernameExists: 8002,
  EmployeeCannotDeleteSelf: 8003,
  RoleNotFound: 8101,
  RoleNameExists: 8102,
  RoleInUse: 8103,

  // ===== 9xxx: System =====
  InternalError: 9001,
  DatabaseError: 9002,
  NetworkError: 9003,
  TimeoutError: 9004,
  ConfigError: 9005,
  BridgeNotInitialized: 9101,
  BridgeNotConnected: 9102,
  BridgeConnectionFailed: 9103,
  PrinterNotAvailable: 9201,
  PrintFailed: 9202,
} as const;

export type ErrorCodeType = (typeof ErrorCode)[keyof typeof ErrorCode];

/**
 * Error categories
 */
export const ErrorCategory = {
  General: 'general',
  Auth: 'auth',
  Permission: 'permission',
  Tenant: 'tenant',
  Order: 'order',
  Payment: 'payment',
  Product: 'product',
  Table: 'table',
  Employee: 'employee',
  System: 'system',
} as const;

export type ErrorCategoryType = (typeof ErrorCategory)[keyof typeof ErrorCategory];

/**
 * Get error category from code
 */
export function getErrorCategory(code: number): ErrorCategoryType {
  if (code < 1000) return ErrorCategory.General;
  if (code < 2000) return ErrorCategory.Auth;
  if (code < 3000) return ErrorCategory.Permission;
  if (code < 4000) return ErrorCategory.Tenant;
  if (code < 5000) return ErrorCategory.Order;
  if (code < 6000) return ErrorCategory.Payment;
  if (code < 7000) return ErrorCategory.Product;
  if (code < 8000) return ErrorCategory.Table;
  if (code < 9000) return ErrorCategory.Employee;
  return ErrorCategory.System;
}

/**
 * Check if code represents success
 */
export function isSuccess(code: number | null | undefined): boolean {
  return code === 0 || code === null || code === undefined;
}

/**
 * Check if code represents an error
 */
export function isError(code: number | null | undefined): boolean {
  return code !== null && code !== undefined && code !== 0;
}

/**
 * API Response type
 */
export interface ApiResponse<T = unknown> {
  code: number | null;
  message: string;
  data: T | null;
  details?: Record<string, unknown> | null;
}

/**
 * Type guard for error response
 */
export function isErrorResponse<T>(response: ApiResponse<T>): boolean {
  return isError(response.code);
}
"#;

    // Determine output path
    let out_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let out_path = Path::new(&out_dir)
        .parent()
        .unwrap()
        .join("red_coral/src/generated/error-codes.ts");

    // Create directory if needed
    if let Some(parent) = out_path.parent() {
        fs::create_dir_all(parent).ok();
    }

    // Write file
    if let Err(e) = fs::write(&out_path, ts_content) {
        eprintln!("Warning: Failed to write TypeScript file: {}", e);
    } else {
        println!("cargo:warning=Generated {}", out_path.display());
    }

    // Tell cargo to rerun if codes.rs changes
    println!("cargo:rerun-if-changed=src/error/codes.rs");
}
```

**Step 2: Update `shared/Cargo.toml` to include build.rs**

Add at the top level (after `[package]`):

```toml
build = "build.rs"
```

**Step 3: Create output directory**

```bash
mkdir -p red_coral/src/generated
```

**Step 4: Test generation**

Run: `GENERATE_TS=1 cargo build -p shared`
Expected: File created at `red_coral/src/generated/error-codes.ts`

**Step 5: Verify generated file**

Run: `head -20 red_coral/src/generated/error-codes.ts`
Expected: See TypeScript header and first few error codes

**Step 6: Commit**

```bash
git add shared/build.rs shared/Cargo.toml red_coral/src/generated/error-codes.ts
git commit -m "feat(shared): add TypeScript error code generator"
```

---

## Phase 2: Edge-Server Migration

### Task 2.1: Update Edge-Server to Use New Error System

**Files:**
- Modify: `edge-server/src/utils/error.rs`
- Modify: `edge-server/Cargo.toml` (if needed)

**Step 1: Replace `edge-server/src/utils/error.rs`**

```rust
//! Edge server error handling
//!
//! This module re-exports the unified error system from shared crate
//! and provides edge-server specific extensions.

// Re-export unified error types
pub use shared::error::{ApiResponse, AppError, AppResult, ErrorCategory, ErrorCode};

// Legacy type aliases for backward compatibility
pub type AppResponse<T> = ApiResponse<T>;

/// Convenience functions for creating JSON responses
pub mod response {
    use super::*;
    use axum::Json;

    /// Create success JSON response
    pub fn ok<T: serde::Serialize>(data: T) -> Json<ApiResponse<T>> {
        Json(ApiResponse::success(data))
    }

    /// Create success JSON response with message
    pub fn ok_with_message<T: serde::Serialize>(
        data: T,
        message: impl Into<String>,
    ) -> Json<ApiResponse<T>> {
        Json(ApiResponse::success_with_message(data, message))
    }
}

pub use response::{ok, ok_with_message};
```

**Step 2: Verify edge-server still compiles**

Run: `cargo check -p edge-server`
Expected: Compilation succeeds (may have warnings about unused imports)

**Step 3: Run edge-server tests**

Run: `cargo test -p edge-server --lib`
Expected: All tests pass

**Step 4: Commit**

```bash
git add edge-server/src/utils/error.rs
git commit -m "refactor(edge-server): migrate to unified error system"
```

---

### Task 2.2: Update Edge-Server API Handlers (Sample)

**Files:**
- Verify existing handlers work with new error types

**Step 1: Check a sample handler compiles correctly**

Run: `cargo check -p edge-server`
Expected: No errors related to AppError usage

**Step 2: Verify the IntoResponse works**

The new AppError already implements IntoResponse via shared crate.

**Step 3: Commit if any fixes needed**

```bash
git add -A
git commit -m "fix(edge-server): update handlers for new error types"
```

---

## Phase 3: Tauri Migration

### Task 3.1: Update Tauri Response to Use Unified Types

**Files:**
- Modify: `red_coral/src-tauri/src/core/response.rs`
- Modify: `red_coral/src-tauri/src/core/mod.rs`

**Step 1: Update `red_coral/src-tauri/src/core/response.rs`**

Replace or update the ApiResponse struct to align with shared:

```rust
//! Tauri API response types
//!
//! Uses the unified error system from shared crate.

use serde::Serialize;
use shared::error::ErrorCode;
use std::collections::HashMap;
use serde_json::Value;

/// Tauri API response (compatible with unified system)
#[derive(Debug, Clone, Serialize)]
pub struct ApiResponse<T: Serialize> {
    /// Error code (null or 0 = success)
    pub code: Option<u16>,
    /// Message
    pub message: String,
    /// Response data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    /// Context details for i18n
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<HashMap<String, Value>>,
}

impl<T: Serialize> ApiResponse<T> {
    /// Create success response
    pub fn success(data: T) -> Self {
        Self {
            code: Some(0),
            message: "success".to_string(),
            data: Some(data),
            details: None,
        }
    }

    /// Create success response with message
    pub fn success_with_message(data: T, message: impl Into<String>) -> Self {
        Self {
            code: Some(0),
            message: message.into(),
            data: Some(data),
            details: None,
        }
    }
}

impl ApiResponse<()> {
    /// Create success response without data
    pub fn ok() -> Self {
        Self {
            code: Some(0),
            message: "success".to_string(),
            data: None,
            details: None,
        }
    }

    /// Create error response with code
    pub fn error(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code: Some(code.code()),
            message: message.into(),
            data: None,
            details: None,
        }
    }

    /// Create error response with code and details
    pub fn error_with_details(
        code: ErrorCode,
        message: impl Into<String>,
        details: HashMap<String, Value>,
    ) -> Self {
        Self {
            code: Some(code.code()),
            message: message.into(),
            data: None,
            details: Some(details),
        }
    }

    /// Create error from legacy string code (for migration)
    #[deprecated(note = "Use error() with ErrorCode instead")]
    pub fn error_legacy(code: impl Into<String>, message: impl Into<String>) -> Self {
        // Try to parse as number, fallback to Unknown
        let code_str = code.into();
        let code_num = code_str.parse::<u16>().unwrap_or(1);
        Self {
            code: Some(code_num),
            message: message.into(),
            data: None,
            details: None,
        }
    }
}

// Re-export ErrorCode for convenience
pub use shared::error::ErrorCode;
```

**Step 2: Update imports in mod.rs if needed**

Ensure `shared` is available in Tauri's Cargo.toml dependencies.

**Step 3: Verify compilation**

Run: `cargo check -p red_coral`
Expected: Compilation succeeds

**Step 4: Commit**

```bash
git add red_coral/src-tauri/src/core/response.rs
git commit -m "refactor(tauri): migrate to unified error system"
```

---

### Task 3.2: Deprecate Old Error Codes

**Files:**
- Modify: `red_coral/src-tauri/src/core/error_codes.rs`

**Step 1: Add deprecation notice to `error_codes.rs`**

Add at the top of the file:

```rust
//! DEPRECATED: Use shared::error::ErrorCode instead
//!
//! This module is kept for backward compatibility during migration.
//! All new code should use `shared::error::ErrorCode`.

#![deprecated(since = "0.2.0", note = "Use shared::error::ErrorCode instead")]
```

**Step 2: Verify compilation**

Run: `cargo check -p red_coral`
Expected: Deprecation warnings but no errors

**Step 3: Commit**

```bash
git add red_coral/src-tauri/src/core/error_codes.rs
git commit -m "deprecate(tauri): mark old error_codes as deprecated"
```

---

## Phase 4: Frontend Migration

### Task 4.1: Update Frontend Error Utilities

**Files:**
- Modify: `red_coral/src/utils/error/index.ts`

**Step 1: Update `red_coral/src/utils/error/index.ts`**

```typescript
/**
 * Unified error handling utilities
 */

import { useTranslation } from 'react-i18next';
import {
  ErrorCode,
  ErrorCategory,
  getErrorCategory,
  isError,
  isSuccess,
  type ApiResponse,
  type ErrorCodeType,
} from '@/generated/error-codes';

// Re-export everything from generated file
export * from '@/generated/error-codes';

/**
 * Hook to get localized error message
 */
export function useErrorMessage() {
  const { t } = useTranslation();

  return (code: number, details?: Record<string, unknown>): string => {
    const key = `errors.${code}`;
    const translated = t(key, details ?? {});

    // If no translation found, return default message
    if (translated === key) {
      return t('errors.1', { code }); // Unknown error fallback
    }

    return translated;
  };
}

/**
 * Extract error message from API response
 */
export function getErrorMessage(response: ApiResponse): string {
  if (isSuccess(response.code)) {
    return response.message;
  }

  // For now, return the message directly
  // In production, use i18n lookup: t(`errors.${response.code}`, response.details)
  return response.message;
}

/**
 * Extract error from unknown error type
 */
export function extractError(error: unknown): string {
  if (error instanceof Error) {
    return error.message;
  }
  if (typeof error === 'string') {
    return error;
  }
  if (error && typeof error === 'object' && 'message' in error) {
    return String((error as { message: unknown }).message);
  }
  return 'Unknown error';
}

/**
 * Create error handler callback
 */
export function createErrorHandler(onError: (message: string) => void) {
  return (error: unknown) => {
    const message = extractError(error);
    onError(message);
  };
}

// Legacy compatibility - map old string codes to new numeric codes
export const LegacyErrorCodes: Record<string, number> = {
  'AUTH_NOT_AUTHENTICATED': ErrorCode.NotAuthenticated,
  'AUTH_INVALID_CREDENTIALS': ErrorCode.InvalidCredentials,
  'AUTH_TOKEN_EXPIRED': ErrorCode.TokenExpired,
  'BRIDGE_NOT_INITIALIZED': ErrorCode.BridgeNotInitialized,
  'BRIDGE_NOT_CONNECTED': ErrorCode.BridgeNotConnected,
  // Add more mappings as needed
};
```

**Step 2: Verify TypeScript compiles**

Run: `cd red_coral && npm run type-check` (or equivalent)
Expected: No type errors

**Step 3: Commit**

```bash
git add red_coral/src/utils/error/index.ts
git commit -m "refactor(frontend): migrate to unified error codes"
```

---

### Task 4.2: Update i18n Error Translations

**Files:**
- Modify: `red_coral/src/infrastructure/i18n/locales/en-US.json`
- Modify: `red_coral/src/infrastructure/i18n/locales/zh-CN.json`

**Step 1: Add error translations to `en-US.json`**

Add/update the `errors` section (merge with existing):

```json
{
  "errors": {
    "0": "Success",
    "1": "Unknown error",
    "2": "Validation failed: {{message}}",
    "3": "{{resource}} not found",
    "4": "{{resource}} already exists",
    "5": "Invalid request",
    "6": "Invalid format",
    "7": "Required field missing: {{field}}",
    "8": "Value out of range",
    "1001": "Please login first",
    "1002": "Invalid username or password",
    "1003": "Session expired, please login again",
    "1004": "Invalid token",
    "1005": "Session expired",
    "1006": "Account locked",
    "1007": "Account disabled",
    "2001": "Permission denied",
    "2002": "Role required",
    "2003": "Admin permission required",
    "2004": "Cannot modify admin",
    "2005": "Cannot delete admin",
    "3001": "Please select a tenant",
    "3002": "Tenant not found",
    "3003": "Activation failed",
    "3004": "Invalid certificate",
    "3005": "License expired",
    "4001": "Order not found",
    "4002": "Order already paid",
    "4003": "Order already completed",
    "4004": "Order already voided",
    "4005": "Order has payment records",
    "4006": "Order item not found",
    "4007": "Order is empty",
    "5001": "Payment failed",
    "5002": "Insufficient payment amount, {{remaining}} more needed",
    "5003": "Invalid payment method",
    "5004": "Payment already refunded",
    "5005": "Refund amount exceeds payment",
    "6001": "Product not found",
    "6002": "Invalid product price",
    "6003": "Product out of stock",
    "6101": "Category not found",
    "6102": "Category has products",
    "6103": "Category name already exists",
    "6201": "Specification not found",
    "6301": "Attribute not found",
    "6302": "Failed to bind attribute",
    "7001": "Table not found",
    "7002": "Table {{table_name}} is occupied",
    "7003": "Table already empty",
    "7101": "Zone not found",
    "7102": "Zone has tables",
    "7103": "Zone name already exists",
    "8001": "Employee not found",
    "8002": "Username {{username}} already exists",
    "8003": "Cannot delete yourself",
    "8101": "Role not found",
    "8102": "Role name already exists",
    "8103": "Role is in use",
    "9001": "Internal server error, please try again",
    "9002": "Database error",
    "9003": "Network error",
    "9004": "Request timeout",
    "9005": "Configuration error",
    "9101": "System not initialized",
    "9102": "Not connected to server",
    "9103": "Failed to connect to server",
    "9201": "Printer not available",
    "9202": "Print failed"
  }
}
```

**Step 2: Add error translations to `zh-CN.json`**

Add/update the `errors` section:

```json
{
  "errors": {
    "0": "成功",
    "1": "未知错误",
    "2": "验证失败：{{message}}",
    "3": "{{resource}}不存在",
    "4": "{{resource}}已存在",
    "5": "无效请求",
    "6": "格式错误",
    "7": "必填字段缺失：{{field}}",
    "8": "值超出范围",
    "1001": "请先登录",
    "1002": "用户名或密码错误",
    "1003": "登录已过期，请重新登录",
    "1004": "无效令牌",
    "1005": "会话已过期",
    "1006": "账户已锁定",
    "1007": "账户已禁用",
    "2001": "无权限执行此操作",
    "2002": "需要特定角色",
    "2003": "需要管理员权限",
    "2004": "不能修改管理员",
    "2005": "不能删除管理员",
    "3001": "请选择租户",
    "3002": "租户不存在",
    "3003": "激活失败",
    "3004": "证书无效",
    "3005": "许可证已过期",
    "4001": "订单不存在",
    "4002": "订单已支付，无法修改",
    "4003": "订单已完成",
    "4004": "订单已作废",
    "4005": "订单有支付记录，无法删除",
    "4006": "订单项不存在",
    "4007": "订单为空",
    "5001": "支付失败",
    "5002": "支付金额不足，还差 {{remaining}}",
    "5003": "无效支付方式",
    "5004": "已退款",
    "5005": "退款金额超过支付金额",
    "6001": "商品不存在",
    "6002": "商品价格无效",
    "6003": "商品缺货",
    "6101": "分类不存在",
    "6102": "分类下有商品，无法删除",
    "6103": "分类名已存在",
    "6201": "规格不存在",
    "6301": "属性不存在",
    "6302": "属性绑定失败",
    "7001": "桌台不存在",
    "7002": "桌台 {{table_name}} 正在使用中",
    "7003": "桌台已空",
    "7101": "区域不存在",
    "7102": "区域下有桌台，无法删除",
    "7103": "区域名已存在",
    "8001": "员工不存在",
    "8002": "用户名 {{username}} 已存在",
    "8003": "不能删除自己",
    "8101": "角色不存在",
    "8102": "角色名已存在",
    "8103": "角色使用中，无法删除",
    "9001": "系统内部错误，请稍后重试",
    "9002": "数据库错误",
    "9003": "网络错误",
    "9004": "请求超时",
    "9005": "配置错误",
    "9101": "系统未初始化",
    "9102": "未连接到服务器",
    "9103": "服务器连接失败",
    "9201": "打印机不可用",
    "9202": "打印失败"
  }
}
```

**Step 3: Verify JSON is valid**

Run: `cd red_coral && node -e "require('./src/infrastructure/i18n/locales/en-US.json'); require('./src/infrastructure/i18n/locales/zh-CN.json'); console.log('JSON valid')"`
Expected: "JSON valid"

**Step 4: Commit**

```bash
git add red_coral/src/infrastructure/i18n/locales/*.json
git commit -m "feat(i18n): add unified error code translations"
```

---

## Phase 5: Final Verification

### Task 5.1: Run Full Test Suite

**Step 1: Run all Rust tests**

Run: `cargo test --workspace --lib`
Expected: All tests pass

**Step 2: Run cargo clippy**

Run: `cargo clippy --workspace -- -D warnings`
Expected: No warnings

**Step 3: Build release**

Run: `cargo build --workspace --release`
Expected: Build succeeds

**Step 4: Verify TypeScript types**

Run: `cd red_coral && npm run type-check`
Expected: No type errors

**Step 5: Final commit**

```bash
git add -A
git commit -m "chore: unified error system implementation complete"
```

---

## Summary

**Files Created:**
- `shared/src/error/mod.rs`
- `shared/src/error/codes.rs`
- `shared/src/error/category.rs`
- `shared/src/error/http.rs`
- `shared/src/error/types.rs`
- `shared/build.rs`
- `red_coral/src/generated/error-codes.ts`

**Files Modified:**
- `shared/src/lib.rs`
- `shared/Cargo.toml`
- `edge-server/src/utils/error.rs`
- `red_coral/src-tauri/src/core/response.rs`
- `red_coral/src-tauri/src/core/error_codes.rs`
- `red_coral/src/utils/error/index.ts`
- `red_coral/src/infrastructure/i18n/locales/en-US.json`
- `red_coral/src/infrastructure/i18n/locales/zh-CN.json`

**Total Tasks:** 12 tasks across 5 phases
**Estimated Commits:** 12-15 commits
