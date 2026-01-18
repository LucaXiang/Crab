//! 模式管理命令

use std::sync::Arc;
use tauri::State;
use tokio::sync::RwLock;

use crate::core::{ClientBridge, ClientModeConfig, ModeInfo, ModeType, ServerModeConfig};

/// 获取当前模式信息
#[tauri::command]
pub async fn get_mode_info(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
) -> Result<ModeInfo, String> {
    let bridge = bridge.read().await;
    Ok(bridge.get_mode_info().await)
}

/// 启动 Server 模式
#[tauri::command]
pub async fn start_server_mode(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
) -> Result<(), String> {
    let bridge = bridge.read().await;
    bridge.start_server_mode().await.map_err(|e| e.to_string())
}

/// 启动 Client 模式
#[tauri::command]
pub async fn start_client_mode(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    edge_url: String,
    message_addr: String,
) -> Result<(), String> {
    let bridge = bridge.read().await;
    bridge
        .start_client_mode(&edge_url, &message_addr)
        .await
        .map_err(|e| e.to_string())
}

/// 停止当前模式
#[tauri::command]
pub async fn stop_mode(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
) -> Result<(), String> {
    let bridge = bridge.read().await;
    bridge.stop().await.map_err(|e| e.to_string())
}

/// 获取当前模式类型
#[tauri::command]
pub async fn get_current_mode_type(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
) -> Result<ModeType, String> {
    let bridge = bridge.read().await;
    let info = bridge.get_mode_info().await;
    Ok(info.mode)
}

/// 检查是否首次运行
#[tauri::command]
pub async fn check_first_run(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
) -> Result<bool, String> {
    let bridge = bridge.read().await;
    let tenant_manager = bridge.tenant_manager().read().await;
    // 如果没有激活的租户，则认为是首次运行
    Ok(tenant_manager.list_tenants().is_empty())
}

/// 获取应用配置 (用于前端显示/编辑)
#[derive(Debug, Clone, serde::Serialize)]
pub struct AppConfigResponse {
    pub current_mode: ModeType,
    pub current_tenant: Option<String>,
    pub server_config: ServerModeConfig,
    pub client_config: Option<ClientModeConfig>,
    pub known_tenants: Vec<String>,
}

#[tauri::command]
pub async fn get_app_config(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
) -> Result<AppConfigResponse, String> {
    let bridge = bridge.read().await;
    let info = bridge.get_mode_info().await;
    let tenant_manager = bridge.tenant_manager().read().await;

    // 构建响应
    Ok(AppConfigResponse {
        current_mode: info.mode,
        current_tenant: info.tenant_id,
        server_config: ServerModeConfig::default(), // TODO: 从配置读取
        client_config: None, // TODO: 从配置读取
        known_tenants: tenant_manager
            .list_tenants()
            .into_iter()
            .map(|t| t.tenant_id)
            .collect(),
    })
}
