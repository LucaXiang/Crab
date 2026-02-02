//! 员工认证命令

use std::sync::Arc;
use tauri::State;

use crate::core::response::ErrorCode;
use crate::core::session_cache::EmployeeSession;
use crate::core::{ApiResponse, AuthData, ClientBridge};

/// 统一登录命令 (使用 ClientBridge)
///
/// 根据当前模式自动选择登录方式：
/// - Server 模式: 使用 CrabClient 的 In-Process 登录
/// - Client 模式: 使用 mTLS HTTP 登录到远程 Edge Server
#[tauri::command]
pub async fn login_employee(
    bridge: State<'_, Arc<ClientBridge>>,
    username: String,
    password: String,
) -> Result<ApiResponse<AuthData>, String> {
    match bridge.login_employee(&username, &password).await {
        Ok(session) => Ok(ApiResponse::success(AuthData {
            mode: session.login_mode,
            session: Some(session),
        })),
        Err(e) => Ok(ApiResponse::error_with_code(
            ErrorCode::InvalidCredentials,
            e.to_string(),
        )),
    }
}

/// 登出 (使用 ClientBridge)
#[tauri::command]
pub async fn logout_employee(
    bridge: State<'_, Arc<ClientBridge>>,
) -> Result<ApiResponse<()>, String> {
    match bridge.logout_employee().await {
        Ok(_) => Ok(ApiResponse::success(())),
        Err(e) => Ok(ApiResponse::error_with_code(
            ErrorCode::InternalError,
            e.to_string(),
        )),
    }
}

/// 获取当前活动会话 (用于启动时恢复登录状态)
///
/// 返回从磁盘恢复的会话，如果没有缓存会话则返回 null
#[tauri::command]
pub async fn get_current_session(
    bridge: State<'_, Arc<ClientBridge>>,
) -> Result<ApiResponse<Option<EmployeeSession>>, String> {
    Ok(ApiResponse::success(bridge.get_current_session().await))
}
