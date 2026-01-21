//! 订单 API Commands
//!
//! 通过 ClientBridge -> CrabClient -> EdgeServer REST API

use std::sync::Arc;
use tauri::State;
use tokio::sync::RwLock;
use urlencoding::encode;

use crate::core::response::ErrorCode;
use crate::core::{ApiResponse, ClientBridge, FetchOrderListResponse, OrderListData};
use shared::models::Order;

#[derive(Debug, serde::Deserialize)]
pub struct FetchOrderListParams {
    pub page: i32,
    pub limit: i32,
    pub search: Option<String>,
    pub start_time: Option<u64>,
}

// ============ Order Queries ============

#[tauri::command(rename_all = "snake_case")]
pub async fn fetch_order_list(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    params: FetchOrderListParams,
) -> Result<ApiResponse<FetchOrderListResponse>, String> {
    let bridge = bridge.read().await;
    let offset = (params.page - 1) * params.limit;

    let mut query = format!("/api/orders?limit={}&offset={}", params.limit, offset);
    if let Some(search) = params.search {
        if !search.is_empty() {
            query.push_str(&format!("&search={}", encode(&search)));
        }
    }
    if let Some(start_time) = params.start_time {
        query.push_str(&format!("&start_time={}", start_time));
    }

    match bridge.get::<Vec<Order>>(&query).await {
        Ok(orders) => {
            // Mock total since API returns simple list
            let total = if orders.len() < params.limit as usize {
                offset as i64 + orders.len() as i64
            } else {
                offset as i64 + orders.len() as i64 + 1 // Assume more
            };
            Ok(ApiResponse::success(FetchOrderListResponse {
                orders,
                total,
                page: params.page,
            }))
        }
        Err(e) => Ok(ApiResponse::error_with_code(ErrorCode::DatabaseError, e.to_string())),
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
        Err(e) => Ok(ApiResponse::error_with_code(ErrorCode::DatabaseError, e.to_string())),
    }
}

#[tauri::command(rename_all = "snake_case")]
pub async fn list_open_orders(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
) -> Result<ApiResponse<OrderListData>, String> {
    let bridge = bridge.read().await;
    match bridge.get::<Vec<Order>>("/api/orders/open").await {
        Ok(orders) => Ok(ApiResponse::success(OrderListData { orders })),
        Err(e) => Ok(ApiResponse::error_with_code(ErrorCode::DatabaseError, e.to_string())),
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
        Err(e) => Ok(ApiResponse::error_with_code(ErrorCode::OrderNotFound, e.to_string())),
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
        Err(e) => Ok(ApiResponse::error_with_code(ErrorCode::OrderNotFound, e.to_string())),
    }
}

#[tauri::command(rename_all = "snake_case")]
pub async fn get_last_order(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
) -> Result<ApiResponse<Option<Order>>, String> {
    let bridge = bridge.read().await;
    match bridge.get::<Option<Order>>("/api/orders/last").await {
        Ok(order) => Ok(ApiResponse::success(order)),
        Err(e) => Ok(ApiResponse::error_with_code(ErrorCode::DatabaseError, e.to_string())),
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
        Err(e) => Ok(ApiResponse::error_with_code(ErrorCode::DatabaseError, e.to_string())),
    }
}

