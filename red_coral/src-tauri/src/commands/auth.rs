//! 员工认证命令

use std::sync::Arc;
use tauri::State;
use tokio::sync::RwLock;

use crate::core::{ApiResponse, AuthData, ClientBridge};
use crate::core::response::ErrorCode;

/// 统一登录命令 (使用 ClientBridge)
///
/// 根据当前模式自动选择登录方式：
/// - Server 模式: 使用 CrabClient 的 In-Process 登录
/// - Client 模式: 使用 mTLS HTTP 登录到远程 Edge Server
#[tauri::command(rename_all = "snake_case")]
pub async fn login_employee(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    username: String,
    password: String,
) -> Result<ApiResponse<AuthData>, String> {
    let bridge = bridge.read().await;

    match bridge.login_employee(&username, &password).await {
        Ok(session) => Ok(ApiResponse::success(AuthData {
            mode: session.login_mode,
            session: Some(session),
        })),
        Err(e) => Ok(ApiResponse::error_with_code(ErrorCode::InvalidCredentials, e.to_string())),
    }
}

/// 登出 (使用 ClientBridge)
#[tauri::command(rename_all = "snake_case")]
pub async fn logout_employee(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
) -> Result<ApiResponse<()>, String> {
    let bridge = bridge.read().await;
    match bridge.logout_employee().await {
        Ok(_) => Ok(ApiResponse::success(())),
        Err(e) => Ok(ApiResponse::error_with_code(ErrorCode::InternalError, e.to_string())),
    }
}
