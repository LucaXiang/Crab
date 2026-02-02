//! Sync Commands
//!
//! 提供同步状态查询的 Tauri 命令接口

use std::sync::Arc;
use tauri::State;

use crate::core::response::{ApiResponse, ErrorCode};
use crate::core::ClientBridge;

/// 同步状态响应
#[derive(serde::Serialize, serde::Deserialize)]
pub struct SyncStatusResponse {
    pub epoch: String,
    pub versions: std::collections::HashMap<String, u64>,
}

/// 获取同步状态
#[tauri::command]
pub async fn get_sync_status(
    bridge: State<'_, Arc<ClientBridge>>,
) -> Result<ApiResponse<SyncStatusResponse>, String> {

    match bridge.get::<SyncStatusResponse>("/api/sync/status").await {
        Ok(data) => Ok(ApiResponse::success(data)),
        Err(e) => Ok(ApiResponse::error_with_code(
            ErrorCode::NetworkError,
            e.to_string(),
        )),
    }
}
