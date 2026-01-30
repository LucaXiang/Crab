//! HTTP status code mapping for error codes

use super::codes::ErrorCode;
use http::StatusCode;

impl ErrorCode {
    /// Get the appropriate HTTP status code for this error code
    pub fn http_status(&self) -> StatusCode {
        match self {
            // Success
            Self::Success => StatusCode::OK,

            // 404 Not Found
            Self::NotFound
            | Self::OrderNotFound
            | Self::OrderItemNotFound
            | Self::ProductNotFound
            | Self::CategoryNotFound
            | Self::SpecNotFound
            | Self::AttributeNotFound
            | Self::TableNotFound
            | Self::ZoneNotFound
            | Self::EmployeeNotFound
            | Self::RoleNotFound => StatusCode::NOT_FOUND,

            // 409 Conflict
            Self::AlreadyExists
            | Self::OrderAlreadyPaid
            | Self::OrderAlreadyCompleted
            | Self::OrderAlreadyVoided
            | Self::PaymentAlreadyRefunded
            | Self::CategoryNameExists
            | Self::CategoryHasProducts
            | Self::ZoneNameExists
            | Self::ZoneHasTables
            | Self::EmployeeUsernameExists
            | Self::RoleNameExists
            | Self::RoleInUse => StatusCode::CONFLICT,

            // 401 Unauthorized
            Self::NotAuthenticated
            | Self::InvalidCredentials
            | Self::TokenExpired
            | Self::TokenInvalid
            | Self::SessionExpired
            | Self::AccountLocked
            | Self::AccountDisabled => StatusCode::UNAUTHORIZED,

            // 403 Forbidden
            Self::PermissionDenied
            | Self::RoleRequired
            | Self::AdminRequired
            | Self::CannotModifyAdmin
            | Self::CannotDeleteAdmin
            | Self::TenantNotSelected
            | Self::TenantNotFound
            | Self::ActivationFailed
            | Self::CertificateInvalid
            | Self::LicenseExpired => StatusCode::FORBIDDEN,

            // 402 Payment Required
            Self::PaymentInsufficientAmount => StatusCode::PAYMENT_REQUIRED,

            // 503 Service Unavailable (transient errors, client can retry)
            Self::NetworkError
            | Self::TimeoutError => StatusCode::SERVICE_UNAVAILABLE,

            // 500 Internal Server Error
            Self::InternalError
            | Self::DatabaseError
            | Self::ConfigError
            | Self::BridgeNotInitialized
            | Self::BridgeNotConnected
            | Self::BridgeConnectionFailed
            | Self::PrinterNotAvailable
            | Self::PrintFailed
            | Self::ClientDisconnected => StatusCode::INTERNAL_SERVER_ERROR,

            // 400 Bad Request (default for validation/business errors)
            _ => StatusCode::BAD_REQUEST,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_success_status() {
        assert_eq!(ErrorCode::Success.http_status(), StatusCode::OK);
    }

    #[test]
    fn test_not_found_status() {
        assert_eq!(ErrorCode::NotFound.http_status(), StatusCode::NOT_FOUND);
        assert_eq!(
            ErrorCode::OrderNotFound.http_status(),
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            ErrorCode::ProductNotFound.http_status(),
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            ErrorCode::TableNotFound.http_status(),
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            ErrorCode::EmployeeNotFound.http_status(),
            StatusCode::NOT_FOUND
        );
    }

    #[test]
    fn test_conflict_status() {
        assert_eq!(ErrorCode::AlreadyExists.http_status(), StatusCode::CONFLICT);
        assert_eq!(
            ErrorCode::OrderAlreadyPaid.http_status(),
            StatusCode::CONFLICT
        );
        assert_eq!(
            ErrorCode::CategoryNameExists.http_status(),
            StatusCode::CONFLICT
        );
        assert_eq!(ErrorCode::RoleInUse.http_status(), StatusCode::CONFLICT);
    }

    #[test]
    fn test_unauthorized_status() {
        assert_eq!(
            ErrorCode::NotAuthenticated.http_status(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            ErrorCode::InvalidCredentials.http_status(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            ErrorCode::TokenExpired.http_status(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            ErrorCode::TokenInvalid.http_status(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            ErrorCode::AccountLocked.http_status(),
            StatusCode::UNAUTHORIZED
        );
    }

    #[test]
    fn test_forbidden_status() {
        assert_eq!(
            ErrorCode::PermissionDenied.http_status(),
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            ErrorCode::AdminRequired.http_status(),
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            ErrorCode::TenantNotSelected.http_status(),
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            ErrorCode::LicenseExpired.http_status(),
            StatusCode::FORBIDDEN
        );
    }

    #[test]
    fn test_payment_required_status() {
        assert_eq!(
            ErrorCode::PaymentInsufficientAmount.http_status(),
            StatusCode::PAYMENT_REQUIRED
        );
    }

    #[test]
    fn test_internal_error_status() {
        assert_eq!(
            ErrorCode::InternalError.http_status(),
            StatusCode::INTERNAL_SERVER_ERROR
        );
        assert_eq!(
            ErrorCode::DatabaseError.http_status(),
            StatusCode::INTERNAL_SERVER_ERROR
        );
        assert_eq!(
            ErrorCode::BridgeNotInitialized.http_status(),
            StatusCode::INTERNAL_SERVER_ERROR
        );
    }

    #[test]
    fn test_service_unavailable_status() {
        assert_eq!(
            ErrorCode::NetworkError.http_status(),
            StatusCode::SERVICE_UNAVAILABLE
        );
        assert_eq!(
            ErrorCode::TimeoutError.http_status(),
            StatusCode::SERVICE_UNAVAILABLE
        );
    }

    #[test]
    fn test_bad_request_status() {
        // Validation and business rule errors default to 400
        assert_eq!(
            ErrorCode::ValidationFailed.http_status(),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            ErrorCode::InvalidRequest.http_status(),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            ErrorCode::InvalidFormat.http_status(),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(ErrorCode::OrderEmpty.http_status(), StatusCode::BAD_REQUEST);
        assert_eq!(
            ErrorCode::PaymentFailed.http_status(),
            StatusCode::BAD_REQUEST
        );
    }
}
