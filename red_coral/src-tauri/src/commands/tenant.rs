//! 租户管理命令

use std::sync::Arc;
use tauri::State;
use tokio::sync::RwLock;

use crate::core::response::{ActivationResultData, ApiResponse, ErrorCode, TenantListData};
use crate::core::ClientBridge;
use crate::core::DeleteData;

/// 获取已激活的租户列表
#[tauri::command(rename_all = "snake_case")]
pub async fn list_tenants(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
) -> Result<ApiResponse<TenantListData>, String> {
    let bridge = bridge.read().await;
    let tenant_manager = bridge.tenant_manager().read().await;
    Ok(ApiResponse::success(TenantListData {
        tenants: tenant_manager.list_tenants(),
    }))
}

/// 激活新租户 (设备激活)
///
/// 同时预激活 edge-server，为 Server 模式做准备
#[tauri::command(rename_all = "snake_case")]
pub async fn activate_tenant(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    auth_url: String,
    username: String,
    password: String,
) -> Result<ApiResponse<ActivationResultData>, String> {
    let bridge = bridge.read().await;

    match bridge
        .handle_activation(&auth_url, &username, &password)
        .await
    {
        Ok((tenant_id, subscription_status)) => Ok(ApiResponse::success(ActivationResultData {
            tenant_id,
            subscription_status,
        })),
        Err(e) => Ok(ApiResponse::error_with_code(
            ErrorCode::ActivationFailed,
            e.to_string(),
        )),
    }
}

/// 切换当前租户
#[tauri::command(rename_all = "snake_case")]
pub async fn switch_tenant(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    tenant_id: String,
) -> Result<ApiResponse<()>, String> {
    let bridge = bridge.read().await;

    // 使用 ClientBridge 的 switch_tenant 方法
    // 它会自动更新 TenantManager 和 Config
    match bridge.switch_tenant(&tenant_id).await {
        Ok(_) => Ok(ApiResponse::success(())),
        Err(e) => Ok(ApiResponse::error_with_code(
            ErrorCode::TenantNotFound,
            e.to_string(),
        )),
    }
}

/// 移除租户
#[tauri::command(rename_all = "snake_case")]
pub async fn remove_tenant(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    tenant_id: String,
) -> Result<ApiResponse<DeleteData>, String> {
    let bridge = bridge.read().await;
    let mut tenant_manager = bridge.tenant_manager().write().await;

    match tenant_manager.remove_tenant(&tenant_id) {
        Ok(_) => Ok(ApiResponse::success(DeleteData::success())),
        Err(e) => Ok(ApiResponse::error_with_code(
            ErrorCode::TenantNotFound,
            e.to_string(),
        )),
    }
}

/// 获取当前租户ID
#[tauri::command(rename_all = "snake_case")]
pub async fn get_current_tenant(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
) -> Result<ApiResponse<Option<String>>, String> {
    let bridge = bridge.read().await;
    let tenant_manager = bridge.tenant_manager().read().await;
    Ok(ApiResponse::success(
        tenant_manager.current_tenant_id().map(|s| s.to_string()),
    ))
}
