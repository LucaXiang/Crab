//! SessionCache - 员工会话缓存
//!
//! 支持离线登录的会话缓存机制。
//! 使用 Argon2 存储密码哈希，确保安全性。

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SessionCacheError {
    #[error("Employee not found: {0}")]
    EmployeeNotFound(String),

    #[error("Invalid password")]
    InvalidPassword,

    #[error("Session expired")]
    SessionExpired,

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Password hash error: {0}")]
    PasswordHash(String),
}

/// 登录模式
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum LoginMode {
    Online,
    Offline,
}

/// 员工会话
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EmployeeSession {
    pub username: String,
    pub token: String,
    pub user_info: shared::client::UserInfo,
    pub login_mode: LoginMode,
    pub expires_at: Option<u64>,
    pub logged_in_at: u64,
}

/// 缓存的员工数据
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct CachedEmployee {
    /// 密码的 Argon2 哈希
    password_hash: String,
    /// 缓存的 JWT token
    cached_token: String,
    /// Token 过期时间
    token_expires_at: Option<u64>,
    /// 用户信息
    user_info: shared::client::UserInfo,
    /// 上次在线登录时间
    last_online_login: u64,
}

/// 会话缓存文件结构
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
struct SessionCacheFile {
    employees: HashMap<String, CachedEmployee>,
}

/// 员工会话缓存管理器
pub struct SessionCache {
    /// 缓存文件路径
    file_path: PathBuf,
    /// 缓存数据
    data: SessionCacheFile,
}

impl SessionCache {
    /// 创建新的 SessionCache
    pub fn new(tenant_path: &Path) -> Self {
        let file_path = tenant_path.join("session_cache.json");
        Self {
            file_path,
            data: SessionCacheFile::default(),
        }
    }

    /// 从文件加载 SessionCache
    pub fn load(tenant_path: &Path) -> Result<Self, SessionCacheError> {
        let file_path = tenant_path.join("session_cache.json");

        let data = if file_path.exists() {
            let content = std::fs::read_to_string(&file_path)?;
            serde_json::from_str(&content)?
        } else {
            SessionCacheFile::default()
        };

        Ok(Self { file_path, data })
    }

    /// 保存到文件
    pub fn save(&self) -> Result<(), SessionCacheError> {
        let content = serde_json::to_string_pretty(&self.data)?;
        std::fs::write(&self.file_path, content)?;
        Ok(())
    }

    /// 更新员工缓存 (在线登录成功后调用)
    pub fn update_employee_cache(
        &mut self,
        username: &str,
        password: &str,
        session: &EmployeeSession,
    ) -> Result<(), SessionCacheError> {
        // 使用 Argon2 哈希密码
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let password_hash = argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| SessionCacheError::PasswordHash(e.to_string()))?
            .to_string();

        let cached = CachedEmployee {
            password_hash,
            cached_token: session.token.clone(),
            token_expires_at: session.expires_at,
            user_info: session.user_info.clone(),
            last_online_login: session.logged_in_at,
        };

        self.data.employees.insert(username.to_string(), cached);
        self.save()?;

        tracing::debug!(username = %username, "Employee cache updated");

        Ok(())
    }

    /// 验证离线登录
    pub fn verify_offline_login(
        &self,
        username: &str,
        password: &str,
    ) -> Result<EmployeeSession, SessionCacheError> {
        let cached = self.data.employees.get(username)
            .ok_or_else(|| SessionCacheError::EmployeeNotFound(username.to_string()))?;

        // 验证密码
        let parsed_hash = PasswordHash::new(&cached.password_hash)
            .map_err(|e| SessionCacheError::PasswordHash(e.to_string()))?;

        Argon2::default()
            .verify_password(password.as_bytes(), &parsed_hash)
            .map_err(|_| SessionCacheError::InvalidPassword)?;

        // 检查缓存的 token 是否过期
        // 离线模式下我们仍然使用缓存的 token，但标记为离线登录
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // 如果 token 过期超过 7 天，拒绝离线登录
        if let Some(expires_at) = cached.token_expires_at {
            let max_offline_duration = 7 * 24 * 60 * 60; // 7 days in seconds
            if now > expires_at + max_offline_duration {
                return Err(SessionCacheError::SessionExpired);
            }
        }

        Ok(EmployeeSession {
            username: username.to_string(),
            token: cached.cached_token.clone(),
            user_info: cached.user_info.clone(),
            login_mode: LoginMode::Offline,
            expires_at: cached.token_expires_at,
            logged_in_at: now,
        })
    }

    /// 检查是否有员工缓存
    pub fn has_employee(&self, username: &str) -> bool {
        self.data.employees.contains_key(username)
    }

    /// 移除员工缓存
    pub fn remove_employee(&mut self, username: &str) -> Result<(), SessionCacheError> {
        self.data.employees.remove(username);
        self.save()?;
        Ok(())
    }

    /// 清除所有缓存
    pub fn clear(&mut self) -> Result<(), SessionCacheError> {
        self.data.employees.clear();
        self.save()?;
        Ok(())
    }

    /// 获取缓存的员工列表
    pub fn list_employees(&self) -> Vec<String> {
        self.data.employees.keys().cloned().collect()
    }

    // ============ 当前活动会话持久化 ============

    /// 保存当前活动会话 (用于刷新后恢复登录状态)
    pub fn save_current_session(&self, session: &EmployeeSession) -> Result<(), SessionCacheError> {
        let path = self.file_path.parent()
            .map(|p| p.join("current_session.json"))
            .ok_or_else(|| SessionCacheError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Invalid session cache path"
            )))?;

        let content = serde_json::to_string_pretty(session)?;
        std::fs::write(&path, content)?;
        tracing::debug!(username = %session.username, "Current session saved");
        Ok(())
    }

    /// 加载当前活动会话
    pub fn load_current_session(&self) -> Result<Option<EmployeeSession>, SessionCacheError> {
        let path = self.file_path.parent()
            .map(|p| p.join("current_session.json"))
            .ok_or_else(|| SessionCacheError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Invalid session cache path"
            )))?;

        if !path.exists() {
            return Ok(None);
        }

        let content = std::fs::read_to_string(&path)?;
        let session: EmployeeSession = serde_json::from_str(&content)?;

        // 检查 session 是否过期 (token expires_at)
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        if let Some(expires_at) = session.expires_at {
            if now > expires_at {
                // Token 过期，清除缓存
                let _ = std::fs::remove_file(&path);
                tracing::info!(username = %session.username, "Cached session expired, cleared");
                return Ok(None);
            }
        }

        tracing::info!(username = %session.username, "Loaded cached session");
        Ok(Some(session))
    }

    /// 清除当前活动会话
    pub fn clear_current_session(&self) -> Result<(), SessionCacheError> {
        let path = self.file_path.parent()
            .map(|p| p.join("current_session.json"))
            .ok_or_else(|| SessionCacheError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Invalid session cache path"
            )))?;

        if path.exists() {
            std::fs::remove_file(&path)?;
            tracing::debug!("Current session cleared");
        }
        Ok(())
    }
}
