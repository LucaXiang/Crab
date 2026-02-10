//! Bridge configuration types

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::error::BridgeError;
use super::types::ModeType;

/// Server 模式配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerModeConfig {
    /// HTTP 端口
    pub http_port: u16,
    /// 数据目录
    pub data_dir: PathBuf,
    /// 消息总线端口
    pub message_port: u16,
}

impl Default for ServerModeConfig {
    fn default() -> Self {
        Self {
            http_port: 9625,
            data_dir: PathBuf::from("./data"),
            message_port: 9626,
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
    std::env::var("AUTH_SERVER_URL").unwrap_or_else(|_| {
        tracing::debug!("AUTH_SERVER_URL not set, using development default");
        "http://127.0.0.1:3001".to_string()
    })
}

/// 应用配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// 当前模式 (None = 未选模式)
    pub current_mode: Option<ModeType>,
    /// 当前租户
    pub current_tenant: Option<String>,
    /// 当前激活的 entity_id (注销时用)
    #[serde(default)]
    pub active_entity_id: Option<String>,
    /// Server 模式配置
    pub server_config: ServerModeConfig,
    /// Client 模式配置
    pub client_config: Option<ClientModeConfig>,
    /// 已知租户列表
    pub known_tenants: Vec<String>,
    /// Auth Server URL (全局，激活和 edge-server 都用)
    #[serde(default = "default_auth_url")]
    pub auth_url: String,
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
        }
    }
}

impl AppConfig {
    /// 从文件加载配置
    pub fn load(path: &std::path::Path) -> Result<Self, BridgeError> {
        if path.exists() {
            let content = std::fs::read_to_string(path)?;
            serde_json::from_str(&content).map_err(|e| BridgeError::Config(e.to_string()))
        } else {
            Ok(Self::default())
        }
    }

    /// 保存配置到文件
    pub fn save(&self, path: &std::path::Path) -> Result<(), BridgeError> {
        let content =
            serde_json::to_string_pretty(self).map_err(|e| BridgeError::Config(e.to_string()))?;
        std::fs::write(path, content)?;
        Ok(())
    }
}
