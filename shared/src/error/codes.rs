//! Unified error codes for the Crab framework
//!
//! This module defines all error codes used across edge-server, tauri, and frontend.
//! Error codes are organized by category:
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

use serde::{Deserialize, Serialize};
use std::fmt;

/// Unified error code enum
///
/// All error codes are represented as u16 values for efficient serialization
/// and cross-language compatibility (Rust, TypeScript, etc.)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(into = "u16", try_from = "u16")]
#[repr(u16)]
pub enum ErrorCode {
    // ==================== 0xxx: General ====================
    /// Operation completed successfully
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

    // ==================== 1xxx: Auth ====================
    /// User is not authenticated
    NotAuthenticated = 1001,
    /// Invalid credentials (username/password)
    InvalidCredentials = 1002,
    /// Token has expired
    TokenExpired = 1003,
    /// Token is invalid
    TokenInvalid = 1004,
    /// Session has expired
    SessionExpired = 1005,
    /// Account is locked
    AccountLocked = 1006,
    /// Account is disabled
    AccountDisabled = 1007,

    // ==================== 2xxx: Permission ====================
    /// Permission denied
    PermissionDenied = 2001,
    /// Specific role required
    RoleRequired = 2002,
    /// Admin role required
    AdminRequired = 2003,
    /// Cannot modify admin user
    CannotModifyAdmin = 2004,
    /// Cannot delete admin user
    CannotDeleteAdmin = 2005,

    // ==================== 3xxx: Tenant ====================
    /// Tenant not selected
    TenantNotSelected = 3001,
    /// Tenant not found
    TenantNotFound = 3002,
    /// Activation failed (generic activation process failure)
    ActivationFailed = 3003,
    /// Certificate is invalid
    CertificateInvalid = 3004,
    /// License has expired
    LicenseExpired = 3005,
    /// Device limit reached (quota full, need to replace an existing device)
    DeviceLimitReached = 3007,
    /// Client limit reached (quota full, need to replace an existing client)
    ClientLimitReached = 3008,
    /// Tenant credentials invalid (wrong username/password)
    TenantCredentialsInvalid = 3009,
    /// Feature not available in current subscription plan
    FeatureNotAvailable = 3010,
    /// No active subscription for tenant
    TenantNoSubscription = 3011,
    /// Auth server internal error
    AuthServerError = 3012,
    /// Verification code expired
    VerificationCodeExpired = 3013,
    /// Verification code invalid
    VerificationCodeInvalid = 3014,
    /// Too many verification attempts
    TooManyAttempts = 3015,
    /// Email not verified
    EmailNotVerified = 3016,
    /// Payment setup failed (Stripe)
    PaymentSetupFailed = 3017,
    /// Password too short
    PasswordTooShort = 3018,
    /// P12 certificate required before payment (Verifactu compliance)
    P12Required = 3019,
    /// Hardware ID mismatch with certificate
    DeviceIdMismatch = 3020,
    /// Certificate missing device_id extension
    CertificateMissingDeviceId = 3021,

    // ==================== 4xxx: Order ====================
    /// Order not found
    OrderNotFound = 4001,
    /// Order has already been paid
    OrderAlreadyPaid = 4002,
    /// Order has already been completed
    OrderAlreadyCompleted = 4003,
    /// Order has already been voided
    OrderAlreadyVoided = 4004,
    /// Order has existing payments
    OrderHasPayments = 4005,
    /// Order item not found
    OrderItemNotFound = 4006,
    /// Order is empty
    OrderEmpty = 4007,

    // ==================== 5xxx: Payment ====================
    /// Payment processing failed
    PaymentFailed = 5001,
    /// Insufficient payment amount
    PaymentInsufficientAmount = 5002,
    /// Invalid payment method
    PaymentInvalidMethod = 5003,
    /// Payment has already been refunded
    PaymentAlreadyRefunded = 5004,
    /// Refund amount exceeds payment
    PaymentRefundExceedsAmount = 5005,

    // ==================== 6xxx: Product ====================
    /// Product not found
    ProductNotFound = 6001,
    /// Product has invalid price
    ProductInvalidPrice = 6002,
    /// Product is out of stock
    ProductOutOfStock = 6003,
    /// Category not found
    CategoryNotFound = 6101,
    /// Category has products
    CategoryHasProducts = 6102,
    /// Category name already exists
    CategoryNameExists = 6103,
    /// Specification not found
    SpecNotFound = 6201,
    /// Product external_id already exists
    ProductExternalIdExists = 6202,
    /// Product external_id is required
    ProductExternalIdRequired = 6203,
    /// Product cannot belong to virtual category
    ProductCategoryInvalid = 6204,
    /// Attribute not found
    AttributeNotFound = 6301,
    /// Attribute binding failed
    AttributeBindFailed = 6302,
    /// Attribute is in use by products/categories
    AttributeInUse = 6303,
    /// Attribute binding already inherited from category
    AttributeDuplicateBinding = 6304,
    /// Tag not found
    TagNotFound = 6401,
    /// Tag is in use by products
    TagInUse = 6402,

