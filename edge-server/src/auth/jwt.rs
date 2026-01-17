//! JWT 令牌服务
//!
//! 处理 JWT 令牌的生成、验证和解析。

use chrono::{Duration, Utc};
use jsonwebtoken::errors::ErrorKind;
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use ring::rand::{SecureRandom, SystemRandom};
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
        let secret = match load_jwt_secret() {
            Ok(key) => String::from_utf8(key).unwrap_or_else(|_| {
                tracing::error!("JWT secret contains invalid UTF-8 characters");
                generate_secure_jwt_secret()
                    .map(|key| {
                        String::from_utf8(key).unwrap_or_else(|_| {
                            "emergency-fallback-key-must-be-replaced".to_string()
                        })
                    })
                    .unwrap_or_else(|_| "emergency-fallback-key-must-be-replaced".to_string())
            }),
            Err(e) => {
                #[cfg(debug_assertions)]
                {
                    tracing::warn!("JWT configuration error: {}, using emergency key", e);
                    "emergency-fallback-key-must-be-replaced-in-production".to_string()
                }
                #[cfg(not(debug_assertions))]
                {
                    panic!("🚨 FATAL: JWT_SECRET configuration failed: {}", e);
                }
            }
        };

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

    #[error("密钥生成失败: {0}")]
    KeyGenerationFailed(String),

    #[error("配置错误: {0}")]
    ConfigError(String),
}

/// 生成安全的 JWT 密钥 (可打印字符)
pub fn generate_secure_jwt_secret() -> Result<Vec<u8>, JwtError> {
    let rng = SystemRandom::new();
    let mut key = vec![0u8; 32]; // 256-bit key

    rng.fill(&mut key).map_err(|_| {
        JwtError::KeyGenerationFailed("Failed to generate secure random key".to_string())
    })?;

    Ok(key)
}

/// 生成可打印的安全 JWT 密钥 (用于开发环境)
pub fn generate_secure_printable_jwt_secret() -> String {
    let allowed_chars =
        "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789!@#$%^&*()-_=+[]{}|;:,.<>?";

    let rng = SystemRandom::new();
    let mut key = String::new();

    for _ in 0..64 {
        // 生成64个字符的密钥
        let mut byte = [0u8; 1];
        if rng.fill(&mut byte).is_err() {
            // 如果随机数生成失败，使用固定的安全密钥
            return "CrabEdgeServerDevelopmentSecureKey2024!".to_string();
        }
        let idx = (byte[0] as usize) % allowed_chars.len();
        key.push(allowed_chars.chars().nth(idx).unwrap());
    }

    key
}

