//! 模式管理命令

use std::sync::Arc;
use tauri::State;
use tokio::sync::RwLock;

use crate::core::response::{ApiResponse, AppConfigResponse, ErrorCode};
use crate::core::{AppState, ClientBridge, ModeInfo, ModeType, ServerModeConfig};

/// 获取应用状态 (用于前端路由守卫)
///
/// 返回当前应用所处的状态，前端可据此决定显示哪个页面。
/// 参考设计文档: `docs/plans/2026-01-18-application-state-machine.md`
#[tauri::command(rename_all = "snake_case")]
pub async fn get_app_state(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
) -> Result<ApiResponse<AppState>, String> {
    let bridge = bridge.read().await;
    Ok(ApiResponse::success(bridge.get_app_state().await))
}

/// 获取当前模式信息
#[tauri::command(rename_all = "snake_case")]
pub async fn get_mode_info(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
) -> Result<ApiResponse<ModeInfo>, String> {
    let bridge = bridge.read().await;
    Ok(ApiResponse::success(bridge.get_mode_info().await))
}

/// 启动 Server 模式
#[tauri::command(rename_all = "snake_case")]
pub async fn start_server_mode(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
) -> Result<ApiResponse<()>, String> {
    let bridge = bridge.read().await;
    match bridge.start_server_mode().await {
        Ok(_) => Ok(ApiResponse::success(())),
        Err(e) => Ok(ApiResponse::error_with_code(ErrorCode::InternalError, e.to_string())),
    }
}

/// 启动 Client 模式
#[tauri::command(rename_all = "snake_case")]
pub async fn start_client_mode(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    edge_url: String,
    message_addr: String,
) -> Result<ApiResponse<()>, String> {
    let bridge = bridge.read().await;
    match bridge.start_client_mode(&edge_url, &message_addr).await {
        Ok(_) => Ok(ApiResponse::success(())),
        Err(e) => Ok(ApiResponse::error_with_code(ErrorCode::BridgeConnectionFailed, e.to_string())),
    }
}

/// 停止当前模式
#[tauri::command(rename_all = "snake_case")]
pub async fn stop_mode(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
) -> Result<ApiResponse<()>, String> {
    let bridge = bridge.read().await;
    match bridge.stop().await {
        Ok(_) => Ok(ApiResponse::success(())),
        Err(e) => Ok(ApiResponse::error_with_code(ErrorCode::InternalError, e.to_string())),
    }
}

/// 获取当前模式类型
#[tauri::command(rename_all = "snake_case")]
pub async fn get_current_mode_type(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
) -> Result<ApiResponse<ModeType>, String> {
    let bridge = bridge.read().await;
    let info = bridge.get_mode_info().await;
    Ok(ApiResponse::success(info.mode))
}

/// 检查是否首次运行
#[tauri::command(rename_all = "snake_case")]
pub async fn check_first_run(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
) -> Result<ApiResponse<bool>, String> {
    let bridge = bridge.read().await;
    let tenant_manager = bridge.tenant_manager().read().await;
    // 如果没有激活的租户，则认为是首次运行
    Ok(ApiResponse::success(
        tenant_manager.list_tenants().is_empty(),
    ))
}

/// 重新连接 (仅 Client 模式)
#[tauri::command(rename_all = "snake_case")]
pub async fn reconnect(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
) -> Result<ApiResponse<()>, String> {
    // Clone the Arc to work with it independently
    let bridge_arc = (*bridge).clone();

    // First, read mode info and client config
    let client_config = {
        let bridge = bridge_arc.read().await;
        let mode_info = bridge.get_mode_info().await;

        if mode_info.mode != ModeType::Client {
            return Ok(ApiResponse::error_with_code(
                ErrorCode::InvalidRequest,
                "Reconnect is only available in Client mode",
            ));
        }

        // Get client config from the stored AppConfig
        bridge.get_client_config().await
    };

    // Extract client config details
    let client_config = match client_config {
        Some(config) => config,
        None => {
            return Ok(ApiResponse::error_with_code(
                ErrorCode::ConfigError,
                "No client configuration found",
            ))
        }
    };

    // Now stop and restart
    {
        let bridge = bridge_arc.read().await;
        if let Err(e) = bridge.stop().await {
            return Ok(ApiResponse::error_with_code(ErrorCode::BridgeConnectionFailed, e.to_string()));
        }
    }

    // Restart client mode
    {
        let bridge = bridge_arc.read().await;
        if let Err(e) = bridge
            .start_client_mode(&client_config.edge_url, &client_config.message_addr)
            .await
        {
            return Ok(ApiResponse::error_with_code(ErrorCode::BridgeConnectionFailed, e.to_string()));
        }
    }

    tracing::info!("Client mode reconnected successfully");
    Ok(ApiResponse::success(()))
}

#[tauri::command(rename_all = "snake_case")]
pub async fn get_app_config(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
) -> Result<ApiResponse<AppConfigResponse>, String> {
    let bridge = bridge.read().await;
    let info = bridge.get_mode_info().await;
    let tenant_manager = bridge.tenant_manager().read().await;

    // 构建响应
    Ok(ApiResponse::success(AppConfigResponse {
        current_mode: info.mode,
        current_tenant: info.tenant_id,
        server_config: ServerModeConfig::default(), // TODO: 从配置读取
        client_config: None,                        // TODO: 从配置读取
        known_tenants: tenant_manager
            .list_tenants()
            .into_iter()
            .map(|t| t.tenant_id)
            .collect(),
    }))
}