    // ==================== 65xx: File Upload ====================
    /// File too large
    FileTooLarge = 6501,
    /// Unsupported file format
    UnsupportedFileFormat = 6502,
    /// Invalid/corrupted image file
    InvalidImageFile = 6503,
    /// No file provided in request
    NoFileProvided = 6504,
    /// Empty file provided
    EmptyFile = 6505,
    /// No filename provided
    NoFilename = 6506,
    /// Invalid file extension
    InvalidFileExtension = 6507,
    /// Image processing failed
    ImageProcessingFailed = 6508,
    /// File storage failed
    FileStorageFailed = 6509,

    /// Marketing group not found
    MarketingGroupNotFound = 6601,

    /// Label template not found
    LabelTemplateNotFound = 6701,

    /// Price rule not found
    PriceRuleNotFound = 6801,

    /// Print destination not found
    PrintDestinationNotFound = 6511,
    /// Print destination is in use by categories
    PrintDestinationInUse = 6512,

    // ==================== 7xxx: Table ====================
    /// Table not found
    TableNotFound = 7001,
    /// Table is occupied
    TableOccupied = 7002,
    /// Table is already empty
    TableAlreadyEmpty = 7003,
    /// Zone not found
    ZoneNotFound = 7101,
    /// Zone has tables
    ZoneHasTables = 7102,
    /// Zone name already exists
    ZoneNameExists = 7103,
    /// Table has active orders
    TableHasOrders = 7104,

    /// Shift not found
    ShiftNotFound = 7201,
    /// Daily report not found
    DailyReportNotFound = 7301,

    // ==================== 8xxx: Employee ====================
    /// Employee not found
    EmployeeNotFound = 8001,
    /// Member not found
    MemberNotFound = 8005,
    /// Employee username already exists
    EmployeeUsernameExists = 8002,
    /// Cannot delete self
    EmployeeCannotDeleteSelf = 8003,
    /// Cannot modify/delete system employee
    EmployeeIsSystem = 8004,
    /// Role not found
    RoleNotFound = 8101,
    /// Role name already exists
    RoleNameExists = 8102,
    /// Role is in use
    RoleInUse = 8103,
    /// Cannot modify/delete system role
    RoleIsSystem = 8104,

    // ==================== 9xxx: System ====================
    /// Internal server error
    InternalError = 9001,
    /// Database error
    DatabaseError = 9002,
    /// Network error
    NetworkError = 9003,
    /// Operation timeout
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
    /// Print operation failed
    PrintFailed = 9202,
    /// Client disconnected
    ClientDisconnected = 9301,
    /// Subscription blocked (canceled or unpaid)
    SubscriptionBlocked = 3006,

    // ==================== 94xx: Storage ====================
    /// Storage full (disk space insufficient)
    StorageFull = 9401,
    /// Out of memory
    OutOfMemory = 9402,
    /// Storage corrupted (data file damaged)
    StorageCorrupted = 9403,
    /// System busy (IO error, retry later)
    SystemBusy = 9404,
}

impl ErrorCode {
    /// Get the numeric code value
    #[inline]
    pub const fn code(&self) -> u16 {
        *self as u16
    }

    /// Check if this is a success code
    #[inline]
    pub const fn is_success(&self) -> bool {
        matches!(self, ErrorCode::Success)
    }

