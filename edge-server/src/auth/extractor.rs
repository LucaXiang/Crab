//! JWT Extractor
//!
//! Custom extractor for automatically validating JWT tokens

use axum::{extract::FromRequestParts, http::request::Parts};

use crate::AppError;
use crate::auth::{CurrentUser, JwtService};
use crate::core::ServerState;
use crate::security_log;

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
                security_log!("WARN", "auth_missing", uri = format!("{:?}", parts.uri));
                return Err(AppError::unauthorized());
            }
        };

        // Validate token
        let jwt_service = state.get_jwt_service();
        match jwt_service.validate_token(token) {
            Ok(claims) => {
                let user = CurrentUser::try_from(claims)
                    .map_err(|e| AppError::invalid_token(format!("Malformed JWT claims: {}", e)))?;

                // Store in extensions for potential reuse
                parts.extensions.insert(user.clone());

                Ok(user)
            }
            Err(e) => {
                security_log!(
                    "WARN",
                    "auth_failed",
                    error = format!("{}", e),
                    uri = format!("{:?}", parts.uri)
                );

                match e {
                    crate::auth::JwtError::ExpiredToken => Err(AppError::token_expired()),
                    _ => Err(AppError::invalid_token("Invalid token")),
                }
            }
        }
    }
}
