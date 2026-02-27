//! Credit Notes Commands
//!
//! 退款凭证 CRUD — 代理到 edge-server REST API

use std::sync::Arc;
use tauri::State;

use crate::core::{ApiResponse, ClientBridge};

/// POST /api/credit-notes - 创建退款凭证
#[tauri::command]
pub async fn create_credit_note(
    bridge: State<'_, Arc<ClientBridge>>,
    request: serde_json::Value,
) -> Result<ApiResponse<serde_json::Value>, String> {
    match bridge
        .post::<serde_json::Value, _>("/api/credit-notes", &request)
        .await
    {
        Ok(detail) => Ok(ApiResponse::success(detail)),
        Err(e) => Ok(ApiResponse::from_bridge_error(e)),
    }
}

/// GET /api/credit-notes/:id - 获取退款凭证详情
#[tauri::command]
pub async fn fetch_credit_note_detail(
    bridge: State<'_, Arc<ClientBridge>>,
    id: i64,
) -> Result<ApiResponse<serde_json::Value>, String> {
    match bridge
        .get::<serde_json::Value>(&format!("/api/credit-notes/{}", id))
        .await
    {
        Ok(detail) => Ok(ApiResponse::success(detail)),
        Err(e) => Ok(ApiResponse::from_bridge_error(e)),
    }
}

/// GET /api/credit-notes/by-order/:order_pk - 获取订单的所有退款记录
#[tauri::command]
pub async fn fetch_credit_notes_by_order(
    bridge: State<'_, Arc<ClientBridge>>,
    order_pk: i64,
) -> Result<ApiResponse<serde_json::Value>, String> {
    match bridge
        .get::<serde_json::Value>(&format!("/api/credit-notes/by-order/{}", order_pk))
        .await
    {
        Ok(notes) => Ok(ApiResponse::success(notes)),
        Err(e) => Ok(ApiResponse::from_bridge_error(e)),
    }
}

/// GET /api/credit-notes/refundable/:order_pk - 获取可退金额信息
#[tauri::command]
pub async fn fetch_refundable_info(
    bridge: State<'_, Arc<ClientBridge>>,
    order_pk: i64,
) -> Result<ApiResponse<serde_json::Value>, String> {
    match bridge
        .get::<serde_json::Value>(&format!("/api/credit-notes/refundable/{}", order_pk))
        .await
    {
        Ok(info) => Ok(ApiResponse::success(info)),
        Err(e) => Ok(ApiResponse::from_bridge_error(e)),
    }
}
