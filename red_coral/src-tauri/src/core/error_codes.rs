//! DEPRECATED: Use shared::error::ErrorCode instead
//!
//! This module is deprecated since v0.2.0. The unified error system in
//! `shared::error::ErrorCode` provides numeric error codes that work
//! across all components (edge-server, tauri, frontend).
//!
//! Migration guide:
//! - Replace `error_codes::auth::NOT_AUTHENTICATED` with `ErrorCode::NotAuthenticated`
//! - Use `ErrorCode::code()` method to get the numeric code
//!
//! Error Codes - 标准化错误码 (Legacy)
//!
//! 错误码格式: {CATEGORY}_{OPERATION}_{REASON}
//! 前端通过错误码映射到本地化消息

/// 认证相关错误
pub mod auth {
    pub const NOT_AUTHENTICATED: &str = "AUTH_NOT_AUTHENTICATED";
    pub const INVALID_CREDENTIALS: &str = "AUTH_INVALID_CREDENTIALS";
    pub const TOKEN_EXPIRED: &str = "AUTH_TOKEN_EXPIRED";
    pub const TOKEN_INVALID: &str = "AUTH_TOKEN_INVALID";
    pub const SESSION_EXPIRED: &str = "AUTH_SESSION_EXPIRED";
    pub const PERMISSION_DENIED: &str = "AUTH_PERMISSION_DENIED";
    pub const USER_DISABLED: &str = "AUTH_USER_DISABLED";
    pub const USER_NOT_FOUND: &str = "AUTH_USER_NOT_FOUND";
}

/// 桥接层/连接相关错误
pub mod bridge {
    pub const NOT_INITIALIZED: &str = "BRIDGE_NOT_INITIALIZED";
    pub const NOT_CONNECTED: &str = "BRIDGE_NOT_CONNECTED";
    pub const ALREADY_RUNNING: &str = "BRIDGE_ALREADY_RUNNING";
    pub const CONNECTION_FAILED: &str = "BRIDGE_CONNECTION_FAILED";
    pub const CONNECTION_LOST: &str = "BRIDGE_CONNECTION_LOST";
    pub const TIMEOUT: &str = "BRIDGE_TIMEOUT";
}

/// 租户相关错误
pub mod tenant {
    pub const NOT_SELECTED: &str = "TENANT_NOT_SELECTED";
    pub const NOT_FOUND: &str = "TENANT_NOT_FOUND";
    pub const ACTIVATION_REQUIRED: &str = "TENANT_ACTIVATION_REQUIRED";
    pub const ACTIVATION_FAILED: &str = "TENANT_ACTIVATION_FAILED";
    pub const CERTIFICATE_INVALID: &str = "TENANT_CERTIFICATE_INVALID";
    pub const CERTIFICATE_EXPIRED: &str = "TENANT_CERTIFICATE_EXPIRED";
    pub const SUBSCRIPTION_EXPIRED: &str = "TENANT_SUBSCRIPTION_EXPIRED";
    pub const SUBSCRIPTION_INVALID: &str = "TENANT_SUBSCRIPTION_INVALID";
}

/// 服务器相关错误
pub mod server {
    pub const START_FAILED: &str = "SERVER_START_FAILED";
    pub const NOT_RUNNING: &str = "SERVER_NOT_RUNNING";
    pub const INTERNAL_ERROR: &str = "SERVER_INTERNAL_ERROR";
    pub const UNAVAILABLE: &str = "SERVER_UNAVAILABLE";
    pub const DATABASE_ERROR: &str = "SERVER_DATABASE_ERROR";
}

/// 资源 CRUD 错误 - Tags
pub mod tag {
    pub const LIST_FAILED: &str = "TAG_LIST_FAILED";
    pub const GET_FAILED: &str = "TAG_GET_FAILED";
    pub const CREATE_FAILED: &str = "TAG_CREATE_FAILED";
    pub const UPDATE_FAILED: &str = "TAG_UPDATE_FAILED";
    pub const DELETE_FAILED: &str = "TAG_DELETE_FAILED";
    pub const NOT_FOUND: &str = "TAG_NOT_FOUND";
    pub const NAME_EXISTS: &str = "TAG_NAME_EXISTS";
}

/// 资源 CRUD 错误 - Categories
pub mod category {
    pub const LIST_FAILED: &str = "CATEGORY_LIST_FAILED";
    pub const GET_FAILED: &str = "CATEGORY_GET_FAILED";
    pub const CREATE_FAILED: &str = "CATEGORY_CREATE_FAILED";
    pub const UPDATE_FAILED: &str = "CATEGORY_UPDATE_FAILED";
    pub const DELETE_FAILED: &str = "CATEGORY_DELETE_FAILED";
    pub const NOT_FOUND: &str = "CATEGORY_NOT_FOUND";
    pub const NAME_EXISTS: &str = "CATEGORY_NAME_EXISTS";
    pub const HAS_PRODUCTS: &str = "CATEGORY_HAS_PRODUCTS";
}