    /// Get the developer-facing English message for this error code
    pub const fn message(&self) -> &'static str {
        match self {
            // General
            ErrorCode::Success => "Operation completed successfully",
            ErrorCode::Unknown => "An unknown error occurred",
            ErrorCode::ValidationFailed => "Validation failed",
            ErrorCode::NotFound => "Resource not found",
            ErrorCode::AlreadyExists => "Resource already exists",
            ErrorCode::InvalidRequest => "Invalid request",
            ErrorCode::InvalidFormat => "Invalid format",
            ErrorCode::RequiredField => "Required field is missing",
            ErrorCode::ValueOutOfRange => "Value is out of range",

            // Auth
            ErrorCode::NotAuthenticated => "User is not authenticated",
            ErrorCode::InvalidCredentials => "Invalid username or password",
            ErrorCode::TokenExpired => "Authentication token has expired",
            ErrorCode::TokenInvalid => "Authentication token is invalid",
            ErrorCode::SessionExpired => "Session has expired",
            ErrorCode::AccountLocked => "Account is locked",
            ErrorCode::AccountDisabled => "Account is disabled",

            // Permission
            ErrorCode::PermissionDenied => "Permission denied",
            ErrorCode::RoleRequired => "Specific role is required",
            ErrorCode::AdminRequired => "Administrator role is required",
            ErrorCode::CannotModifyAdmin => "Cannot modify administrator user",
            ErrorCode::CannotDeleteAdmin => "Cannot delete administrator user",

            // Tenant
            ErrorCode::TenantNotSelected => "No tenant selected",
            ErrorCode::TenantNotFound => "Tenant not found",
            ErrorCode::ActivationFailed => "Activation failed",
            ErrorCode::CertificateInvalid => "Certificate is invalid",
            ErrorCode::LicenseExpired => "License has expired",
            ErrorCode::DeviceLimitReached => "Device limit reached",
            ErrorCode::ClientLimitReached => "Client limit reached",
            ErrorCode::TenantCredentialsInvalid => "Invalid tenant username or password",
            ErrorCode::FeatureNotAvailable => "Feature not available in current subscription plan",
            ErrorCode::TenantNoSubscription => "No active subscription",
            ErrorCode::AuthServerError => "Auth server internal error",
            ErrorCode::VerificationCodeExpired => "Verification code has expired",
            ErrorCode::VerificationCodeInvalid => "Invalid verification code",
            ErrorCode::TooManyAttempts => "Too many attempts",
            ErrorCode::EmailNotVerified => "Email not verified",
            ErrorCode::PaymentSetupFailed => "Payment setup failed",
            ErrorCode::PasswordTooShort => "Password must be at least 8 characters",
            ErrorCode::P12Required => "P12 certificate must be uploaded before payment",
            ErrorCode::DeviceIdMismatch => "Hardware ID mismatch with certificate",
            ErrorCode::CertificateMissingDeviceId => "Certificate missing device_id extension",

            // Order
            ErrorCode::OrderNotFound => "Order not found",
            ErrorCode::OrderAlreadyPaid => "Order has already been paid",
            ErrorCode::OrderAlreadyCompleted => "Order has already been completed",
            ErrorCode::OrderAlreadyVoided => "Order has already been voided",
            ErrorCode::OrderHasPayments => "Order has existing payments",
            ErrorCode::OrderItemNotFound => "Order item not found",
            ErrorCode::OrderEmpty => "Order is empty",

            // Payment
            ErrorCode::PaymentFailed => "Payment processing failed",
            ErrorCode::PaymentInsufficientAmount => "Insufficient payment amount",
            ErrorCode::PaymentInvalidMethod => "Invalid payment method",
            ErrorCode::PaymentAlreadyRefunded => "Payment has already been refunded",
            ErrorCode::PaymentRefundExceedsAmount => "Refund amount exceeds original payment",

            // Product
            ErrorCode::ProductNotFound => "Product not found",
            ErrorCode::ProductInvalidPrice => "Product has invalid price",
            ErrorCode::ProductOutOfStock => "Product is out of stock",
            ErrorCode::CategoryNotFound => "Category not found",
            ErrorCode::CategoryHasProducts => "Category has associated products",
            ErrorCode::CategoryNameExists => "Category name already exists",
            ErrorCode::SpecNotFound => "Specification not found",
            ErrorCode::ProductExternalIdExists => "Product external_id already exists",
            ErrorCode::ProductExternalIdRequired => "Product external_id is required",
            ErrorCode::ProductCategoryInvalid => "Product cannot belong to a virtual category",
            ErrorCode::AttributeNotFound => "Attribute not found",
            ErrorCode::AttributeBindFailed => "Failed to bind attribute",
            ErrorCode::AttributeInUse => "Attribute is in use by products/categories",
            ErrorCode::AttributeDuplicateBinding => {
                "Attribute binding already inherited from category"
            }
            ErrorCode::TagNotFound => "Tag not found",
            ErrorCode::TagInUse => "Tag is in use by products",
            ErrorCode::MarketingGroupNotFound => "Marketing group not found",
            ErrorCode::LabelTemplateNotFound => "Label template not found",
            ErrorCode::PriceRuleNotFound => "Price rule not found",

            // File Upload
            ErrorCode::FileTooLarge => "File too large",
            ErrorCode::UnsupportedFileFormat => "Unsupported file format",
            ErrorCode::InvalidImageFile => "Invalid image file",
            ErrorCode::NoFileProvided => "No file provided",
            ErrorCode::EmptyFile => "Empty file provided",
            ErrorCode::NoFilename => "No filename provided",
            ErrorCode::InvalidFileExtension => "Invalid file extension",
            ErrorCode::ImageProcessingFailed => "Image processing failed",
            ErrorCode::FileStorageFailed => "File storage failed",
            ErrorCode::PrintDestinationNotFound => "Print destination not found",
            ErrorCode::PrintDestinationInUse => "Print destination is in use by categories",

            // Table
            ErrorCode::TableNotFound => "Table not found",
            ErrorCode::TableOccupied => "Table is occupied",
            ErrorCode::TableAlreadyEmpty => "Table is already empty",
            ErrorCode::ZoneNotFound => "Zone not found",
            ErrorCode::ZoneHasTables => "Zone has associated tables",
            ErrorCode::ZoneNameExists => "Zone name already exists",
            ErrorCode::TableHasOrders => "Table has active orders",
            ErrorCode::ShiftNotFound => "Shift not found",
            ErrorCode::DailyReportNotFound => "Daily report not found",

            // Employee
            ErrorCode::MemberNotFound => "Member not found",
            ErrorCode::EmployeeNotFound => "Employee not found",
            ErrorCode::EmployeeUsernameExists => "Employee username already exists",
            ErrorCode::EmployeeCannotDeleteSelf => "Cannot delete own account",
            ErrorCode::EmployeeIsSystem => "Cannot modify system employee",
            ErrorCode::RoleNotFound => "Role not found",
            ErrorCode::RoleNameExists => "Role name already exists",
            ErrorCode::RoleInUse => "Role is currently in use",
            ErrorCode::RoleIsSystem => "Cannot modify system role",

            // System
            ErrorCode::InternalError => "Internal server error",
            ErrorCode::DatabaseError => "Database error",
            ErrorCode::NetworkError => "Network error",
            ErrorCode::TimeoutError => "Operation timed out",
            ErrorCode::ConfigError => "Configuration error",
            ErrorCode::BridgeNotInitialized => "Bridge is not initialized",
            ErrorCode::BridgeNotConnected => "Bridge is not connected",
            ErrorCode::BridgeConnectionFailed => "Bridge connection failed",
            ErrorCode::PrinterNotAvailable => "Printer is not available",
            ErrorCode::PrintFailed => "Print operation failed",
            ErrorCode::ClientDisconnected => "Client disconnected",
            ErrorCode::SubscriptionBlocked => "Subscription is blocked",

            // Storage
            ErrorCode::StorageFull => "Storage full (disk space insufficient)",
            ErrorCode::OutOfMemory => "Out of memory",
            ErrorCode::StorageCorrupted => "Storage corrupted (data file damaged)",
            ErrorCode::SystemBusy => "System busy, please retry later",
        }
    }
}

