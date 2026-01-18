//! 通用 API Commands
//!
//! 提供通用的 HTTP 方法，用于前端调用尚未封装的 API

use std::sync::Arc;
use tauri::State;
use tokio::sync::RwLock;

use crate::core::ClientBridge;

/// 通用 GET 请求
#[tauri::command]
pub async fn api_get(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    path: String,
) -> Result<serde_json::Value, String> {
    let bridge = bridge.read().await;
    bridge.get(&path).await.map_err(|e| e.to_string())
}

/// 通用 POST 请求
#[tauri::command]
pub async fn api_post(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    path: String,
    body: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let bridge = bridge.read().await;
    bridge.post(&path, &body).await.map_err(|e| e.to_string())
}

/// 通用 PUT 请求
#[tauri::command]
pub async fn api_put(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    path: String,
    body: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let bridge = bridge.read().await;
    bridge.put(&path, &body).await.map_err(|e| e.to_string())
}

/// 通用 DELETE 请求
#[tauri::command]
pub async fn api_delete(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    path: String,
) -> Result<serde_json::Value, String> {
    let bridge = bridge.read().await;
    bridge.delete(&path).await.map_err(|e| e.to_string())
}
