use std::sync::Arc;
use tauri::State;

use crate::core::bridge::ClientBridge;
use crate::core::ApiResponse;

#[tauri::command]
pub async fn export_data(
    bridge: State<'_, Arc<ClientBridge>>,
    path: String,
) -> Result<ApiResponse<()>, String> {
    // Server mode: call edge-server export directly (in-process, zero network)
    if let Some(server_state) = bridge.get_server_state().await {
        let zip_bytes = edge_server::api::data_transfer::export_zip(&server_state)
            .await
            .map_err(|e| e.to_string())?;
        tokio::fs::write(&path, &zip_bytes)
            .await
            .map_err(|e| format!("Failed to write file: {e}"))?;
        return Ok(ApiResponse::success(()));
    }

    // Client mode: GET /api/data-transfer/export via mTLS
    let (edge_url, http_client, token) = bridge
        .get_edge_http_context()
        .await
        .ok_or_else(|| "Not connected to edge server".to_string())?;

    let resp = http_client
        .get(format!("{edge_url}/api/data-transfer/export"))
        .bearer_auth(&token)
        .send()
        .await
        .map_err(|e| format!("Export request failed: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("Export failed with status: {}", resp.status()));
    }

    let zip_bytes = resp
        .bytes()
        .await
        .map_err(|e| format!("Failed to read response: {e}"))?;

    tokio::fs::write(&path, &zip_bytes)
        .await
        .map_err(|e| format!("Failed to write file: {e}"))?;

    Ok(ApiResponse::success(()))
}

#[tauri::command]
pub async fn import_data(
    bridge: State<'_, Arc<ClientBridge>>,
    path: String,
) -> Result<ApiResponse<()>, String> {
    let zip_bytes = tokio::fs::read(&path)
        .await
        .map_err(|e| format!("Failed to read file: {e}"))?;

    // Server mode: call edge-server import directly (in-process, zero network)
    if let Some(server_state) = bridge.get_server_state().await {
        edge_server::api::data_transfer::import_zip(&server_state, &zip_bytes)
            .await
            .map_err(|e| e.to_string())?;
        return Ok(ApiResponse::success(()));
    }

    // Client mode: POST /api/data-transfer/import via mTLS
    let (edge_url, http_client, token) = bridge
        .get_edge_http_context()
        .await
        .ok_or_else(|| "Not connected to edge server".to_string())?;

    let resp = http_client
        .post(format!("{edge_url}/api/data-transfer/import"))
        .bearer_auth(&token)
        .header("content-type", "application/zip")
        .body(zip_bytes)
        .send()
        .await
        .map_err(|e| format!("Import request failed: {e}"))?;

    if !resp.status().is_success() {
        let body = resp
            .text()
            .await
            .unwrap_or_else(|_| "unknown error".to_string());
        return Err(format!("Import failed: {body}"));
    }

    Ok(ApiResponse::success(()))
}
