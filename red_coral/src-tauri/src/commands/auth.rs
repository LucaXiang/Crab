//! 员工认证命令

use std::sync::Arc;
use serde::Serialize;
use tauri::State;

use crate::core::response::ErrorCode;
use crate::core::session_cache::EmployeeSession;
use crate::core::{ApiResponse, AuthData, ClientBridge};
use shared::client::{EscalateResponse, UserInfo};

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

/// 权限提升 (主管授权)
///
/// 验证授权人凭据并检查权限，成功时记录审计日志
#[tauri::command]
pub async fn escalate_permission(
    bridge: State<'_, Arc<ClientBridge>>,
    username: String,
    password: String,
    required_permission: String,
) -> Result<ApiResponse<UserInfo>, String> {
    #[derive(Serialize)]
    struct EscalateReq {
        username: String,
        password: String,
        required_permission: String,
    }

    let request = EscalateReq {
        username,
        password,
        required_permission,
    };

    match bridge.post::<EscalateResponse, _>("/api/auth/escalate", &request).await {
        Ok(response) => Ok(ApiResponse::success(response.authorizer)),
        Err(e) => {
            let error_msg = e.to_string();
            // 区分权限不足和凭据错误
            if error_msg.contains("permission") {
                Ok(ApiResponse::error_with_code(ErrorCode::PermissionDenied, error_msg))
            } else {
                Ok(ApiResponse::error_with_code(ErrorCode::InvalidCredentials, error_msg))
            }
        }
    }
}
