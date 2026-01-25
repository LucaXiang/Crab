//! 订单 API Commands
//!
//! 通过 ClientBridge -> CrabClient -> EdgeServer REST API

use std::sync::Arc;
use tauri::State;
use tokio::sync::RwLock;
use urlencoding::encode;

use crate::core::{ApiResponse, ClientBridge, OrderListData};
use shared::models::Order;

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
}

/// Split item in a payment
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct SplitItem {
    pub instance_id: String,
    pub name: String,
    pub quantity: i32,
}

/// Order item option for detail view
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct OrderItemOptionDetail {
    pub attribute_name: String,
    pub option_name: String,
    pub price_modifier: f64,
}

/// Order item for detail view
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct OrderItemDetail {
    pub id: String,
    pub instance_id: String,
    pub name: String,
    pub spec_name: Option<String>,
    pub price: f64,
    pub quantity: i32,
    pub unpaid_quantity: i32,
    pub unit_price: f64,
    pub line_total: f64,
    pub discount_amount: f64,
    pub surcharge_amount: f64,
    pub note: Option<String>,
    pub selected_options: Vec<OrderItemOptionDetail>,
}

/// Payment for detail view
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct OrderPaymentDetail {
    pub method: String,
    pub amount: f64,
    pub timestamp: i64,
    pub note: Option<String>,
    pub cancelled: bool,
    pub cancel_reason: Option<String>,
    pub split_items: Vec<SplitItem>,
}

/// Event for detail view
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct OrderEventDetail {
    pub event_id: String,
    pub event_type: String,
    pub timestamp: i64,
    pub payload: Option<serde_json::Value>,
}

/// Full order detail (matches backend OrderDetail)
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct OrderDetail {
    pub order_id: String,
    pub receipt_number: String,
    pub table_name: Option<String>,
    pub zone_name: Option<String>,
    pub status: String,
    pub is_retail: bool,
    pub guest_count: i32,
    pub total: f64,
    pub paid_amount: f64,
    pub total_discount: f64,
    pub total_surcharge: f64,
    pub start_time: i64,
    pub end_time: Option<i64>,
    pub operator_name: Option<String>,
    pub items: Vec<OrderItemDetail>,
    pub payments: Vec<OrderPaymentDetail>,
    pub timeline: Vec<OrderEventDetail>,
}

/// Backend response for paginated order list
#[derive(Debug, serde::Deserialize)]
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

#[tauri::command(rename_all = "snake_case")]
pub async fn fetch_order_list(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    params: FetchOrderListParams,
) -> Result<ApiResponse<FetchOrderListSummaryResponse>, String> {
    let bridge = bridge.read().await;

    // Calculate date range (default: last 7 days)
    let end_date = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let start_date = if let Some(start_time) = params.start_time {
        // Convert timestamp to date string
        chrono::DateTime::from_timestamp_millis(start_time as i64)
            .map(|dt| dt.format("%Y-%m-%d").to_string())
            .unwrap_or_else(|| {
                (chrono::Utc::now() - chrono::Duration::days(7))
                    .format("%Y-%m-%d")
                    .to_string()
            })
    } else {
        (chrono::Utc::now() - chrono::Duration::days(7))
            .format("%Y-%m-%d")
            .to_string()
    };

    // Calculate offset from page number (1-indexed)
    let offset = (params.page.max(1) - 1) * params.limit;

    // Build query with optional search parameter
    let mut query = format!(
        "/api/orders/history?start_date={}&end_date={}&limit={}&offset={}",
        encode(&start_date),
        encode(&end_date),
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
#[tauri::command(rename_all = "snake_case")]
pub async fn fetch_order_detail(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    order_id: String,
) -> Result<ApiResponse<OrderDetail>, String> {
    let bridge = bridge.read().await;
    match bridge
        .get::<OrderDetail>(&format!("/api/orders/{}", encode(&order_id)))
        .await
    {
        Ok(detail) => Ok(ApiResponse::success(detail)),
        Err(e) => Ok(ApiResponse::from_bridge_error(e)),
    }
}

#[tauri::command(rename_all = "snake_case")]
pub async fn list_orders(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    limit: Option<i32>,
    offset: Option<i32>,
) -> Result<ApiResponse<OrderListData>, String> {
    let bridge = bridge.read().await;
    let limit = limit.unwrap_or(50);
    let offset = offset.unwrap_or(0);
    match bridge
        .get::<Vec<Order>>(&format!("/api/orders?limit={}&offset={}", limit, offset))
        .await
    {
        Ok(orders) => Ok(ApiResponse::success(OrderListData { orders })),
        Err(e) => Ok(ApiResponse::from_bridge_error(e)),
    }
}

#[tauri::command(rename_all = "snake_case")]
pub async fn list_open_orders(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
) -> Result<ApiResponse<OrderListData>, String> {
    let bridge = bridge.read().await;
    match bridge.get::<Vec<Order>>("/api/orders/open").await {
        Ok(orders) => Ok(ApiResponse::success(OrderListData { orders })),
        Err(e) => Ok(ApiResponse::from_bridge_error(e)),
    }
}

#[tauri::command(rename_all = "snake_case")]
pub async fn get_order(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    id: String,
) -> Result<ApiResponse<Order>, String> {
    let bridge = bridge.read().await;
    match bridge
        .get::<Order>(&format!("/api/orders/{}", encode(&id)))
        .await
    {
        Ok(order) => Ok(ApiResponse::success(order)),
        Err(e) => Ok(ApiResponse::from_bridge_error(e)),
    }
}

#[tauri::command(rename_all = "snake_case")]
pub async fn get_order_by_receipt(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    receipt: String,
) -> Result<ApiResponse<Order>, String> {
    let bridge = bridge.read().await;
    match bridge
        .get::<Order>(&format!("/api/orders/receipt/{}", encode(&receipt)))
        .await
    {
        Ok(order) => Ok(ApiResponse::success(order)),
        Err(e) => Ok(ApiResponse::from_bridge_error(e)),
    }
}

#[tauri::command(rename_all = "snake_case")]
pub async fn get_last_order(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
) -> Result<ApiResponse<Option<Order>>, String> {
    let bridge = bridge.read().await;
    match bridge.get::<Option<Order>>("/api/orders/last").await {
        Ok(order) => Ok(ApiResponse::success(order)),
        Err(e) => Ok(ApiResponse::from_bridge_error(e)),
    }
}

#[tauri::command(rename_all = "snake_case")]
pub async fn verify_order_chain(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    from_hash: Option<String>,
) -> Result<ApiResponse<bool>, String> {
    let bridge = bridge.read().await;
    let query = match from_hash {
        Some(hash) => format!("/api/orders/verify?from_hash={}", encode(&hash)),
        None => "/api/orders/verify".to_string(),
    };
    match bridge.get::<bool>(&query).await {
        Ok(valid) => Ok(ApiResponse::success(valid)),
        Err(e) => Ok(ApiResponse::from_bridge_error(e)),
    }
}
