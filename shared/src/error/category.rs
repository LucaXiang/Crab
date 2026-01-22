//! Error category classification

use super::codes::ErrorCode;
use serde::{Deserialize, Serialize};

/// Error category classification based on error code ranges
///
/// Categories are determined by the leading digit of the error code:
/// - 0xxx: General errors
/// - 1xxx: Authentication errors
/// - 2xxx: Permission errors
/// - 3xxx: Tenant errors
/// - 4xxx: Order errors
/// - 5xxx: Payment errors
/// - 6xxx: Product errors
/// - 7xxx: Table errors
/// - 8xxx: Employee errors
/// - 9xxx: System errors
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCategory {
    /// General errors (0xxx)
    General,
    /// Authentication errors (1xxx)
    Auth,
    /// Permission errors (2xxx)
    Permission,
    /// Tenant errors (3xxx)
    Tenant,
    /// Order errors (4xxx)
    Order,
    /// Payment errors (5xxx)
    Payment,
    /// Product errors (6xxx)
    Product,
    /// Table errors (7xxx)
    Table,
    /// Employee errors (8xxx)
    Employee,
    /// System errors (9xxx)
    System,
}

impl ErrorCategory {
    /// Determine category from error code value
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

    /// Get the string name for this category
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
        assert_eq!(ErrorCategory::from_code(8), ErrorCategory::General);
        assert_eq!(ErrorCategory::from_code(999), ErrorCategory::General);

        assert_eq!(ErrorCategory::from_code(1001), ErrorCategory::Auth);
        assert_eq!(ErrorCategory::from_code(1999), ErrorCategory::Auth);

        assert_eq!(ErrorCategory::from_code(2001), ErrorCategory::Permission);
        assert_eq!(ErrorCategory::from_code(3001), ErrorCategory::Tenant);
        assert_eq!(ErrorCategory::from_code(4001), ErrorCategory::Order);
        assert_eq!(ErrorCategory::from_code(5001), ErrorCategory::Payment);
        assert_eq!(ErrorCategory::from_code(6001), ErrorCategory::Product);
        assert_eq!(ErrorCategory::from_code(7001), ErrorCategory::Table);
        assert_eq!(ErrorCategory::from_code(8001), ErrorCategory::Employee);
        assert_eq!(ErrorCategory::from_code(9001), ErrorCategory::System);
        assert_eq!(ErrorCategory::from_code(10000), ErrorCategory::System);
    }

    #[test]
    fn test_error_code_category() {
        assert_eq!(ErrorCode::Success.category(), ErrorCategory::General);
        assert_eq!(ErrorCode::NotAuthenticated.category(), ErrorCategory::Auth);
        assert_eq!(
            ErrorCode::PermissionDenied.category(),
            ErrorCategory::Permission
        );
        assert_eq!(ErrorCode::TenantNotFound.category(), ErrorCategory::Tenant);
        assert_eq!(ErrorCode::OrderNotFound.category(), ErrorCategory::Order);
        assert_eq!(ErrorCode::PaymentFailed.category(), ErrorCategory::Payment);
        assert_eq!(
            ErrorCode::ProductNotFound.category(),
            ErrorCategory::Product
        );
        assert_eq!(ErrorCode::TableNotFound.category(), ErrorCategory::Table);
        assert_eq!(
            ErrorCode::EmployeeNotFound.category(),
            ErrorCategory::Employee
        );
        assert_eq!(ErrorCode::InternalError.category(), ErrorCategory::System);
    }

    #[test]
    fn test_category_name() {
        assert_eq!(ErrorCategory::General.name(), "general");
        assert_eq!(ErrorCategory::Auth.name(), "auth");
        assert_eq!(ErrorCategory::Permission.name(), "permission");
        assert_eq!(ErrorCategory::Tenant.name(), "tenant");
        assert_eq!(ErrorCategory::Order.name(), "order");
        assert_eq!(ErrorCategory::Payment.name(), "payment");
        assert_eq!(ErrorCategory::Product.name(), "product");
        assert_eq!(ErrorCategory::Table.name(), "table");
        assert_eq!(ErrorCategory::Employee.name(), "employee");
        assert_eq!(ErrorCategory::System.name(), "system");
    }

    #[test]
    fn test_category_serialize() {
        let category = ErrorCategory::Auth;
        let json = serde_json::to_string(&category).unwrap();
        assert_eq!(json, "\"auth\"");

        let category = ErrorCategory::Permission;
        let json = serde_json::to_string(&category).unwrap();
        assert_eq!(json, "\"permission\"");
    }

    #[test]
    fn test_category_deserialize() {
        let category: ErrorCategory = serde_json::from_str("\"auth\"").unwrap();
        assert_eq!(category, ErrorCategory::Auth);

        let category: ErrorCategory = serde_json::from_str("\"system\"").unwrap();
        assert_eq!(category, ErrorCategory::System);
    }
}