/// 资源 CRUD 错误 - Products
pub mod product {
    pub const LIST_FAILED: &str = "PRODUCT_LIST_FAILED";
    pub const GET_FAILED: &str = "PRODUCT_GET_FAILED";
    pub const CREATE_FAILED: &str = "PRODUCT_CREATE_FAILED";
    pub const UPDATE_FAILED: &str = "PRODUCT_UPDATE_FAILED";
    pub const DELETE_FAILED: &str = "PRODUCT_DELETE_FAILED";
    pub const NOT_FOUND: &str = "PRODUCT_NOT_FOUND";
    pub const NAME_EXISTS: &str = "PRODUCT_NAME_EXISTS";
    pub const INVALID_PRICE: &str = "PRODUCT_INVALID_PRICE";
    pub const CATEGORY_REQUIRED: &str = "PRODUCT_CATEGORY_REQUIRED";
}

/// 资源 CRUD 错误 - Specifications
pub mod spec {
    pub const LIST_FAILED: &str = "SPEC_LIST_FAILED";
    pub const GET_FAILED: &str = "SPEC_GET_FAILED";
    pub const CREATE_FAILED: &str = "SPEC_CREATE_FAILED";
    pub const UPDATE_FAILED: &str = "SPEC_UPDATE_FAILED";
    pub const DELETE_FAILED: &str = "SPEC_DELETE_FAILED";
    pub const NOT_FOUND: &str = "SPEC_NOT_FOUND";
    pub const CANNOT_DELETE_BASE: &str = "SPEC_CANNOT_DELETE_BASE";
}

/// 资源 CRUD 错误 - Attributes
pub mod attribute {
    pub const LIST_FAILED: &str = "ATTRIBUTE_LIST_FAILED";
    pub const GET_FAILED: &str = "ATTRIBUTE_GET_FAILED";
    pub const CREATE_FAILED: &str = "ATTRIBUTE_CREATE_FAILED";
    pub const UPDATE_FAILED: &str = "ATTRIBUTE_UPDATE_FAILED";
    pub const DELETE_FAILED: &str = "ATTRIBUTE_DELETE_FAILED";
    pub const NOT_FOUND: &str = "ATTRIBUTE_NOT_FOUND";
    pub const BIND_FAILED: &str = "ATTRIBUTE_BIND_FAILED";
    pub const UNBIND_FAILED: &str = "ATTRIBUTE_UNBIND_FAILED";
}

/// 资源 CRUD 错误 - Kitchen Printers
pub mod printer {
    pub const LIST_FAILED: &str = "PRINTER_LIST_FAILED";
    pub const GET_FAILED: &str = "PRINTER_GET_FAILED";
    pub const CREATE_FAILED: &str = "PRINTER_CREATE_FAILED";
    pub const UPDATE_FAILED: &str = "PRINTER_UPDATE_FAILED";
    pub const DELETE_FAILED: &str = "PRINTER_DELETE_FAILED";
    pub const NOT_FOUND: &str = "PRINTER_NOT_FOUND";
    pub const PRINT_FAILED: &str = "PRINTER_PRINT_FAILED";
    pub const NOT_AVAILABLE: &str = "PRINTER_NOT_AVAILABLE";
}

/// 资源 CRUD 错误 - Zones
pub mod zone {
    pub const LIST_FAILED: &str = "ZONE_LIST_FAILED";
    pub const GET_FAILED: &str = "ZONE_GET_FAILED";
    pub const CREATE_FAILED: &str = "ZONE_CREATE_FAILED";
    pub const UPDATE_FAILED: &str = "ZONE_UPDATE_FAILED";
    pub const DELETE_FAILED: &str = "ZONE_DELETE_FAILED";
    pub const NOT_FOUND: &str = "ZONE_NOT_FOUND";
    pub const HAS_TABLES: &str = "ZONE_HAS_TABLES";
}

/// 资源 CRUD 错误 - Tables
pub mod table {
    pub const LIST_FAILED: &str = "TABLE_LIST_FAILED";
    pub const GET_FAILED: &str = "TABLE_GET_FAILED";
    pub const CREATE_FAILED: &str = "TABLE_CREATE_FAILED";
    pub const UPDATE_FAILED: &str = "TABLE_UPDATE_FAILED";
    pub const DELETE_FAILED: &str = "TABLE_DELETE_FAILED";
    pub const NOT_FOUND: &str = "TABLE_NOT_FOUND";
    pub const OCCUPIED: &str = "TABLE_OCCUPIED";
}