impl From<ErrorCode> for u16 {
    #[inline]
    fn from(code: ErrorCode) -> Self {
        code.code()
    }
}

/// Error when converting from an invalid u16 to ErrorCode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidErrorCode(pub u16);

impl fmt::Display for InvalidErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid error code: {}", self.0)
    }
}

impl std::error::Error for InvalidErrorCode {}

impl TryFrom<u16> for ErrorCode {
    type Error = InvalidErrorCode;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        match value {
            // General
            0 => Ok(ErrorCode::Success),
            1 => Ok(ErrorCode::Unknown),
            2 => Ok(ErrorCode::ValidationFailed),
            3 => Ok(ErrorCode::NotFound),
            4 => Ok(ErrorCode::AlreadyExists),
            5 => Ok(ErrorCode::InvalidRequest),
            6 => Ok(ErrorCode::InvalidFormat),
            7 => Ok(ErrorCode::RequiredField),
            8 => Ok(ErrorCode::ValueOutOfRange),

            // Auth
            1001 => Ok(ErrorCode::NotAuthenticated),
            1002 => Ok(ErrorCode::InvalidCredentials),
            1003 => Ok(ErrorCode::TokenExpired),
            1004 => Ok(ErrorCode::TokenInvalid),
            1005 => Ok(ErrorCode::SessionExpired),
            1006 => Ok(ErrorCode::AccountLocked),
            1007 => Ok(ErrorCode::AccountDisabled),

            // Permission
            2001 => Ok(ErrorCode::PermissionDenied),
            2002 => Ok(ErrorCode::RoleRequired),
            2003 => Ok(ErrorCode::AdminRequired),
            2004 => Ok(ErrorCode::CannotModifyAdmin),
            2005 => Ok(ErrorCode::CannotDeleteAdmin),

            // Tenant
            3001 => Ok(ErrorCode::TenantNotSelected),
            3002 => Ok(ErrorCode::TenantNotFound),
            3003 => Ok(ErrorCode::ActivationFailed),
            3004 => Ok(ErrorCode::CertificateInvalid),
            3005 => Ok(ErrorCode::LicenseExpired),
            3007 => Ok(ErrorCode::DeviceLimitReached),
            3008 => Ok(ErrorCode::ClientLimitReached),
            3009 => Ok(ErrorCode::TenantCredentialsInvalid),
            3010 => Ok(ErrorCode::FeatureNotAvailable),
            3011 => Ok(ErrorCode::TenantNoSubscription),
            3012 => Ok(ErrorCode::AuthServerError),
            3013 => Ok(ErrorCode::VerificationCodeExpired),
            3014 => Ok(ErrorCode::VerificationCodeInvalid),
            3015 => Ok(ErrorCode::TooManyAttempts),
            3016 => Ok(ErrorCode::EmailNotVerified),
            3017 => Ok(ErrorCode::PaymentSetupFailed),
            3018 => Ok(ErrorCode::PasswordTooShort),
            3019 => Ok(ErrorCode::P12Required),
            3020 => Ok(ErrorCode::DeviceIdMismatch),
            3021 => Ok(ErrorCode::CertificateMissingDeviceId),

            // Order
            4001 => Ok(ErrorCode::OrderNotFound),
            4002 => Ok(ErrorCode::OrderAlreadyPaid),
            4003 => Ok(ErrorCode::OrderAlreadyCompleted),
            4004 => Ok(ErrorCode::OrderAlreadyVoided),
            4005 => Ok(ErrorCode::OrderHasPayments),
            4006 => Ok(ErrorCode::OrderItemNotFound),
            4007 => Ok(ErrorCode::OrderEmpty),

            // Payment
            5001 => Ok(ErrorCode::PaymentFailed),
            5002 => Ok(ErrorCode::PaymentInsufficientAmount),
            5003 => Ok(ErrorCode::PaymentInvalidMethod),
            5004 => Ok(ErrorCode::PaymentAlreadyRefunded),
            5005 => Ok(ErrorCode::PaymentRefundExceedsAmount),

            // Product
            6001 => Ok(ErrorCode::ProductNotFound),
            6002 => Ok(ErrorCode::ProductInvalidPrice),
            6003 => Ok(ErrorCode::ProductOutOfStock),
            6101 => Ok(ErrorCode::CategoryNotFound),
            6102 => Ok(ErrorCode::CategoryHasProducts),
            6103 => Ok(ErrorCode::CategoryNameExists),
            6201 => Ok(ErrorCode::SpecNotFound),
            6202 => Ok(ErrorCode::ProductExternalIdExists),
            6203 => Ok(ErrorCode::ProductExternalIdRequired),
            6301 => Ok(ErrorCode::AttributeNotFound),
            6204 => Ok(ErrorCode::ProductCategoryInvalid),
            6302 => Ok(ErrorCode::AttributeBindFailed),
            6303 => Ok(ErrorCode::AttributeInUse),
            6304 => Ok(ErrorCode::AttributeDuplicateBinding),
            6401 => Ok(ErrorCode::TagNotFound),
            6402 => Ok(ErrorCode::TagInUse),
            6511 => Ok(ErrorCode::PrintDestinationNotFound),
            6512 => Ok(ErrorCode::PrintDestinationInUse),
            6601 => Ok(ErrorCode::MarketingGroupNotFound),
            6701 => Ok(ErrorCode::LabelTemplateNotFound),
            6801 => Ok(ErrorCode::PriceRuleNotFound),

            // File Upload
            6501 => Ok(ErrorCode::FileTooLarge),
            6502 => Ok(ErrorCode::UnsupportedFileFormat),
            6503 => Ok(ErrorCode::InvalidImageFile),
            6504 => Ok(ErrorCode::NoFileProvided),
            6505 => Ok(ErrorCode::EmptyFile),
            6506 => Ok(ErrorCode::NoFilename),
            6507 => Ok(ErrorCode::InvalidFileExtension),
            6508 => Ok(ErrorCode::ImageProcessingFailed),
            6509 => Ok(ErrorCode::FileStorageFailed),

            // Table
            7001 => Ok(ErrorCode::TableNotFound),
            7002 => Ok(ErrorCode::TableOccupied),
            7003 => Ok(ErrorCode::TableAlreadyEmpty),
            7101 => Ok(ErrorCode::ZoneNotFound),
            7102 => Ok(ErrorCode::ZoneHasTables),
            7103 => Ok(ErrorCode::ZoneNameExists),
            7104 => Ok(ErrorCode::TableHasOrders),
            7201 => Ok(ErrorCode::ShiftNotFound),
            7301 => Ok(ErrorCode::DailyReportNotFound),

            // Employee
            8005 => Ok(ErrorCode::MemberNotFound),
            8001 => Ok(ErrorCode::EmployeeNotFound),
            8002 => Ok(ErrorCode::EmployeeUsernameExists),
            8003 => Ok(ErrorCode::EmployeeCannotDeleteSelf),
            8004 => Ok(ErrorCode::EmployeeIsSystem),
            8101 => Ok(ErrorCode::RoleNotFound),
            8102 => Ok(ErrorCode::RoleNameExists),
            8103 => Ok(ErrorCode::RoleInUse),
            8104 => Ok(ErrorCode::RoleIsSystem),

            // System
            9001 => Ok(ErrorCode::InternalError),
            9002 => Ok(ErrorCode::DatabaseError),
            9003 => Ok(ErrorCode::NetworkError),
            9004 => Ok(ErrorCode::TimeoutError),
            9005 => Ok(ErrorCode::ConfigError),
            9101 => Ok(ErrorCode::BridgeNotInitialized),
            9102 => Ok(ErrorCode::BridgeNotConnected),
            9103 => Ok(ErrorCode::BridgeConnectionFailed),
            9201 => Ok(ErrorCode::PrinterNotAvailable),
            9202 => Ok(ErrorCode::PrintFailed),
            9301 => Ok(ErrorCode::ClientDisconnected),
            3006 => Ok(ErrorCode::SubscriptionBlocked),

            // Storage
            9401 => Ok(ErrorCode::StorageFull),
            9402 => Ok(ErrorCode::OutOfMemory),
            9403 => Ok(ErrorCode::StorageCorrupted),
            9404 => Ok(ErrorCode::SystemBusy),

            _ => Err(InvalidErrorCode(value)),
        }
    }
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.code())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_code_values() {
        // General
        assert_eq!(ErrorCode::Success.code(), 0);
        assert_eq!(ErrorCode::Unknown.code(), 1);
        assert_eq!(ErrorCode::ValidationFailed.code(), 2);
        assert_eq!(ErrorCode::NotFound.code(), 3);
        assert_eq!(ErrorCode::AlreadyExists.code(), 4);
        assert_eq!(ErrorCode::InvalidRequest.code(), 5);
        assert_eq!(ErrorCode::InvalidFormat.code(), 6);
        assert_eq!(ErrorCode::RequiredField.code(), 7);
        assert_eq!(ErrorCode::ValueOutOfRange.code(), 8);

        // Auth
        assert_eq!(ErrorCode::NotAuthenticated.code(), 1001);
        assert_eq!(ErrorCode::InvalidCredentials.code(), 1002);
        assert_eq!(ErrorCode::TokenExpired.code(), 1003);
        assert_eq!(ErrorCode::TokenInvalid.code(), 1004);
        assert_eq!(ErrorCode::SessionExpired.code(), 1005);
        assert_eq!(ErrorCode::AccountLocked.code(), 1006);
        assert_eq!(ErrorCode::AccountDisabled.code(), 1007);

        // Permission
        assert_eq!(ErrorCode::PermissionDenied.code(), 2001);
        assert_eq!(ErrorCode::RoleRequired.code(), 2002);
        assert_eq!(ErrorCode::AdminRequired.code(), 2003);
        assert_eq!(ErrorCode::CannotModifyAdmin.code(), 2004);
        assert_eq!(ErrorCode::CannotDeleteAdmin.code(), 2005);

        // Tenant
        assert_eq!(ErrorCode::TenantNotSelected.code(), 3001);
        assert_eq!(ErrorCode::TenantNotFound.code(), 3002);
        assert_eq!(ErrorCode::ActivationFailed.code(), 3003);
        assert_eq!(ErrorCode::CertificateInvalid.code(), 3004);
        assert_eq!(ErrorCode::LicenseExpired.code(), 3005);
        assert_eq!(ErrorCode::DeviceLimitReached.code(), 3007);
        assert_eq!(ErrorCode::ClientLimitReached.code(), 3008);
        assert_eq!(ErrorCode::TenantCredentialsInvalid.code(), 3009);
        assert_eq!(ErrorCode::TenantNoSubscription.code(), 3011);
        assert_eq!(ErrorCode::AuthServerError.code(), 3012);

        // Order
        assert_eq!(ErrorCode::OrderNotFound.code(), 4001);
        assert_eq!(ErrorCode::OrderAlreadyPaid.code(), 4002);
        assert_eq!(ErrorCode::OrderAlreadyCompleted.code(), 4003);
        assert_eq!(ErrorCode::OrderAlreadyVoided.code(), 4004);
        assert_eq!(ErrorCode::OrderHasPayments.code(), 4005);
        assert_eq!(ErrorCode::OrderItemNotFound.code(), 4006);
        assert_eq!(ErrorCode::OrderEmpty.code(), 4007);

        // Payment
        assert_eq!(ErrorCode::PaymentFailed.code(), 5001);
        assert_eq!(ErrorCode::PaymentInsufficientAmount.code(), 5002);
        assert_eq!(ErrorCode::PaymentInvalidMethod.code(), 5003);
        assert_eq!(ErrorCode::PaymentAlreadyRefunded.code(), 5004);
        assert_eq!(ErrorCode::PaymentRefundExceedsAmount.code(), 5005);

        // Product
        assert_eq!(ErrorCode::ProductNotFound.code(), 6001);
        assert_eq!(ErrorCode::ProductInvalidPrice.code(), 6002);
        assert_eq!(ErrorCode::ProductOutOfStock.code(), 6003);
        assert_eq!(ErrorCode::CategoryNotFound.code(), 6101);
        assert_eq!(ErrorCode::CategoryHasProducts.code(), 6102);
        assert_eq!(ErrorCode::CategoryNameExists.code(), 6103);
        assert_eq!(ErrorCode::SpecNotFound.code(), 6201);
        assert_eq!(ErrorCode::ProductCategoryInvalid.code(), 6204);
        assert_eq!(ErrorCode::AttributeNotFound.code(), 6301);
        assert_eq!(ErrorCode::AttributeBindFailed.code(), 6302);
        assert_eq!(ErrorCode::AttributeInUse.code(), 6303);
        assert_eq!(ErrorCode::AttributeDuplicateBinding.code(), 6304);
        assert_eq!(ErrorCode::TagNotFound.code(), 6401);
        assert_eq!(ErrorCode::TagInUse.code(), 6402);
        assert_eq!(ErrorCode::PrintDestinationNotFound.code(), 6511);
        assert_eq!(ErrorCode::PrintDestinationInUse.code(), 6512);
        assert_eq!(ErrorCode::MarketingGroupNotFound.code(), 6601);
        assert_eq!(ErrorCode::LabelTemplateNotFound.code(), 6701);
        assert_eq!(ErrorCode::PriceRuleNotFound.code(), 6801);

        // Table
        assert_eq!(ErrorCode::TableNotFound.code(), 7001);
        assert_eq!(ErrorCode::TableOccupied.code(), 7002);
        assert_eq!(ErrorCode::TableAlreadyEmpty.code(), 7003);
        assert_eq!(ErrorCode::ZoneNotFound.code(), 7101);
        assert_eq!(ErrorCode::ZoneHasTables.code(), 7102);
        assert_eq!(ErrorCode::ZoneNameExists.code(), 7103);
        assert_eq!(ErrorCode::TableHasOrders.code(), 7104);
        assert_eq!(ErrorCode::ShiftNotFound.code(), 7201);
        assert_eq!(ErrorCode::DailyReportNotFound.code(), 7301);

        // Employee
        assert_eq!(ErrorCode::MemberNotFound.code(), 8005);
        assert_eq!(ErrorCode::EmployeeNotFound.code(), 8001);
        assert_eq!(ErrorCode::EmployeeUsernameExists.code(), 8002);
        assert_eq!(ErrorCode::EmployeeCannotDeleteSelf.code(), 8003);
        assert_eq!(ErrorCode::EmployeeIsSystem.code(), 8004);
        assert_eq!(ErrorCode::RoleNotFound.code(), 8101);
        assert_eq!(ErrorCode::RoleNameExists.code(), 8102);
        assert_eq!(ErrorCode::RoleInUse.code(), 8103);
        assert_eq!(ErrorCode::RoleIsSystem.code(), 8104);

        // System
        assert_eq!(ErrorCode::InternalError.code(), 9001);
        assert_eq!(ErrorCode::DatabaseError.code(), 9002);
        assert_eq!(ErrorCode::NetworkError.code(), 9003);
        assert_eq!(ErrorCode::TimeoutError.code(), 9004);
        assert_eq!(ErrorCode::ConfigError.code(), 9005);
        assert_eq!(ErrorCode::BridgeNotInitialized.code(), 9101);
        assert_eq!(ErrorCode::BridgeNotConnected.code(), 9102);
        assert_eq!(ErrorCode::BridgeConnectionFailed.code(), 9103);
        assert_eq!(ErrorCode::PrinterNotAvailable.code(), 9201);
        assert_eq!(ErrorCode::PrintFailed.code(), 9202);

        // Storage
        assert_eq!(ErrorCode::StorageFull.code(), 9401);
        assert_eq!(ErrorCode::OutOfMemory.code(), 9402);
        assert_eq!(ErrorCode::StorageCorrupted.code(), 9403);
        assert_eq!(ErrorCode::SystemBusy.code(), 9404);
    }

    #[test]
    fn test_is_success() {
        assert!(ErrorCode::Success.is_success());
        assert!(!ErrorCode::Unknown.is_success());
        assert!(!ErrorCode::NotFound.is_success());
        assert!(!ErrorCode::InternalError.is_success());
    }

    #[test]
    fn test_try_from_valid() {
        assert_eq!(ErrorCode::try_from(0), Ok(ErrorCode::Success));
        assert_eq!(ErrorCode::try_from(1001), Ok(ErrorCode::NotAuthenticated));
        assert_eq!(ErrorCode::try_from(4001), Ok(ErrorCode::OrderNotFound));
        assert_eq!(ErrorCode::try_from(9001), Ok(ErrorCode::InternalError));
        // Storage
        assert_eq!(ErrorCode::try_from(9401), Ok(ErrorCode::StorageFull));
        assert_eq!(ErrorCode::try_from(9402), Ok(ErrorCode::OutOfMemory));
        assert_eq!(ErrorCode::try_from(9403), Ok(ErrorCode::StorageCorrupted));
        assert_eq!(ErrorCode::try_from(9404), Ok(ErrorCode::SystemBusy));
    }

    #[test]
    fn test_try_from_invalid() {
        assert_eq!(ErrorCode::try_from(999), Err(InvalidErrorCode(999)));
        assert_eq!(ErrorCode::try_from(10000), Err(InvalidErrorCode(10000)));
        assert_eq!(ErrorCode::try_from(1234), Err(InvalidErrorCode(1234)));
    }

    #[test]
    fn test_from_error_code_to_u16() {
        let code: u16 = ErrorCode::Success.into();
        assert_eq!(code, 0);

        let code: u16 = ErrorCode::NotAuthenticated.into();
        assert_eq!(code, 1001);

        let code: u16 = ErrorCode::InternalError.into();
        assert_eq!(code, 9001);
    }

    #[test]
    fn test_serialize() {
        let code = ErrorCode::NotFound;
        let json = serde_json::to_string(&code).unwrap();
        assert_eq!(json, "3");

        let code = ErrorCode::OrderNotFound;
        let json = serde_json::to_string(&code).unwrap();
        assert_eq!(json, "4001");

        let code = ErrorCode::Success;
        let json = serde_json::to_string(&code).unwrap();
        assert_eq!(json, "0");
    }

    #[test]
    fn test_deserialize() {
        let code: ErrorCode = serde_json::from_str("0").unwrap();
        assert_eq!(code, ErrorCode::Success);

        let code: ErrorCode = serde_json::from_str("3").unwrap();
        assert_eq!(code, ErrorCode::NotFound);

        let code: ErrorCode = serde_json::from_str("4001").unwrap();
        assert_eq!(code, ErrorCode::OrderNotFound);

        let code: ErrorCode = serde_json::from_str("9001").unwrap();
        assert_eq!(code, ErrorCode::InternalError);
    }

    #[test]
    fn test_deserialize_invalid() {
        let result: Result<ErrorCode, _> = serde_json::from_str("999");
        assert!(result.is_err());

        let result: Result<ErrorCode, _> = serde_json::from_str("10000");
        assert!(result.is_err());
    }

    #[test]
    fn test_display() {
        assert_eq!(format!("{}", ErrorCode::Success), "0");
        assert_eq!(format!("{}", ErrorCode::NotFound), "3");
        assert_eq!(format!("{}", ErrorCode::OrderNotFound), "4001");
        assert_eq!(format!("{}", ErrorCode::InternalError), "9001");
    }

    #[test]
    fn test_message() {
        assert_eq!(
            ErrorCode::Success.message(),
            "Operation completed successfully"
        );
        assert_eq!(ErrorCode::NotFound.message(), "Resource not found");
        assert_eq!(ErrorCode::OrderNotFound.message(), "Order not found");
        assert_eq!(ErrorCode::InternalError.message(), "Internal server error");
    }

    #[test]
    fn test_invalid_error_code_display() {
        let err = InvalidErrorCode(999);
        assert_eq!(format!("{}", err), "invalid error code: 999");
    }

    #[test]
    fn test_roundtrip() {
        // Test that serialization -> deserialization roundtrip works
        let codes = [
            ErrorCode::Success,
            ErrorCode::NotAuthenticated,
            ErrorCode::PermissionDenied,
            ErrorCode::OrderNotFound,
            ErrorCode::InternalError,
        ];

        for code in codes {
            let json = serde_json::to_string(&code).unwrap();
            let parsed: ErrorCode = serde_json::from_str(&json).unwrap();
            assert_eq!(code, parsed);
        }
    }

    #[test]
    fn test_debug() {
        // Test that Debug derive works correctly
        let debug_str = format!("{:?}", ErrorCode::Success);
        assert_eq!(debug_str, "Success");

        let debug_str = format!("{:?}", ErrorCode::OrderNotFound);
        assert_eq!(debug_str, "OrderNotFound");
    }

    #[test]
    fn test_clone_copy() {
        let code = ErrorCode::Success;
        let cloned = code.clone();
        let copied = code;

        assert_eq!(code, cloned);
        assert_eq!(code, copied);
    }

    #[test]
    fn test_hash() {
        use std::collections::HashSet;

        let mut set = HashSet::new();
        set.insert(ErrorCode::Success);
        set.insert(ErrorCode::NotFound);
        set.insert(ErrorCode::Success); // Duplicate

        assert_eq!(set.len(), 2);
        assert!(set.contains(&ErrorCode::Success));
        assert!(set.contains(&ErrorCode::NotFound));
    }
}
