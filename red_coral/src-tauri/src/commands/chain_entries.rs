//! Chain Entries Commands
//!
//! 统一 hash 链时间线 — 代理到 edge-server REST API

use std::sync::Arc;
use tauri::State;
use urlencoding::encode;

use crate::core::{ApiResponse, ClientBridge};

#[derive(Debug, serde::Deserialize)]
pub struct FetchChainEntriesParams {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    /// Cursor: only return entries with chain_id < before (stable pagination)
    pub before: Option<i64>,
    pub search: Option<String>,
}

/// GET /api/chain-entries — 获取 chain_entry 时间线列表
#[tauri::command]
pub async fn fetch_chain_entries(
    bridge: State<'_, Arc<ClientBridge>>,
    params: FetchChainEntriesParams,
) -> Result<ApiResponse<serde_json::Value>, String> {
    let limit = params.limit.unwrap_or(50);
    let mut query = format!("/api/chain-entries?limit={}", limit);
    if let Some(before) = params.before {
        query.push_str(&format!("&before={}", before));
    } else {
        let offset = params.offset.unwrap_or(0);
        query.push_str(&format!("&offset={}", offset));
    }
    if let Some(search) = &params.search {
        if !search.is_empty() {
            query.push_str(&format!("&search={}", encode(search)));
        }
    }
    match bridge.get::<serde_json::Value>(&query).await {
        Ok(data) => Ok(ApiResponse::success(data)),
        Err(e) => Ok(ApiResponse::from_bridge_error(e)),
    }
}

/// GET /api/chain-entries/anulacion/:id — 获取发票作废详情
#[tauri::command]
pub async fn fetch_chain_anulacion_detail(
    bridge: State<'_, Arc<ClientBridge>>,
    id: i64,
) -> Result<ApiResponse<serde_json::Value>, String> {
    match bridge
        .get::<serde_json::Value>(&format!("/api/chain-entries/anulacion/{}", id))
        .await
    {
        Ok(data) => Ok(ApiResponse::success(data)),
        Err(e) => Ok(ApiResponse::from_bridge_error(e)),
    }
}

/// GET /api/anulacion/eligibility/:order_pk — 检查订单是否可作废
#[tauri::command]
pub async fn check_anulacion_eligibility(
    bridge: State<'_, Arc<ClientBridge>>,
    order_pk: i64,
) -> Result<ApiResponse<serde_json::Value>, String> {
    match bridge
        .get::<serde_json::Value>(&format!("/api/anulacion/eligibility/{}", order_pk))
        .await
    {
        Ok(data) => Ok(ApiResponse::success(data)),
        Err(e) => Ok(ApiResponse::from_bridge_error(e)),
    }
}

/// POST /api/anulacion — 创建发票作废
#[tauri::command]
pub async fn create_anulacion(
    bridge: State<'_, Arc<ClientBridge>>,
    request: serde_json::Value,
) -> Result<ApiResponse<serde_json::Value>, String> {
    match bridge
        .post::<serde_json::Value, _>("/api/anulacion", &request)
        .await
    {
        Ok(data) => Ok(ApiResponse::success(data)),
        Err(e) => Ok(ApiResponse::from_bridge_error(e)),
    }
}

/// GET /api/invoices/upgrade/eligibility/:order_pk — 检查订单是否可以升级
#[tauri::command]
pub async fn check_upgrade_eligibility(
    bridge: State<'_, Arc<ClientBridge>>,
    order_pk: i64,
) -> Result<ApiResponse<serde_json::Value>, String> {
    match bridge
        .get::<serde_json::Value>(&format!("/api/invoices/upgrade/eligibility/{}", order_pk))
        .await
    {
        Ok(data) => Ok(ApiResponse::success(data)),
        Err(e) => Ok(ApiResponse::from_bridge_error(e)),
    }
}

/// POST /api/invoices/upgrade — 创建 F3 sustitutiva
#[tauri::command]
pub async fn create_upgrade(
    bridge: State<'_, Arc<ClientBridge>>,
    request: serde_json::Value,
) -> Result<ApiResponse<serde_json::Value>, String> {
    match bridge
        .post::<serde_json::Value, _>("/api/invoices/upgrade", &request)
        .await
    {
        Ok(data) => Ok(ApiResponse::success(data)),
        Err(e) => Ok(ApiResponse::from_bridge_error(e)),
    }
}

/// GET /api/chain-entries/upgrade/:id — 获取 F3 升级详情
#[tauri::command]
pub async fn fetch_chain_upgrade_detail(
    bridge: State<'_, Arc<ClientBridge>>,
    id: i64,
) -> Result<ApiResponse<serde_json::Value>, String> {
    match bridge
        .get::<serde_json::Value>(&format!("/api/chain-entries/upgrade/{}", id))
        .await
    {
        Ok(data) => Ok(ApiResponse::success(data)),
        Err(e) => Ok(ApiResponse::from_bridge_error(e)),
    }
}

/// GET /api/chain-entries/credit-note/:id — 获取退款凭证详情（含 hash）
#[tauri::command]
pub async fn fetch_chain_credit_note_detail(
    bridge: State<'_, Arc<ClientBridge>>,
    id: i64,
) -> Result<ApiResponse<serde_json::Value>, String> {
    match bridge
        .get::<serde_json::Value>(&format!("/api/chain-entries/credit-note/{}", id))
        .await
    {
        Ok(data) => Ok(ApiResponse::success(data)),
        Err(e) => Ok(ApiResponse::from_bridge_error(e)),
    }
}
