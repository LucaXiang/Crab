//! 租户管理命令

use std::sync::Arc;
use tauri::State;

use crate::core::response::{ActivationResultData, ApiResponse, ErrorCode};
use crate::core::{AppState, ClientBridge};

/// 激活 Server 模式设备
///
/// 调用 /api/server/activate，保存 Server 证书到磁盘。
/// auth_url 从 AppConfig 全局配置获取，前端不再传递。
#[tauri::command]
pub async fn activate_server_tenant(
    bridge: State<'_, Arc<ClientBridge>>,
    username: String,
    password: String,
    replace_entity_id: Option<String>,
) -> Result<ApiResponse<ActivationResultData>, String> {
    let auth_url = bridge.get_auth_url().await;

    match bridge
        .handle_activation_with_replace(
            &auth_url,
            &username,
            &password,
            replace_entity_id.as_deref(),
        )
        .await
    {
        Ok((tenant_id, subscription_status)) => Ok(ApiResponse::success(ActivationResultData {
            tenant_id,
            subscription_status,
            quota_info: None,
        })),
        Err(crate::core::bridge::BridgeError::Tenant(
            crate::core::tenant_manager::TenantError::DeviceLimitReached(quota_info),
        )) => {
            let mut details = std::collections::HashMap::new();
            details.insert(
                "quota_info".to_string(),
                serde_json::to_value(&quota_info).unwrap_or_default(),
            );
            Ok(ApiResponse {
                code: Some(ErrorCode::DeviceLimitReached.code()),
                message: ErrorCode::DeviceLimitReached.message().to_string(),
                data: None,
                details: Some(details),
            })
        }
        Err(e) => Ok(ApiResponse::error_with_code(
            ErrorCode::ActivationFailed,
            e.to_string(),
        )),
    }
}

/// 激活 Client 模式设备
///
/// 调用 /api/client/activate，保存 Client 证书到磁盘。
#[tauri::command]
pub async fn activate_client_tenant(
    bridge: State<'_, Arc<ClientBridge>>,
    username: String,
    password: String,
    replace_entity_id: Option<String>,
) -> Result<ApiResponse<ActivationResultData>, String> {
    let auth_url = bridge.get_auth_url().await;

    match bridge
        .handle_client_activation_with_replace(
            &auth_url,
            &username,
            &password,
            replace_entity_id.as_deref(),
        )
        .await
    {
        Ok((tenant_id, subscription_status)) => Ok(ApiResponse::success(ActivationResultData {
            tenant_id,
            subscription_status,
            quota_info: None,
        })),
        Err(crate::core::bridge::BridgeError::Tenant(
            crate::core::tenant_manager::TenantError::ClientLimitReached(quota_info),
        )) => {
            let mut details = std::collections::HashMap::new();
            details.insert(
                "quota_info".to_string(),
                serde_json::to_value(&quota_info).unwrap_or_default(),
            );
            Ok(ApiResponse {
                code: Some(ErrorCode::ClientLimitReached.code()),
                message: ErrorCode::ClientLimitReached.message().to_string(),
                data: None,
                details: Some(details),
            })
        }
        Err(e) => Ok(ApiResponse::error_with_code(
            ErrorCode::ActivationFailed,
            e.to_string(),
        )),
    }
}

/// 验证租户凭据 (不签发证书，返回租户信息和配额)
#[tauri::command]
pub async fn verify_tenant(
    bridge: State<'_, Arc<ClientBridge>>,
    username: String,
    password: String,
) -> Result<ApiResponse<shared::activation::TenantVerifyData>, String> {
    match bridge.verify_tenant(&username, &password).await {
        Ok(data) => Ok(ApiResponse::success(data)),
        Err(e) => Ok(ApiResponse::error_with_code(
            ErrorCode::ActivationFailed,
            e.to_string(),
        )),
    }
}

/// 注销当前模式 (释放配额 + 删除本地证书)
#[tauri::command]
pub async fn deactivate_current_mode(
    bridge: State<'_, Arc<ClientBridge>>,
    username: String,
    password: String,
) -> Result<ApiResponse<()>, String> {
    match bridge.deactivate_current_mode(&username, &password).await {
        Ok(()) => Ok(ApiResponse::success(())),
        Err(e) => Ok(ApiResponse::error_with_code(
            ErrorCode::InternalError,
            e.to_string(),
        )),
    }
}

/// 退出当前租户（停止服务器 + 移除租户数据）
#[tauri::command]
pub async fn exit_tenant(
    bridge: State<'_, Arc<ClientBridge>>,
) -> Result<ApiResponse<()>, String> {
    match bridge.exit_tenant().await {
        Ok(_) => Ok(ApiResponse::success(())),
        Err(e) => Ok(ApiResponse::error_with_code(
            ErrorCode::InternalError,
            e.to_string(),
        )),
    }
}

/// 获取当前租户ID
#[tauri::command]
pub async fn get_current_tenant(
    bridge: State<'_, Arc<ClientBridge>>,
) -> Result<ApiResponse<Option<String>>, String> {
    let tenant_manager = bridge.tenant_manager().read().await;
    Ok(ApiResponse::success(
        tenant_manager.current_tenant_id().map(|s| s.to_string()),
    ))
}

/// 重新检查订阅状态
///
/// 从 auth-server 同步最新订阅信息，返回更新后的 AppState。
/// 用于 SubscriptionBlockedScreen 的"重新检查"按钮。
#[tauri::command]
pub async fn check_subscription(
    bridge: State<'_, Arc<ClientBridge>>,
) -> Result<ApiResponse<AppState>, String> {
    match bridge.check_subscription().await {
        Ok(state) => Ok(ApiResponse::success(state)),
        Err(e) => Ok(ApiResponse::error_with_code(
            ErrorCode::InternalError,
            e.to_string(),
        )),
    }
}
