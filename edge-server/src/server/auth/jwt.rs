//! JWT Token Service
//!
//! Handles JWT token generation, validation, and parsing.

use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use jsonwebtoken::errors::ErrorKind;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use chrono::{Duration, Utc};

/// JWT Configuration
#[derive(Debug, Clone)]
pub struct JwtConfig {
    /// JWT secret key (should be at least 32 bytes)
    pub secret: String,
    /// Token expiration time in minutes
    pub expiration_minutes: i64,
    /// Token issuer
    pub issuer: String,
    /// Token audience
    pub audience: String,
}

impl Default for JwtConfig {
    fn default() -> Self {
        let secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| {
            #[cfg(debug_assertions)]
            {
                tracing::warn!("⚠️  JWT_SECRET not set! Using insecure default key. DO NOT USE IN PRODUCTION!");
                "dev-secret-key-change-in-production-min-32-chars-long".to_string()
            }
            #[cfg(not(debug_assertions))]
            {
                panic!("🚨 FATAL: JWT_SECRET environment variable is not set!");
            }
        });
        
        Self {
            secret,
            expiration_minutes: std::env::var("JWT_EXPIRATION_MINUTES")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(1440), // 24 hours default
            issuer: std::env::var("JWT_ISSUER")
                .unwrap_or_else(|_| "edge-server".to_string()),
            audience: std::env::var("JWT_AUDIENCE")
                .unwrap_or_else(|_| "edge-clients".to_string()),
        }
    }
}

/// JWT Claims stored in the token
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// User ID
    pub sub: String,
    /// Username
    pub username: String,
    /// Role name
    pub role: String,
    /// Permissions (comma-separated)
    pub permissions: String,
    /// Token type
    pub token_type: String,
    /// Expiration timestamp
    pub exp: i64,
    /// Issued at timestamp
    pub iat: i64,
    /// Issuer
    pub iss: String,
    /// Audience
    pub aud: String,
}

/// JWT Errors
#[derive(Error, Debug)]
pub enum JwtError {
    #[error("Invalid token: {0}")]
    InvalidToken(String),

    #[error("Token expired")]
    ExpiredToken,

    #[error("Invalid signature")]
    InvalidSignature,

    #[error("Token generation failed: {0}")]
    GenerationFailed(String),
}

/// JWT Token Service
#[derive(Debug, Clone)]
pub struct JwtService {
    pub config: JwtConfig,
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
}

impl JwtService {
    /// Create a new JWT service with default config
    pub fn new() -> Self {
        Self::with_config(JwtConfig::default())
    }

    /// Create a new JWT service with custom config
    pub fn with_config(config: JwtConfig) -> Self {
        Self {
            encoding_key: EncodingKey::from_secret(config.secret.as_bytes()),
            decoding_key: DecodingKey::from_secret(config.secret.as_bytes()),
            config,
        }
    }

    /// Generate a JWT token
    pub fn generate_token(
        &self,
        user_id: impl Into<String>,
        username: impl Into<String>,
        role: impl Into<String>,
        permissions: &[String],
    ) -> Result<String, JwtError> {
        let now = Utc::now();
        let expiration = now + Duration::minutes(self.config.expiration_minutes);

        let permissions_str = permissions.join(",");

        let claims = Claims {
            sub: user_id.into(),
            username: username.into(),
            role: role.into(),
            permissions: permissions_str,
            token_type: "access".to_string(),
            exp: expiration.timestamp(),
            iat: now.timestamp(),
            iss: self.config.issuer.clone(),
            aud: self.config.audience.clone(),
        };

        encode(&Header::default(), &claims, &self.encoding_key)
            .map_err(|e| JwtError::GenerationFailed(e.to_string()))
    }

