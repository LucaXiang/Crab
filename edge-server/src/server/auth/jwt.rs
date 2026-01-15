//! JWT 令牌服务
//!
//! 处理 JWT 令牌的生成、验证和解析。

use chrono::{Duration, Utc};
use jsonwebtoken::errors::ErrorKind;
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// JWT 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtConfig {
    /// JWT 密钥 (应至少 32 字节)
    pub secret: String,
    /// 令牌过期时间 (分钟)
    pub expiration_minutes: i64,
    /// 令牌签发者
    pub issuer: String,
    /// 令牌受众
    pub audience: String,
}

impl Default for JwtConfig {
    fn default() -> Self {
        let secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| {
            #[cfg(debug_assertions)]
            {
                tracing::warn!(
                    "⚠️  JWT_SECRET not set! Using insecure default key. DO NOT USE IN PRODUCTION!"
                );
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
                .unwrap_or(1440), // 默认 24 小时
            issuer: std::env::var("JWT_ISSUER").unwrap_or_else(|_| "edge-server".to_string()),
            audience: std::env::var("JWT_AUDIENCE").unwrap_or_else(|_| "edge-clients".to_string()),
        }
    }
}

/// 存储在令牌中的 JWT Claims
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// 用户 ID (Subject)
    pub sub: String,
    /// 用户名
    pub username: String,
    /// 角色名称
    pub role: String,
    /// 权限列表 (逗号分隔)
    pub permissions: String,
    /// 令牌类型
    pub token_type: String,
    /// 过期时间戳
    pub exp: i64,
    /// 签发时间戳
    pub iat: i64,
    /// 签发者
    pub iss: String,
    /// 受众
    pub aud: String,
}

/// JWT 错误
#[derive(Error, Debug)]
pub enum JwtError {
    #[error("无效令牌: {0}")]
    InvalidToken(String),

    #[error("令牌已过期")]
    ExpiredToken,

    #[error("无效签名")]
    InvalidSignature,

    #[error("令牌生成失败: {0}")]
    GenerationFailed(String),
}

/// JWT 令牌服务
#[derive(Debug, Clone)]
pub struct JwtService {
    pub config: JwtConfig,
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
}

impl JwtService {
    /// 使用默认配置创建新的 JWT 服务
    pub fn new() -> Self {
        Self::with_config(JwtConfig::default())
    }

    /// 使用指定配置创建新的 JWT 服务
    pub fn with_config(config: JwtConfig) -> Self {
        let encoding_key = EncodingKey::from_secret(config.secret.as_bytes());
        let decoding_key = DecodingKey::from_secret(config.secret.as_bytes());

        Self {
            config,
            encoding_key,
            decoding_key,
        }
    }

    /// 为用户生成新令牌
    pub fn generate_token(
        &self,
        user_id: &str,
        username: &str,
        role: &str,
        permissions: &[String],
    ) -> Result<String, JwtError> {
        let now = Utc::now();
        let expiration = now + Duration::minutes(self.config.expiration_minutes);

        let permissions_str = permissions.join(",");

        let claims = Claims {
            sub: user_id.to_string(),
            username: username.to_string(),
            role: role.to_string(),
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

    /// 验证并解码令牌
    pub fn validate_token(&self, token: &str) -> Result<Claims, JwtError> {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.set_audience(&[&self.config.audience]);
        validation.set_issuer(&[&self.config.issuer]);

        let token_data = decode::<Claims>(token, &self.decoding_key, &validation).map_err(|e| {
            match e.kind() {
                ErrorKind::ExpiredSignature => JwtError::ExpiredToken,
                ErrorKind::InvalidSignature => JwtError::InvalidSignature,
                _ => JwtError::InvalidToken(e.to_string()),
            }
        })?;

        Ok(token_data.claims)
    }

    /// 从 Authorization 头提取令牌
    pub fn extract_from_header(header: &str) -> Option<&str> {
        header.strip_prefix("Bearer ")
    }

    /// 获取距离过期的剩余秒数
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
            claims
                .permissions
                .split(',')
                .map(|s| s.to_string())
                .collect()
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
