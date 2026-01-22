//! 通用 API Commands
//!
//! 提供通用的 HTTP 方法，用于前端调用尚未封装的 API

use std::sync::Arc;
use tauri::State;
use tokio::sync::RwLock;

use crate::core::response::{ApiResponse, ErrorCode};
use crate::core::ClientBridge;

/// 通用 GET 请求
#[tauri::command(rename_all = "snake_case")]
pub async fn api_get(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    path: String,
) -> Result<ApiResponse<serde_json::Value>, String> {
    let bridge = bridge.read().await;
    match bridge.get(&path).await {
        Ok(data) => Ok(ApiResponse::success(data)),
        Err(e) => Ok(ApiResponse::error_with_code(
            ErrorCode::NetworkError,
            e.to_string(),
        )),
    }
}

/// 通用 POST 请求
#[tauri::command(rename_all = "snake_case")]
pub async fn api_post(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    path: String,
    body: serde_json::Value,
) -> Result<ApiResponse<serde_json::Value>, String> {
    let bridge = bridge.read().await;
    match bridge.post(&path, &body).await {
        Ok(data) => Ok(ApiResponse::success(data)),
        Err(e) => Ok(ApiResponse::error_with_code(
            ErrorCode::NetworkError,
            e.to_string(),
        )),
    }
}

/// 通用 PUT 请求
#[tauri::command(rename_all = "snake_case")]
pub async fn api_put(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    path: String,
    body: serde_json::Value,
) -> Result<ApiResponse<serde_json::Value>, String> {
    let bridge = bridge.read().await;
    match bridge.put(&path, &body).await {
        Ok(data) => Ok(ApiResponse::success(data)),
        Err(e) => Ok(ApiResponse::error_with_code(
            ErrorCode::NetworkError,
            e.to_string(),
        )),
    }
}

/// 通用 DELETE 请求
#[tauri::command(rename_all = "snake_case")]
pub async fn api_delete(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    path: String,
) -> Result<ApiResponse<serde_json::Value>, String> {
    let bridge = bridge.read().await;
    match bridge.delete(&path).await {
        Ok(data) => Ok(ApiResponse::success(data)),
        Err(e) => Ok(ApiResponse::error_with_code(
            ErrorCode::NetworkError,
            e.to_string(),
        )),
    }
}
