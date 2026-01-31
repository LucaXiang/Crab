//! 订单 API Commands
//!
//! 通过 ClientBridge -> CrabClient -> EdgeServer REST API

use std::sync::Arc;
use tauri::State;
use tokio::sync::RwLock;
use urlencoding::encode;

use crate::core::{ApiResponse, ClientBridge};

#[derive(Debug, serde::Deserialize)]
pub struct FetchOrderListParams {
    pub page: i32,
    pub limit: i32,
    pub search: Option<String>,
    pub start_time: Option<u64>,
}

// ============ Order History (Archived) ============

/// Order summary for list view (matches backend OrderSummary)
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct OrderSummary {
    pub order_id: String,
    pub receipt_number: String,
    pub table_name: Option<String>,
    pub status: String,
    pub is_retail: bool,
    pub total: f64,
    pub guest_count: i32,
    pub start_time: i64,
    pub end_time: Option<i64>,
    // === Void Metadata ===
    pub void_type: Option<String>,
    pub loss_reason: Option<String>,
    pub loss_amount: Option<f64>,
}

/// Backend response for paginated order list
#[derive(Debug, serde::Deserialize)]
#[allow(dead_code)] // limit field required for deserialization but not used
struct OrderListApiResponse {
    orders: Vec<OrderSummary>,
    total: i64,
    page: i32,
    limit: i32,
}

/// Fetch order list response with summaries (for frontend)
#[derive(Debug, serde::Serialize)]
pub struct FetchOrderListSummaryResponse {
    pub orders: Vec<OrderSummary>,
    pub total: i64,
    pub page: i32,
}

#[tauri::command]
pub async fn fetch_order_list(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    params: FetchOrderListParams,
) -> Result<ApiResponse<FetchOrderListSummaryResponse>, String> {
    let bridge = bridge.read().await;

    // Calculate time range in UTC millis (default: last 7 days)
    let now_millis = chrono::Utc::now().timestamp_millis();
    let start_millis = params.start_time
        .map(|t| t as i64)
        .unwrap_or_else(|| now_millis - 7 * 24 * 60 * 60 * 1000);
    let end_millis = now_millis;

    // Calculate offset from page number (1-indexed)
    let offset = (params.page.max(1) - 1) * params.limit;

    // Build query with optional search parameter
    let mut query = format!(
        "/api/orders/history?start_time={}&end_time={}&limit={}&offset={}",
        start_millis,
        end_millis,
        params.limit,
        offset
    );
    if let Some(search) = &params.search {
        if !search.is_empty() {
            query.push_str(&format!("&search={}", encode(search)));
        }
    }

    match bridge.get::<OrderListApiResponse>(&query).await {
        Ok(response) => {
            Ok(ApiResponse::success(FetchOrderListSummaryResponse {
                orders: response.orders,
                total: response.total,
                page: response.page,
            }))
        }
        Err(e) => Ok(ApiResponse::from_bridge_error(e)),
    }
}

/// Fetch archived order detail by ID (graph model)
/// Uses serde_json::Value to transparently pass through all fields from edge-server
#[tauri::command]
pub async fn fetch_order_detail(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    order_id: String,
) -> Result<ApiResponse<serde_json::Value>, String> {
    let bridge = bridge.read().await;
    match bridge
        .get::<serde_json::Value>(&format!("/api/orders/{}", encode(&order_id)))
        .await
    {
        Ok(detail) => Ok(ApiResponse::success(detail)),
        Err(e) => Ok(ApiResponse::from_bridge_error(e)),
    }
}