/// 从环境变量安全地加载 JWT 密钥
fn load_jwt_secret() -> Result<Vec<u8>, JwtError> {
    match std::env::var("JWT_SECRET") {
        Ok(secret) => {
            if secret.len() < 32 {
                return Err(JwtError::ConfigError(
                    "JWT_SECRET must be at least 32 characters long".to_string(),
                ));
            }
            Ok(secret.into_bytes())
        }
        Err(_) => {
            #[cfg(debug_assertions)]
            {
                tracing::warn!(
                    "⚠️  JWT_SECRET not set! Generating secure temporary key for development."
                );
                let printable_key = generate_secure_printable_jwt_secret();
                Ok(printable_key.into_bytes())
            }
            #[cfg(not(debug_assertions))]
            {
                Err(JwtError::ConfigError(
                    "JWT_SECRET environment variable must be set in production!".to_string(),
                ))
            }
        }
    }
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
    pub fn new_with_secure_key() -> Result<Self, JwtError> {
        let secret = generate_secure_jwt_secret()?;
        let config = JwtConfig {
            secret: String::from_utf8(secret)
                .map_err(|_| JwtError::ConfigError("Invalid UTF-8 in generated key".to_string()))?,
            ..Default::default()
        };
        Ok(Self::with_config(config))
    }

    /// 验证并解码令牌
    pub fn validate_token(&self, token: &str) -> Result<Claims, JwtError> {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.set_audience(&[&self.config.audience]);
        validation.set_issuer(&[&self.config.issuer]);
        validation.set_required_spec_claims(&["sub", "exp", "iat", "iss", "aud"]);

        let token_data = decode::<Claims>(token, &self.decoding_key, &validation).map_err(|e| {
            match e.kind() {
                ErrorKind::ExpiredSignature => JwtError::ExpiredToken,
                ErrorKind::InvalidSignature => JwtError::InvalidSignature,
                ErrorKind::InvalidToken => JwtError::InvalidToken(e.to_string()),
                _ => JwtError::InvalidToken(format!("Token validation failed: {}", e)),
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

/// 当前用户上下文 (从 JWT Claims 解析)
///
/// 由认证中间件创建，注入到请求处理函数
///
/// # 示例
///
/// ```ignore
/// async fn handler(user: CurrentUser) -> Json<()> {
///     println!("用户: {}, 角色: {}", user.username, user.role);
///     // 检查权限
///     if user.has_permission("products:write") {
///         // 有权限
///     }
/// }
/// ```
#[derive(Debug, Clone)]
pub struct CurrentUser {
    /// 用户 ID
    pub id: String,
    /// 用户名
    pub username: String,
    /// 角色名称
    pub role: String,
    /// 权限列表
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
    /// 是否管理员
    ///
    /// 管理员角色 (`role == "admin"`) 拥有所有权限
    pub fn is_admin(&self) -> bool {
        self.role == "admin"
    }

    /// 检查是否拥有指定权限
    ///
    /// 支持通配符匹配：
    /// - `"products:*"` 匹配 `"products:create"`, `"products:read"` 等
    /// - `"all"` 表示拥有所有权限
    ///
    /// # 规则
    ///
    /// 1. 管理员拥有所有权限
    /// 2. 权限列表包含 `"all"` 则拥有所有权限
    /// 3. 精确匹配或前缀匹配 (`:*` 通配符)
    pub fn has_permission(&self, permission: &str) -> bool {
        // 管理员拥有所有权限
        if self.is_admin() {
            return true;
        }

        // 检查特殊 'all' 权限
        if self.permissions.contains(&"all".to_string()) {
            return true;
        }

        // 精确匹配或通配符匹配
        self.permissions.iter().any(|p| {
            if p == permission {
                return true;
            }
            // 处理通配符模式，如 "products:*" 匹配 "products:create"
            if let Some(prefix) = p.strip_suffix(":*") {
                permission.starts_with(&format!("{}:", prefix))
            } else {
                false
            }
        })
    }

    /// 检查是否拥有任一指定权限
    pub fn has_any_permission(&self, permissions: &[&str]) -> bool {
        if self.is_admin() {
            return true;
        }
        permissions.iter().any(|p| self.has_permission(p))
    }

    /// 检查是否拥有所有指定权限
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
            .expect("Failed to generate test token");

        let claims = service
            .validate_token(&token)
            .expect("Failed to validate test token");

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

    #[test]
    fn test_secure_key_generation() {
        // Test that secure key generation works
        let key1 = generate_secure_jwt_secret().expect("Failed to generate first secure key");
        let key2 = generate_secure_jwt_secret().expect("Failed to generate second secure key");

        // Keys should be different (high probability)
        assert_ne!(key1, key2);

        // Keys should be 32 bytes
        assert_eq!(key1.len(), 32);
        assert_eq!(key2.len(), 32);
    }

    #[test]
    fn test_jwt_service_with_secure_key() {
        // Test creating JWT service with secure key
        let service = JwtService::new_with_secure_key()
            .expect("Failed to create JWT service with secure key");
        let permissions = vec!["products:read".to_string(), "products:write".to_string()];

        let token = service
            .generate_token("user123", "john_doe", "user", &permissions)
            .expect("Failed to generate test token");

        let claims = service
            .validate_token(&token)
            .expect("Failed to validate test token");

        assert_eq!(claims.sub, "user123");
        assert_eq!(claims.username, "john_doe");
        assert_eq!(claims.role, "user");
        assert_eq!(claims.permissions, "products:read,products:write");
    }
}
