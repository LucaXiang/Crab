//! Unified service-layer error type for crab-cloud
//!
//! `ServiceError` bridges the gap between DB-layer errors (`sqlx::Error`, `BoxError`)
//! and the API-layer error (`AppError`). It enables `?` propagation without manual
//! `.map_err(|e| { tracing::error!(...); AppError::new(...) })` boilerplate.

use axum::response::IntoResponse;
use shared::error::{AppError, ErrorCode};

type BoxError = Box<dyn std::error::Error + Send + Sync>;

/// Service-layer error — only two variants, keeps things simple.
///
/// - `Db`: Database/infrastructure errors (auto-logged, mapped to InternalError)
/// - `App`: Business-rule errors (transparent pass-through to client)
#[derive(Debug)]
pub enum ServiceError {
    /// Database or infrastructure error (sqlx, AWS SDK, serde, etc.)
    Db(BoxError),
    /// Business-rule error (already an AppError with the correct ErrorCode)
    App(AppError),
}

impl From<sqlx::Error> for ServiceError {
    fn from(e: sqlx::Error) -> Self {
        ServiceError::Db(e.into())
    }
}

impl From<BoxError> for ServiceError {
    fn from(e: BoxError) -> Self {
        ServiceError::Db(e)
    }
}

impl From<AppError> for ServiceError {
    fn from(e: AppError) -> Self {
        ServiceError::App(e)
    }
}

impl From<ServiceError> for AppError {
    fn from(e: ServiceError) -> Self {
        match e {
            ServiceError::App(app_err) => app_err,
            ServiceError::Db(db_err) => {
                tracing::error!(error = %db_err, "Service database error");
                AppError::new(ErrorCode::InternalError)
            }
        }
    }
}

impl IntoResponse for ServiceError {
    fn into_response(self) -> axum::response::Response {
        let app_error: AppError = self.into();
        app_error.into_response()
    }
}

/// Convenience type alias for service-layer results
#[allow(dead_code)]
pub type ServiceResult<T> = Result<T, ServiceError>;
