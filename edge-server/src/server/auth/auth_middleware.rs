//! Authentication Middleware
//!
//! Provides Axum middleware for JWT authentication and authorization

use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};

use crate::common::AppError;
use crate::security_log;
use crate::server::ServerState;
use crate::server::auth::{CurrentUser, JwtService};

/// Require authentication middleware
///
/// This middleware extracts and validates the JWT token from the Authorization header.
/// If valid, it adds the CurrentUser to the request extensions.
pub async fn require_auth(
    State(state): State<ServerState>,
    mut req: Request,
    next: Next,
) -> Result<Response, AppError> {
    let path = req.uri().path();

    // Allow OPTIONS requests for CORS preflight (skip auth)
    if req.method() == http::Method::OPTIONS {
        tracing::info!("[require_auth] OPTIONS request, skipping");
        return Ok(next.run(req).await);
    }

    // Skip auth for non-API routes (let them return 404 normally)
    if !path.starts_with("/api/") {
        return Ok(next.run(req).await);
    }

    // Skip auth for public API routes
    let is_public_api_route = path == "/api/auth/login" || path == "/api/message/emit";
    if is_public_api_route {
        tracing::info!("[require_auth] Public API route, skipping auth: {}", path);
        return Ok(next.run(req).await);
    }

    let jwt_service = state.get_jwt_service();
    let auth_header = req
        .headers()
        .get(http::header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok());

    let token = match auth_header {
        Some(header) => JwtService::extract_from_header(header).ok_or(AppError::InvalidToken)?,
        None => {
            security_log!(WARN, "auth_missing", uri = ?req.uri());
            return Err(AppError::Unauthorized);
        }
    };

    // Validate token
    match jwt_service.validate_token(token) {
        Ok(claims) => {
            let user = CurrentUser::from(claims);

            tracing::info!(
                user_id = %user.id,
                username = %user.username,
                role = %user.role,
                "User authenticated successfully"
            );

            req.extensions_mut().insert(user);
            Ok(next.run(req).await)
        }
        Err(e) => {
            security_log!(WARN, "auth_failed", error = %e, uri = ?req.uri());

            match e {
                crate::server::auth::JwtError::ExpiredToken => Err(AppError::TokenExpired),
                _ => Err(AppError::InvalidToken),
            }
        }
    }
}

/// Require specific permission middleware
pub async fn require_permission(
    permission: &'static str,
) -> impl Fn(
    Request,
    Next,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Response, AppError>> + Send>>
+ Clone {
    move |req: Request, next: Next| {
        Box::pin(async move {
            let user = req
                .extensions()
                .get::<CurrentUser>()
                .ok_or(AppError::Unauthorized)?;

            if !user.has_permission(permission) {
                security_log!(
                    WARN,
                    "permission_denied",
                    user_id = %user.id,
                    username = %user.username,
                    required_permission = permission
                );
                return Err(AppError::Forbidden(format!(
                    "Permission denied: {}",
                    permission
                )));
            }

            Ok(next.run(req).await)
        })
    }
}

/// Require admin role middleware
pub async fn require_admin(req: Request, next: Next) -> Result<Response, AppError> {
    tracing::info!("[require_admin] Called for path: {}", req.uri().path());
    let user = req
        .extensions()
        .get::<CurrentUser>()
        .ok_or(AppError::Unauthorized)?;
    if !user.is_admin() {
        security_log!(
            WARN,
            "admin_required",
            user_id = %user.id,
            username = %user.username,
            user_role = %user.role
        );
        return Err(AppError::Forbidden("Admin access required".to_string()));
    }

    Ok(next.run(req).await)
}

/// Extension trait to get CurrentUser from request
pub trait CurrentUserExt {
    fn current_user(&self) -> Result<&CurrentUser, AppError>;
}

impl CurrentUserExt for Request {
    fn current_user(&self) -> Result<&CurrentUser, AppError> {
        self.extensions()
            .get::<CurrentUser>()
            .ok_or(AppError::Unauthorized)
    }
}
