//! Bridge configuration types

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::path::Path;

use super::error::BridgeError;
use super::types::ModeType;

/// 从 JSON 文件加载数据，反序列化失败时备份损坏文件并返回默认值。
///
/// 确保 app 永远能启动 —— 任何本地缓存文件的损坏/不兼容都不阻止启动。
pub fn resilient_load<T: Default + DeserializeOwned>(path: &Path) -> T {
    if !path.exists() {
        return T::default();
    }

    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!(path = %path.display(), error = %e, "Failed to read cache file, using defaults");
            return T::default();
        }
    };

    match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!(
                path = %path.display(),
                error = %e,
                "Corrupt/incompatible cache file, backing up and resetting"
            );
            // 备份损坏文件
            let bak = path.with_extension("json.bak");
            if let Err(bak_err) = std::fs::rename(path, &bak) {
                tracing::warn!(error = %bak_err, "Failed to backup corrupt file");
                // 至少尝试删除，避免下次启动再次触发
                let _ = std::fs::remove_file(path);
            }
            T::default()
        }
    }
}

fn default_cloud_url() -> String {
    std::env::var("CRAB_CLOUD_URL").unwrap_or_else(|_| shared::DEFAULT_CLOUD_SYNC_URL.to_string())
}

/// Server 模式配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerModeConfig {
    /// HTTP 端口
    pub http_port: u16,
    /// 消息总线端口
    pub message_port: u16,
    /// Cloud sync URL (mTLS, port 8443)
    #[serde(default = "default_cloud_url")]
    pub cloud_url: String,
}

impl Default for ServerModeConfig {
    fn default() -> Self {
        Self {
            http_port: 9625,
            message_port: 9626,
            cloud_url: default_cloud_url(),
        }
    }
}

/// Client 模式配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientModeConfig {
    /// Edge Server URL (HTTPS)
    pub edge_url: String,
    /// 消息总线地址
    pub message_addr: String,
}

fn default_auth_url() -> String {
    enforce_https(
        &std::env::var("AUTH_SERVER_URL")
            .unwrap_or_else(|_| shared::DEFAULT_AUTH_SERVER_URL.to_string()),
    )
}

/// http:// → https:// 强制升级（敏感数据绝不走明文）
fn enforce_https(url: &str) -> String {
    if let Some(rest) = url.strip_prefix("http://") {
        format!("https://{rest}")
    } else {
        url.to_string()
    }
}

/// 应用配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// 当前模式 (None = 未选模式)
    pub current_mode: Option<ModeType>,
    /// 当前租户
    pub current_tenant: Option<i64>,
    /// 当前激活的 entity_id (注销时用)
    #[serde(default)]
    pub active_entity_id: Option<String>,
    /// Server 模式配置
    pub server_config: ServerModeConfig,
    /// Client 模式配置
    pub client_config: Option<ClientModeConfig>,
    /// 已知租户列表
    pub known_tenants: Vec<i64>,
    /// Auth Server URL (全局，激活和 edge-server 都用)
    #[serde(default = "default_auth_url")]
    pub auth_url: String,
    /// Refresh token (用于无需重新输入密码即可获取 JWT)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            current_mode: None,
            current_tenant: None,
            active_entity_id: None,
            server_config: ServerModeConfig::default(),
            client_config: None,
            known_tenants: Vec::new(),
            auth_url: default_auth_url(),
            refresh_token: None,
        }
    }
}

impl AppConfig {
    /// 从文件加载配置
    ///
    /// 反序列化失败时自动备份损坏文件并返回默认配置，确保 app 永远能启动。
    pub fn load(path: &std::path::Path) -> Result<Self, BridgeError> {
        let mut config: Self = resilient_load(path);
        // 强制 HTTPS — P12 等敏感数据绝不能走明文
        config.auth_url = enforce_https(&config.auth_url);

        // Debug 构建强制使用 dev 环境 URL，忽略 config.json 中缓存的生产地址
        #[cfg(debug_assertions)]
        {
            config.auth_url = shared::DEFAULT_AUTH_SERVER_URL.to_string();
            config.server_config.cloud_url = shared::DEFAULT_CLOUD_SYNC_URL.to_string();
        }

        tracing::info!(
            auth_url = %config.auth_url,
            cloud_url = %config.server_config.cloud_url,
            debug_build = cfg!(debug_assertions),
            "AppConfig loaded"
        );

        Ok(config)
    }

    /// 保存配置到文件
    pub fn save(&self, path: &std::path::Path) -> Result<(), BridgeError> {
        let content =
            serde_json::to_string_pretty(self).map_err(|e| BridgeError::Config(e.to_string()))?;
        std::fs::write(path, content)?;
        Ok(())
    }
}
