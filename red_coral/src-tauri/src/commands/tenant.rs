//! 租户管理命令

use std::sync::Arc;
use tauri::State;
use tokio::sync::RwLock;

use crate::core::{ClientBridge, TenantInfo};

/// 获取已激活的租户列表
#[tauri::command]
pub async fn list_tenants(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
) -> Result<Vec<TenantInfo>, String> {
    let bridge = bridge.read().await;
    let tenant_manager = bridge.tenant_manager().read().await;
    Ok(tenant_manager.list_tenants())
}

/// 激活新租户 (设备激活)
///
/// 同时预激活 edge-server，为 Server 模式做准备
#[tauri::command]
pub async fn activate_tenant(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    auth_url: String,
    username: String,
    password: String,
) -> Result<String, String> {
    let bridge = bridge.read().await;

    // 使用 ClientBridge 的 handle_activation 方法
    // 它会自动调用 TenantManager 激活，并更新配置 (current_tenant)
    bridge
        .handle_activation(&auth_url, &username, &password)
        .await
        .map_err(|e| e.to_string())
}

/// 切换当前租户
#[tauri::command]
pub async fn switch_tenant(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    tenant_id: String,
) -> Result<(), String> {
    let bridge = bridge.read().await;

    // 使用 ClientBridge 的 switch_tenant 方法
    // 它会自动更新 TenantManager 和 Config
    bridge
        .switch_tenant(&tenant_id)
        .await
        .map_err(|e| e.to_string())
}

/// 移除租户
#[tauri::command]
pub async fn remove_tenant(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    tenant_id: String,
) -> Result<(), String> {
    let bridge = bridge.read().await;
    let mut tenant_manager = bridge.tenant_manager().write().await;

    tenant_manager
        .remove_tenant(&tenant_id)
        .map_err(|e| e.to_string())
}

/// 获取当前租户ID
#[tauri::command]
pub async fn get_current_tenant(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
) -> Result<Option<String>, String> {
    let bridge = bridge.read().await;
    let tenant_manager = bridge.tenant_manager().read().await;
    Ok(tenant_manager.current_tenant_id().map(|s| s.to_string()))
}
