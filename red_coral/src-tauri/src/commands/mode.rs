//! 模式管理命令

use std::sync::Arc;
use tauri::State;

use crate::core::response::{ApiResponse, AppConfigResponse, ErrorCode};
use crate::core::{AppState, ClientBridge, ModeInfo, ModeType, ServerModeConfig};

/// 获取应用状态 (用于前端路由守卫)
///
/// 返回当前应用所处的状态，前端可据此决定显示哪个页面。
/// 参考设计文档: `docs/plans/2026-01-18-application-state-machine.md`
#[tauri::command]
pub async fn get_app_state(
    bridge: State<'_, Arc<ClientBridge>>,
) -> Result<ApiResponse<AppState>, String> {
    Ok(ApiResponse::success(bridge.get_app_state().await))
}

/// 获取当前模式信息
#[tauri::command]
pub async fn get_mode_info(
    bridge: State<'_, Arc<ClientBridge>>,
) -> Result<ApiResponse<ModeInfo>, String> {
    Ok(ApiResponse::success(bridge.get_mode_info().await))
}

/// 启动 Server 模式
#[tauri::command]
pub async fn start_server_mode(
    bridge: State<'_, Arc<ClientBridge>>,
) -> Result<ApiResponse<()>, String> {
    match bridge.start_server_mode().await {
        Ok(_) => Ok(ApiResponse::success(())),
        Err(e) => Ok(ApiResponse::error_with_code(
            ErrorCode::InternalError,
            e.to_string(),
        )),
    }
}

/// 启动 Client 模式
#[tauri::command]
pub async fn start_client_mode(
    bridge: State<'_, Arc<ClientBridge>>,
    edge_url: String,
    message_addr: String,
) -> Result<ApiResponse<()>, String> {
    match bridge.start_client_mode(&edge_url, &message_addr).await {
        Ok(_) => Ok(ApiResponse::success(())),
        Err(e) => Ok(ApiResponse::error_with_code(
            ErrorCode::BridgeConnectionFailed,
            e.to_string(),
        )),
    }
}

/// 停止当前模式
#[tauri::command]
pub async fn stop_mode(
    bridge: State<'_, Arc<ClientBridge>>,
) -> Result<ApiResponse<()>, String> {
    match bridge.stop().await {
        Ok(_) => Ok(ApiResponse::success(())),
        Err(e) => Ok(ApiResponse::error_with_code(
            ErrorCode::InternalError,
            e.to_string(),
        )),
    }
}

/// 获取当前模式类型
#[tauri::command]
pub async fn get_current_mode_type(
    bridge: State<'_, Arc<ClientBridge>>,
) -> Result<ApiResponse<ModeType>, String> {
    let info = bridge.get_mode_info().await;
    Ok(ApiResponse::success(info.mode))
}

/// 检查是否首次运行
#[tauri::command]
pub async fn check_first_run(
    bridge: State<'_, Arc<ClientBridge>>,
) -> Result<ApiResponse<bool>, String> {
    let tenant_manager = bridge.tenant_manager().read().await;
    // 如果没有激活的租户，则认为是首次运行
    Ok(ApiResponse::success(
        tenant_manager.list_tenants().is_empty(),
    ))
}

/// 重新连接 (仅 Client 模式)
#[tauri::command]
pub async fn reconnect(
    bridge: State<'_, Arc<ClientBridge>>,
) -> Result<ApiResponse<()>, String> {
    // Clone the Arc to work with it independently
    let bridge_arc = (*bridge).clone();

    // Check mode and get client config
    let mode_info = bridge_arc.get_mode_info().await;
    if mode_info.mode != ModeType::Client {
        return Ok(ApiResponse::error_with_code(
            ErrorCode::InvalidRequest,
            "Reconnect is only available in Client mode",
        ));
    }

    let client_config = match bridge_arc.get_client_config().await {
        Some(config) => config,
        None => {
            return Ok(ApiResponse::error_with_code(
                ErrorCode::ConfigError,
                "No client configuration found",
            ))
        }
    };

    // Stop current connection
    if let Err(e) = bridge_arc.stop().await {
        return Ok(ApiResponse::error_with_code(
            ErrorCode::BridgeConnectionFailed,
            e.to_string(),
        ));
    }

    // Restart client mode
    if let Err(e) = bridge_arc
        .start_client_mode(&client_config.edge_url, &client_config.message_addr)
        .await
    {
        return Ok(ApiResponse::error_with_code(
            ErrorCode::BridgeConnectionFailed,
            e.to_string(),
        ));
    }

    tracing::info!("Client mode reconnected successfully");
    Ok(ApiResponse::success(()))
}

/// 更新 Server 模式配置
#[tauri::command]
pub async fn update_server_config(
    bridge: State<'_, Arc<ClientBridge>>,
    http_port: u16,
    message_port: u16,
) -> Result<ApiResponse<()>, String> {
    match bridge.update_server_config(http_port, message_port).await {
        Ok(_) => Ok(ApiResponse::success(())),
        Err(e) => Ok(ApiResponse::error_with_code(
            ErrorCode::ConfigError,
            e.to_string(),
        )),
    }
}

/// 更新 Client 模式配置
#[tauri::command]
pub async fn update_client_config(
    bridge: State<'_, Arc<ClientBridge>>,
    edge_url: String,
    message_addr: String,
    auth_url: String,
) -> Result<ApiResponse<()>, String> {
    match bridge
        .update_client_config(&edge_url, &message_addr, &auth_url)
        .await
    {
        Ok(_) => Ok(ApiResponse::success(())),
        Err(e) => Ok(ApiResponse::error_with_code(
            ErrorCode::ConfigError,
            e.to_string(),
        )),
    }
}

#[tauri::command]
pub async fn get_app_config(
    bridge: State<'_, Arc<ClientBridge>>,
) -> Result<ApiResponse<AppConfigResponse>, String> {
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
