//! HTTP status code mapping for error codes

use super::codes::ErrorCode;
use http::StatusCode;

impl ErrorCode {
    /// Get the appropriate HTTP status code for this error code.
    ///
    /// Every variant has an explicit mapping — no wildcard fallthrough.
    /// Adding a new ErrorCode variant without mapping here will cause a compile error.
    pub fn http_status(&self) -> StatusCode {
        match self {
            // ==================== 200 OK ====================
            Self::Success => StatusCode::OK,

            // ==================== 400 Bad Request ====================
            // Client sent a malformed or invalid request
            Self::ValidationFailed
            | Self::InvalidRequest
            | Self::InvalidFormat
            | Self::RequiredField
            | Self::PasswordTooShort
            | Self::P12Required
            | Self::OrderNotCompleted
            | Self::CreditNoteOverRefund
            | Self::CreditNoteItemOverRefund
            | Self::ProductInvalidPrice
            | Self::ProductExternalIdRequired
            | Self::ProductCategoryInvalid
            | Self::SpecRootRequired
            | Self::AttributeBindFailed
            | Self::PriceRuleValueOutOfRange => StatusCode::BAD_REQUEST,

            // ==================== 401 Unauthorized ====================
            // Authentication missing or invalid
            Self::NotAuthenticated
            | Self::InvalidCredentials
            | Self::TenantCredentialsInvalid
            | Self::TokenExpired
            | Self::SessionExpired
            | Self::AccountDisabled
            | Self::VerificationCodeInvalid => StatusCode::UNAUTHORIZED,

            // ==================== 403 Forbidden ====================
            // Authenticated but not allowed
            Self::PermissionDenied
            | Self::AdminRequired
            | Self::TenantNotSelected
            | Self::TenantNotFound
            | Self::ActivationFailed
            | Self::CertificateInvalid
            | Self::LicenseExpired
            | Self::StoreLimitReached
            | Self::ResourceLimitExceeded
            | Self::TenantNoSubscription
            | Self::SubscriptionBlocked
            | Self::EmployeeIsSystem
            | Self::RoleIsSystem => StatusCode::FORBIDDEN,

            // ==================== 404 Not Found ====================
            Self::NotFound
            | Self::OrderNotFound
            | Self::OrderItemNotFound
            | Self::ProductNotFound
            | Self::CategoryNotFound
            | Self::AttributeNotFound
            | Self::TableNotFound
            | Self::ZoneNotFound
            | Self::EmployeeNotFound
            | Self::RoleNotFound
            | Self::TagNotFound
            | Self::MarketingGroupNotFound
            | Self::PrintDestinationNotFound
            | Self::LabelTemplateNotFound
            | Self::PriceRuleNotFound
            | Self::ShiftNotFound
            | Self::DailyReportNotFound
            | Self::MemberNotFound => StatusCode::NOT_FOUND,

            // ==================== 409 Conflict ====================
            // Request conflicts with current resource state
            Self::AlreadyExists
            | Self::OrderAlreadyCompleted
            | Self::OrderAlreadyVoided
            | Self::OrderHasCreditNotes
            | Self::OrderAlreadyUpgraded
            | Self::OrderVoidedNoCreditNote
            | Self::ImportBlockedActiveOrders
            | Self::ProductExternalIdExists
            | Self::CategoryHasProducts
            | Self::ZoneHasTables
            | Self::AttributeInUse
            | Self::AttributeDuplicateBinding
            | Self::TagInUse
            | Self::PrintDestinationInUse
            | Self::TableOccupied
            | Self::TableHasOrders => StatusCode::CONFLICT,

            // ==================== 410 Gone ====================
            Self::VerificationCodeExpired => StatusCode::GONE,

            // ==================== 422 Unprocessable Entity ====================
            // Request well-formed but semantically invalid
            Self::P12InvalidFormat
            | Self::P12WrongPassword
            | Self::P12MissingPrivateKey
            | Self::P12MissingCertificate
            | Self::P12ChainVerifyFailed
            | Self::P12UntrustedCa
            | Self::P12NifMismatch
            | Self::P12CertExpired
            | Self::P12CertNotYetValid
            | Self::ImportInvalidFormat => StatusCode::UNPROCESSABLE_ENTITY,

            // ==================== 429 Too Many Requests ====================
            Self::TooManyAttempts => StatusCode::TOO_MANY_REQUESTS,

            // ==================== 500 Internal Server Error ====================
            // Server-side failures
            Self::Unknown
            | Self::InternalError
            | Self::DatabaseError
            | Self::ConfigError
            | Self::AuthServerError
            | Self::BridgeNotInitialized
            | Self::BridgeNotConnected
            | Self::BridgeConnectionFailed
            | Self::PrinterNotAvailable
            | Self::PrintFailed
            | Self::PrintNoPrintersConfigured
            | Self::PrintAllPrintersOffline
            | Self::ClientDisconnected
            | Self::StorageCorrupted
            | Self::ArchiveHashChainError
            | Self::InvoiceNumberError
            | Self::InvoiceConversionError
            | Self::ExportFailed
            | Self::PasswordHashingFailed => StatusCode::INTERNAL_SERVER_ERROR,

            // ==================== 502 Bad Gateway ====================
            Self::PaymentSetupFailed => StatusCode::BAD_GATEWAY,

            // ==================== 503 Service Unavailable ====================
            // Transient errors, client can retry
            Self::NetworkError
            | Self::TimeoutError
            | Self::StorageFull
            | Self::OutOfMemory
            | Self::SystemBusy => StatusCode::SERVICE_UNAVAILABLE,
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
    fn test_bad_request_status() {
        let cases = [
            ErrorCode::ValidationFailed,
            ErrorCode::InvalidRequest,
            ErrorCode::InvalidFormat,
            ErrorCode::RequiredField,
            ErrorCode::PasswordTooShort,
            ErrorCode::P12Required,
            ErrorCode::ProductInvalidPrice,
            ErrorCode::ProductExternalIdRequired,
            ErrorCode::ProductCategoryInvalid,
            ErrorCode::AttributeBindFailed,
        ];
        for code in cases {
            assert_eq!(code.http_status(), StatusCode::BAD_REQUEST, "{code:?}");
        }
    }

    #[test]
    fn test_unauthorized_status() {
        let cases = [
            ErrorCode::NotAuthenticated,
            ErrorCode::InvalidCredentials,
            ErrorCode::TenantCredentialsInvalid,
            ErrorCode::TokenExpired,
            ErrorCode::SessionExpired,
            ErrorCode::AccountDisabled,
            ErrorCode::VerificationCodeInvalid,
        ];
        for code in cases {
            assert_eq!(code.http_status(), StatusCode::UNAUTHORIZED, "{code:?}");
        }
    }

    #[test]
    fn test_forbidden_status() {
        let cases = [
            ErrorCode::PermissionDenied,
            ErrorCode::AdminRequired,
            ErrorCode::TenantNotSelected,
            ErrorCode::LicenseExpired,
            ErrorCode::EmployeeIsSystem,
            ErrorCode::RoleIsSystem,
        ];
        for code in cases {
            assert_eq!(code.http_status(), StatusCode::FORBIDDEN, "{code:?}");
        }
    }

    #[test]
    fn test_not_found_status() {
        let cases = [
            ErrorCode::NotFound,
            ErrorCode::OrderNotFound,
            ErrorCode::ProductNotFound,
            ErrorCode::TableNotFound,
            ErrorCode::EmployeeNotFound,
            ErrorCode::ShiftNotFound,
            ErrorCode::DailyReportNotFound,
            ErrorCode::MemberNotFound,
        ];
        for code in cases {
            assert_eq!(code.http_status(), StatusCode::NOT_FOUND, "{code:?}");
        }
    }

    #[test]
    fn test_conflict_status() {
        let cases = [
            ErrorCode::AlreadyExists,
            ErrorCode::ProductExternalIdExists,
            ErrorCode::TableOccupied,
        ];
        for code in cases {
            assert_eq!(code.http_status(), StatusCode::CONFLICT, "{code:?}");
        }
    }

    #[test]
    fn test_unprocessable_entity_status() {
        let cases = [
            ErrorCode::P12InvalidFormat,
            ErrorCode::P12WrongPassword,
            ErrorCode::P12MissingPrivateKey,
            ErrorCode::P12MissingCertificate,
            ErrorCode::P12ChainVerifyFailed,
            ErrorCode::P12UntrustedCa,
        ];
        for code in cases {
            assert_eq!(
                code.http_status(),
                StatusCode::UNPROCESSABLE_ENTITY,
                "{code:?}"
            );
        }
    }

    #[test]
    fn test_internal_error_status() {
        let cases = [
            ErrorCode::Unknown,
            ErrorCode::InternalError,
            ErrorCode::DatabaseError,
            ErrorCode::BridgeNotInitialized,
            ErrorCode::StorageCorrupted,
        ];
        for code in cases {
            assert_eq!(
                code.http_status(),
                StatusCode::INTERNAL_SERVER_ERROR,
                "{code:?}"
            );
        }
    }

    #[test]
    fn test_service_unavailable_status() {
        let cases = [
            ErrorCode::NetworkError,
            ErrorCode::TimeoutError,
            ErrorCode::StorageFull,
            ErrorCode::OutOfMemory,
            ErrorCode::SystemBusy,
        ];
        for code in cases {
            assert_eq!(
                code.http_status(),
                StatusCode::SERVICE_UNAVAILABLE,
                "{code:?}"
            );
        }
    }

    #[test]
    fn test_special_status_codes() {
        assert_eq!(
            ErrorCode::VerificationCodeExpired.http_status(),
            StatusCode::GONE
        );
        assert_eq!(
            ErrorCode::TooManyAttempts.http_status(),
            StatusCode::TOO_MANY_REQUESTS
        );
        assert_eq!(
            ErrorCode::PaymentSetupFailed.http_status(),
            StatusCode::BAD_GATEWAY
        );
    }
}
