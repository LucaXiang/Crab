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

    // 1. 激活租户 (TenantManager)
    let tenant_id = {
        let mut tenant_manager = bridge.tenant_manager().write().await;
        tenant_manager
            .activate_tenant(&auth_url, &username, &password)
            .await
            .map_err(|e| e.to_string())?
    };

    // 2. 预激活 edge-server (为 Server 模式准备)
    // 使用相同的凭证，这样 start_server_mode 时就已经有凭证了
    {
        let server_config = bridge.get_server_config().await;

        let edge_config = edge_server::Config::builder()
            .work_dir(server_config.data_dir.to_string_lossy().to_string())
            .http_port(server_config.http_port)
            .message_tcp_port(server_config.message_port)
            .auth_server_url(&auth_url)
            .build();

        // 临时初始化 ServerState 来获取 provisioning_service
        let server_state = edge_server::ServerState::initialize(&edge_config).await;

        // 调用 provisioning_service 预激活
        let service = server_state.provisioning_service(auth_url.clone());
        if let Err(e) = service.activate(&username, &password).await {
            tracing::warn!("Failed to pre-activate edge-server: {}", e);
            // 不返回错误，租户激活已成功，只是预激活失败
            // 用户可以在 start_server_mode 时手动激活
        } else {
            tracing::info!("Edge-server pre-activated successfully");
        }

        // ServerState 会被丢弃，但凭证已保存到磁盘
    }

    Ok(tenant_id)
}

/// 切换当前租户
#[tauri::command]
pub async fn switch_tenant(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    tenant_id: String,
) -> Result<(), String> {
    let bridge = bridge.read().await;
    let mut tenant_manager = bridge.tenant_manager().write().await;

    tenant_manager
        .switch_tenant(&tenant_id)
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
