//! JWT Extractor
//!
//! Custom extractor for automatically validating JWT tokens

use axum::{
    extract::FromRequestParts,
    http::request::Parts,
};

use crate::common::AppError;
use crate::security_log;
use crate::server::auth::{CurrentUser, JwtService};
use crate::server::ServerState;

/// JWT Auth Extractor
///
/// Use this extractor in protected handlers to automatically validate JWT
/// and extract CurrentUser
impl FromRequestParts<ServerState> for CurrentUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &ServerState,
    ) -> Result<Self, Self::Rejection> {
        // Check if already extracted (from middleware)
        if let Some(user) = parts.extensions.get::<CurrentUser>() {
            return Ok(user.clone());
        }

        // Extract Authorization header
        let auth_header = parts
            .headers
            .get(http::header::AUTHORIZATION)
            .and_then(|h| h.to_str().ok());

        let token = match auth_header {
            Some(header) => JwtService::extract_from_header(header)
                .ok_or_else(|| AppError::invalid_token("Invalid authorization header"))?,
            None => {
                security_log!(WARN, "auth_missing", uri = ?parts.uri);
                return Err(AppError::Unauthorized);
            }
        };

        // Validate token
        let jwt_service = state.get_jwt_service();
        match jwt_service.validate_token(token) {
            Ok(claims) => {
                let user = CurrentUser::from(claims);

                tracing::info!(
                    user_id = %user.id,
                    username = %user.username,
                    role = %user.role,
                    "User authenticated successfully"
                );

                // Store in extensions for potential reuse
                parts.extensions.insert(user.clone());

                Ok(user)
            }
            Err(e) => {
                security_log!(WARN, "auth_failed", error = %e, uri = ?parts.uri);

                match e {
                    crate::server::auth::JwtError::ExpiredToken => {
                        Err(AppError::TokenExpired)
                    }
                    _ => Err(AppError::invalid_token("Invalid token")),
                }
            }
        }
    }
}
