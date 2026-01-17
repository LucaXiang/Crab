// crab-client/src/cert/credential.rs
// 凭证存储 - 支持 JSON 文件存储

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::fs;

/// 凭证结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Credential {
    pub client_name: String,
    pub token: String,              // Auth Server token (租户)
    pub employee_token: Option<String>, // Employee token (员工)
    pub expires_at: Option<u64>,
    pub tenant_id: String,
}

impl Credential {
    pub fn new(client_name: String, token: String, expires_at: Option<u64>, tenant_id: String) -> Self {
        Self {
            client_name,
            token,
            employee_token: None,
            expires_at,
            tenant_id,
        }
    }

    /// 设置员工 token
    pub fn set_employee_token(&mut self, token: String) {
        self.employee_token = Some(token);
    }

    /// 获取员工 token
    pub fn employee_token(&self) -> Option<&str> {
        self.employee_token.as_deref()
    }

    /// 清除员工 token
    pub fn clear_employee_token(&mut self) {
        self.employee_token = None;
    }

    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            // 获取当前时间的秒数
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            return now > expires_at;
        }
        false
    }
}

/// 凭证存储
#[derive(Debug, Clone)]
pub struct CredentialStorage {
    path: PathBuf,
}

impl CredentialStorage {
    /// 创建凭证存储
    pub fn new(base_path: impl Into<PathBuf>, filename: &str) -> Self {
        let path = base_path.into().join(filename);
        Self { path }
    }

    /// 确保目录存在
    pub fn ensure_dir(&self) -> std::io::Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        Ok(())
    }

    /// 保存凭证
    pub fn save(&self, credential: &Credential) -> std::io::Result<()> {
        self.ensure_dir()?;
        let json = serde_json::to_string_pretty(credential)?;
        fs::write(&self.path, json)
    }

    /// 加载凭证
    pub fn load(&self) -> Option<Credential> {
        if !self.path.exists() {
            return None;
        }
        let json = fs::read_to_string(&self.path).ok()?;
        serde_json::from_str(&json).ok()
    }

    /// 检查凭证是否存在
    pub fn exists(&self) -> bool {
        self.path.exists()
    }

    /// 删除凭证
    pub fn delete(&self) -> std::io::Result<()> {
        if self.path.exists() {
            fs::remove_file(&self.path)?;
        }
        Ok(())
    }

    /// 获取路径
    pub fn path(&self) -> &Path {
        &self.path
    }
}
