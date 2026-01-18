//! 员工认证命令

use std::sync::Arc;
use tauri::State;
use tokio::sync::RwLock;

use crate::core::{ClientBridge, EmployeeSession, LoginMode};

/// 在线登录响应
#[derive(Debug, Clone, serde::Serialize)]
pub struct LoginResponse {
    pub success: bool,
    pub session: Option<EmployeeSession>,
    pub error: Option<String>,
    pub mode: LoginMode,
}

/// 员工在线登录
#[tauri::command]
pub async fn login_online(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    username: String,
    password: String,
    edge_url: String,
) -> Result<LoginResponse, String> {
    let bridge = bridge.read().await;
    let mut tenant_manager = bridge.tenant_manager().write().await;

    match tenant_manager.login_online(&username, &password, &edge_url).await {
        Ok(session) => Ok(LoginResponse {
            success: true,
            session: Some(session),
            error: None,
            mode: LoginMode::Online,
        }),
        Err(e) => Ok(LoginResponse {
            success: false,
            session: None,
            error: Some(e.to_string()),
            mode: LoginMode::Online,
        }),
    }
}

/// 员工离线登录
#[tauri::command]
pub async fn login_offline(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    username: String,
    password: String,
) -> Result<LoginResponse, String> {
    let bridge = bridge.read().await;
    let mut tenant_manager = bridge.tenant_manager().write().await;

    match tenant_manager.login_offline(&username, &password) {
        Ok(session) => Ok(LoginResponse {
            success: true,
            session: Some(session),
            error: None,
            mode: LoginMode::Offline,
        }),
        Err(e) => Ok(LoginResponse {
            success: false,
            session: None,
            error: Some(e.to_string()),
            mode: LoginMode::Offline,
        }),
    }
}

/// 员工自动登录 (优先在线，失败则尝试离线)
#[tauri::command]
pub async fn login_auto(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    username: String,
    password: String,
    edge_url: String,
) -> Result<LoginResponse, String> {
    let bridge = bridge.read().await;
    let mut tenant_manager = bridge.tenant_manager().write().await;

    match tenant_manager.login_auto(&username, &password, &edge_url).await {
        Ok(session) => Ok(LoginResponse {
            success: true,
            mode: session.login_mode,
            session: Some(session),
            error: None,
        }),
        Err(e) => Ok(LoginResponse {
            success: false,
            session: None,
            error: Some(e.to_string()),
            mode: LoginMode::Offline,
        }),
    }
}

/// 登出
#[tauri::command]
pub async fn logout(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
) -> Result<(), String> {
    let bridge = bridge.read().await;
    let mut tenant_manager = bridge.tenant_manager().write().await;
    tenant_manager.logout().map_err(|e| e.to_string())
}

/// 获取当前会话
#[tauri::command]
pub async fn get_current_session(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
) -> Result<Option<EmployeeSession>, String> {
    let bridge = bridge.read().await;
    let tenant_manager = bridge.tenant_manager().read().await;
    Ok(tenant_manager.current_session().cloned())
}

/// 检查是否有离线登录缓存
#[tauri::command]
pub async fn has_offline_cache(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    username: String,
) -> Result<bool, String> {
    let bridge = bridge.read().await;
    let tenant_manager = bridge.tenant_manager().read().await;
    Ok(tenant_manager.has_offline_cache(&username))
}

/// 获取缓存的员工列表 (用于离线登录选择)
#[tauri::command]
pub async fn list_cached_employees(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
) -> Result<Vec<String>, String> {
    let bridge = bridge.read().await;
    let tenant_manager = bridge.tenant_manager().read().await;
    Ok(tenant_manager.list_cached_employees())
}

/// 统一登录命令 (使用 ClientBridge)
///
/// 根据当前模式自动选择登录方式：
/// - Server 模式: 使用 CrabClient 的 In-Process 登录
/// - Client 模式: 使用 mTLS HTTP 登录到远程 Edge Server
#[tauri::command]
pub async fn login_employee(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    username: String,
    password: String,
) -> Result<LoginResponse, String> {
    let bridge = bridge.read().await;

    match bridge.login_employee(&username, &password).await {
        Ok(session) => Ok(LoginResponse {
            success: true,
            mode: session.login_mode,
            session: Some(session),
            error: None,
        }),
        Err(e) => Ok(LoginResponse {
            success: false,
            session: None,
            error: Some(e.to_string()),
            mode: LoginMode::Offline, // default for error case
        }),
    }
}

/// 登出 (使用 ClientBridge)
#[tauri::command]
pub async fn logout_employee(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
) -> Result<(), String> {
    let bridge = bridge.read().await;
    bridge.logout_employee().await.map_err(|e| e.to_string())
}