/// 资源 CRUD 错误 - Employees/Users
pub mod employee {
    pub const LIST_FAILED: &str = "EMPLOYEE_LIST_FAILED";
    pub const GET_FAILED: &str = "EMPLOYEE_GET_FAILED";
    pub const CREATE_FAILED: &str = "EMPLOYEE_CREATE_FAILED";
    pub const UPDATE_FAILED: &str = "EMPLOYEE_UPDATE_FAILED";
    pub const DELETE_FAILED: &str = "EMPLOYEE_DELETE_FAILED";
    pub const NOT_FOUND: &str = "EMPLOYEE_NOT_FOUND";
    pub const USERNAME_EXISTS: &str = "EMPLOYEE_USERNAME_EXISTS";
    pub const CANNOT_DELETE_SELF: &str = "EMPLOYEE_CANNOT_DELETE_SELF";
    pub const CANNOT_DELETE_ADMIN: &str = "EMPLOYEE_CANNOT_DELETE_ADMIN";
}

/// 资源 CRUD 错误 - Roles
pub mod role {
    pub const LIST_FAILED: &str = "ROLE_LIST_FAILED";
    pub const GET_FAILED: &str = "ROLE_GET_FAILED";
    pub const CREATE_FAILED: &str = "ROLE_CREATE_FAILED";
    pub const UPDATE_FAILED: &str = "ROLE_UPDATE_FAILED";
    pub const DELETE_FAILED: &str = "ROLE_DELETE_FAILED";
    pub const NOT_FOUND: &str = "ROLE_NOT_FOUND";
    pub const CANNOT_DELETE_ADMIN: &str = "ROLE_CANNOT_DELETE_ADMIN";
    pub const CANNOT_MODIFY_ADMIN: &str = "ROLE_CANNOT_MODIFY_ADMIN";
}

/// 订单相关错误
pub mod order {
    pub const CREATE_FAILED: &str = "ORDER_CREATE_FAILED";
    pub const UPDATE_FAILED: &str = "ORDER_UPDATE_FAILED";
    pub const NOT_FOUND: &str = "ORDER_NOT_FOUND";
    pub const ALREADY_COMPLETED: &str = "ORDER_ALREADY_COMPLETED";
    pub const ALREADY_VOIDED: &str = "ORDER_ALREADY_VOIDED";
    pub const VOID_FAILED: &str = "ORDER_VOID_FAILED";
    pub const RESTORE_FAILED: &str = "ORDER_RESTORE_FAILED";
    pub const MERGE_FAILED: &str = "ORDER_MERGE_FAILED";
    pub const MOVE_FAILED: &str = "ORDER_MOVE_FAILED";
    pub const HAS_PAYMENTS: &str = "ORDER_HAS_PAYMENTS";
}

/// 支付相关错误
pub mod payment {
    pub const FAILED: &str = "PAYMENT_FAILED";
    pub const INSUFFICIENT_AMOUNT: &str = "PAYMENT_INSUFFICIENT_AMOUNT";
    pub const INVALID_METHOD: &str = "PAYMENT_INVALID_METHOD";
    pub const CANCEL_FAILED: &str = "PAYMENT_CANCEL_FAILED";
    pub const REFUND_FAILED: &str = "PAYMENT_REFUND_FAILED";
}

/// 验证错误
pub mod validation {
    pub const REQUIRED_FIELD: &str = "VALIDATION_REQUIRED_FIELD";
    pub const INVALID_FORMAT: &str = "VALIDATION_INVALID_FORMAT";
    pub const VALUE_TOO_LONG: &str = "VALIDATION_VALUE_TOO_LONG";
    pub const VALUE_TOO_SHORT: &str = "VALIDATION_VALUE_TOO_SHORT";
    pub const VALUE_OUT_OF_RANGE: &str = "VALIDATION_VALUE_OUT_OF_RANGE";
    pub const INVALID_ID: &str = "VALIDATION_INVALID_ID";
}

/// 通用错误
pub mod general {
    pub const UNKNOWN: &str = "UNKNOWN_ERROR";
    pub const NOT_IMPLEMENTED: &str = "NOT_IMPLEMENTED";
    pub const NETWORK_ERROR: &str = "NETWORK_ERROR";
    pub const IO_ERROR: &str = "IO_ERROR";
    pub const PARSE_ERROR: &str = "PARSE_ERROR";
}