    /// Validate and decode a JWT token
    pub fn validate_token(&self, token: &str) -> Result<Claims, JwtError> {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.set_issuer(&[&self.config.issuer]);
        validation.set_audience(&[&self.config.audience]);

        let token_data = decode::<Claims>(token, &self.decoding_key, &validation)
            .map_err(|e| match e.kind() {
                ErrorKind::ExpiredSignature => JwtError::ExpiredToken,
                ErrorKind::InvalidSignature => JwtError::InvalidSignature,
                _ => JwtError::InvalidToken(e.to_string()),
            })?;

        Ok(token_data.claims)
    }

    /// Extract token from Authorization header
    pub fn extract_from_header(header: &str) -> Option<&str> {
        header.strip_prefix("Bearer ")
    }

    /// Get remaining time until expiration in seconds
    pub fn get_expiration_seconds(&self, claims: &Claims) -> i64 {
        let now = Utc::now().timestamp();
        (claims.exp - now).max(0)
    }
}

impl Default for JwtService {
    fn default() -> Self {
        Self::new()
    }
}

/// Current user context extracted from JWT
#[derive(Debug, Clone)]
pub struct CurrentUser {
    pub id: String,
    pub username: String,
    pub role: String,
    pub permissions: Vec<String>,
}

impl From<Claims> for CurrentUser {
    fn from(claims: Claims) -> Self {
        let permissions = if claims.permissions.is_empty() {
            vec![]
        } else {
            claims.permissions.split(',').map(|s| s.to_string()).collect()
        };

        Self {
            id: claims.sub,
            username: claims.username,
            role: claims.role,
            permissions,
        }
    }
}

impl CurrentUser {
    /// Check if user is an admin (role == "admin")
    pub fn is_admin(&self) -> bool {
        self.role == "admin"
    }

    /// Check if user has a specific permission
    /// Supports wildcard matching (e.g., "products:*" matches "products:create")
    pub fn has_permission(&self, permission: &str) -> bool {
        // Admin has all permissions
        if self.is_admin() {
            return true;
        }

        // Check for special 'all' permission
        if self.permissions.contains(&"all".to_string()) {
            return true;
        }

        // Check for exact match or wildcard match
        self.permissions.iter().any(|p| {
            if p == permission {
                return true;
            }
            // Handle wildcard patterns like "products:*" matching "products:create"
            if let Some(prefix) = p.strip_suffix(":*") {
                permission.starts_with(&format!("{}:", prefix))
            } else {
                false
            }
        })
    }

    /// Check if user has any of the specified permissions
    pub fn has_any_permission(&self, permissions: &[&str]) -> bool {
        if self.is_admin() {
            return true;
        }
        permissions.iter().any(|p| self.has_permission(p))
    }

    /// Check if user has all of the specified permissions
    pub fn has_all_permissions(&self, permissions: &[&str]) -> bool {
        if self.is_admin() {
            return true;
        }
        permissions.iter().all(|p| self.has_permission(p))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jwt_generation_and_validation() {
        let service = JwtService::new();
        let permissions = vec!["products:read".to_string(), "products:write".to_string()];

        let token = service
            .generate_token("user123", "john_doe", "user", &permissions)
            .unwrap();

        let claims = service.validate_token(&token).unwrap();

        assert_eq!(claims.sub, "user123");
        assert_eq!(claims.username, "john_doe");
        assert_eq!(claims.role, "user");
        assert_eq!(claims.permissions, "products:read,products:write");
    }

    #[test]
    fn test_current_user_permissions() {
        let user = CurrentUser {
            id: "1".to_string(),
            username: "john".to_string(),
            role: "user".to_string(),
            permissions: vec!["products:read".to_string(), "products:*".to_string()],
        };

        assert!(user.has_permission("products:read"));
        assert!(user.has_permission("products:create")); // Wildcard match
        assert!(!user.has_permission("users:read"));
    }

    #[test]
    fn test_admin_has_all_permissions() {
        let admin = CurrentUser {
            id: "1".to_string(),
            username: "admin".to_string(),
            role: "admin".to_string(),
            permissions: vec![],
        };

        assert!(admin.has_permission("products:read"));
        assert!(admin.has_permission("users:delete"));
        assert!(admin.is_admin());
    }
}
