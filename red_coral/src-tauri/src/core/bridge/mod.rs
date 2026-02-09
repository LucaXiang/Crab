//! ClientBridge - 统一的客户端桥接层
//!
//! 提供 Server/Client 模式的统一接口，屏蔽底层差异。
//! - Server 模式: 本地运行 edge-server，使用 In-Process 通信
//! - Client 模式: 连接远程 edge-server，使用 mTLS 通信

mod activation;
mod api;
mod auth;
mod config;
mod error;
mod lifecycle;
mod order_es;
mod state;
mod types;

// Re-export public types
pub use config::{AppConfig, ClientModeConfig, ServerModeConfig};
pub use error::BridgeError;
pub use types::{AppState, ModeInfo, ModeType};

// Internal types (pub(crate) for use within this crate)
pub(crate) use types::{ClientMode, LocalClientState, RemoteClientState};

use std::path::PathBuf;
use std::sync::Arc;
use tauri::Emitter;
use tokio::sync::RwLock;

use crab_client::CrabClient;
use shared::activation::SubscriptionStatus;
use shared::app_state::{ActivationRequiredReason, ClockDirection};
use shared::order::{CommandResponse, OrderCommand, OrderCommandPayload, OrderEvent, OrderSnapshot, SyncResponse};
use super::tenant_manager::{TenantError, TenantManager};

/// 客户端桥接层
pub struct ClientBridge {
    /// 多租户管理器
    tenant_manager: Arc<RwLock<TenantManager>>,
    /// 当前模式
    mode: RwLock<ClientMode>,
    /// 应用配置
    config: RwLock<AppConfig>,
    /// 配置文件路径
    config_path: PathBuf,
    /// Tauri AppHandle for emitting events (optional for testing)
    app_handle: Option<tauri::AppHandle>,
}

impl ClientBridge {
    /// 创建新的 ClientBridge (convenience wrapper without AppHandle)
    pub fn new(base_path: impl Into<PathBuf>, client_name: &str) -> Result<Self, BridgeError> {
        Self::with_app_handle(base_path, client_name, None)
    }

    /// 创建新的 ClientBridge with optional AppHandle for emitting Tauri events
    pub fn with_app_handle(
        base_path: impl Into<PathBuf>,
        client_name: &str,
        app_handle: Option<tauri::AppHandle>,
    ) -> Result<Self, BridgeError> {
        let base_path = base_path.into();
        std::fs::create_dir_all(&base_path)?;

        let config_path = base_path.join("config.json");
        let config = AppConfig::load(&config_path)?;

        let tenants_path = base_path.join("tenants");
        let mut tenant_manager = TenantManager::new(&tenants_path, client_name);
        tenant_manager.load_existing_tenants()?;

        Ok(Self {
            tenant_manager: Arc::new(RwLock::new(tenant_manager)),
            mode: RwLock::new(ClientMode::Disconnected),
            config: RwLock::new(config),
            config_path,
            app_handle,
        })
    }

    /// Set the app handle after initialization
    pub fn set_app_handle(&mut self, handle: tauri::AppHandle) {
        self.app_handle = Some(handle);
    }

    /// 保存当前配置
    async fn save_config(&self) -> Result<(), BridgeError> {
        let config = self.config.read().await;
        config.save(&self.config_path)
    }
}
