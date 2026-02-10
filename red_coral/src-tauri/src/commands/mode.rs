//! 模式管理命令

use std::sync::Arc;
use tauri::{Emitter, State};

use crate::core::response::{ApiResponse, AppConfigResponse, ErrorCode};
use crate::core::bridge::InitStatus;
use crate::core::{AppState, ClientBridge, ModeInfo, ModeType};

/// 查询后端初始化状态 (先于 get_app_state 调用)
#[tauri::command]
pub fn get_init_status(
    bridge: State<'_, Arc<ClientBridge>>,
) -> Result<ApiResponse<InitStatus>, String> {
    Ok(ApiResponse::success(bridge.get_init_status()))
}

/// 重试后端初始化 (前端点击重试按钮时调用)
///
/// 1. 重置 init_state 为 Pending
/// 2. 重新 spawn restore_last_session
/// 3. 完成后存储结果 + 发送 backend-ready 事件
#[tauri::command]
pub async fn retry_init(
    bridge: State<'_, Arc<ClientBridge>>,
    app_handle: tauri::AppHandle,
) -> Result<ApiResponse<()>, String> {
    bridge.reset_init_state();

    let bridge_arc = (*bridge).clone();
    tauri::async_runtime::spawn(async move {
        let error = match bridge_arc.restore_last_session().await {
            Ok(()) => None,
            Err(e) => {
                tracing::error!("Retry restore_last_session failed: {}", e);
                Some(e.to_string())
            }
        };
        bridge_arc.mark_initialized(error.clone());
        let _ = app_handle.emit("backend-ready", error);
    });

    Ok(ApiResponse::success(()))
}

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
    match tokio::time::timeout(
        std::time::Duration::from_secs(30),
        bridge.start_server_mode(),
    )
    .await
    {
        Ok(Ok(_)) => Ok(ApiResponse::success(())),
        Ok(Err(e)) => Ok(ApiResponse::error_with_code(
            ErrorCode::InternalError,
            e.to_string(),
        )),
        Err(_) => Ok(ApiResponse::error_with_code(
            ErrorCode::InternalError,
            "Server startup timed out (30s). Check logs for details.",
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
) -> Result<ApiResponse<Option<ModeType>>, String> {
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
    if mode_info.mode != Some(ModeType::Client) {
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
) -> Result<ApiResponse<()>, String> {
    match bridge
        .update_client_config(&edge_url, &message_addr)
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
    let server_config = bridge.get_server_config().await;
    let client_config = bridge.get_client_config().await;
    let tenant_manager = bridge.tenant_manager().read().await;

    Ok(ApiResponse::success(AppConfigResponse {
        current_mode: info.mode,
        current_tenant: info.tenant_id,
        server_config,
        client_config,
        known_tenants: tenant_manager
            .list_tenants()
            .into_iter()
            .map(|t| t.tenant_id)
            .collect(),
    }))
}
