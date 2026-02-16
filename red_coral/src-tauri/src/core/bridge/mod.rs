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
use std::sync::{Arc, Mutex};
use tauri::Emitter;
use tokio::sync::RwLock;

use super::tenant_manager::{TenantError, TenantManager};
use crab_client::CrabClient;
use shared::activation::SubscriptionStatus;
use shared::app_state::{ActivationRequiredReason, ClockDirection};
use shared::order::{
    CommandResponse, OrderCommand, OrderCommandPayload, OrderEvent, OrderSnapshot, SyncResponse,
};

/// 后端初始化内部状态
enum InitState {
    /// 正在初始化 (restore_last_session 还在跑)
    Pending,
    /// 初始化成功
    Ok,
    /// 初始化失败
    Failed(String),
}

/// 后端初始化状态 (前端可查询)
#[derive(Debug, Clone, serde::Serialize)]
pub struct InitStatus {
    /// 是否已完成初始化
    pub ready: bool,
    /// 初始化错误 (None = 成功)
    pub error: Option<String>,
}

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
    /// 后端初始化状态 (可重置，支持 retry)
    init_state: Mutex<InitState>,
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

        // 立即恢复租户选择（同步操作），确保 get_app_state 不会错误返回 ServerNoTenant
        if let Some(ref tenant_id) = config.current_tenant {
            if let Err(e) = tenant_manager.switch_tenant(tenant_id) {
                tracing::warn!("Failed to restore tenant {}: {}", tenant_id, e);
            }
        }

        Ok(Self {
            tenant_manager: Arc::new(RwLock::new(tenant_manager)),
            mode: RwLock::new(ClientMode::Disconnected),
            config: RwLock::new(config),
            config_path,
            app_handle,
            init_state: Mutex::new(InitState::Pending),
        })
    }

    /// Set the app handle after initialization
    pub fn set_app_handle(&mut self, handle: tauri::AppHandle) {
        self.app_handle = Some(handle);
    }

    /// 查询后端初始化状态
    pub fn get_init_status(&self) -> InitStatus {
        let state = self.init_state.lock().unwrap_or_else(|e| e.into_inner());
        match &*state {
            InitState::Pending => InitStatus {
                ready: false,
                error: None,
            },
            InitState::Ok => InitStatus {
                ready: true,
                error: None,
            },
            InitState::Failed(e) => InitStatus {
                ready: true,
                error: Some(e.clone()),
            },
        }
    }

    /// 标记初始化完成
    pub(crate) fn mark_initialized(&self, error: Option<String>) {
        let mut state = self.init_state.lock().unwrap_or_else(|e| e.into_inner());
        *state = match error {
            None => InitState::Ok,
            Some(e) => InitState::Failed(e),
        };
    }

    /// 重置初始化状态为 Pending (retry 前调用)
    pub(crate) fn reset_init_state(&self) {
        let mut state = self.init_state.lock().unwrap_or_else(|e| e.into_inner());
        *state = InitState::Pending;
    }

    /// 保存当前配置
    async fn save_config(&self) -> Result<(), BridgeError> {
        let config = self.config.read().await;
        config.save(&self.config_path)
    }
}
