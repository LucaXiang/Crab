//! 订单 API Commands
//!
//! 通过 ClientBridge -> CrabClient -> EdgeServer REST API

use std::sync::Arc;
use tauri::State;
use tokio::sync::RwLock;
use urlencoding::encode;

use crate::core::{ApiResponse, ClientBridge, FetchOrderListResponse, OrderListData};
use shared::models::{
    Order, OrderAddEvent, OrderAddItem, OrderAddPayment, OrderCreate, OrderEvent, OrderRemoveItem,
    OrderUpdateHash, OrderUpdateStatus, OrderUpdateTotals,
};

#[derive(Debug, serde::Deserialize)]
pub struct FetchOrderListParams {
    pub page: i32,
    pub limit: i32,
    pub search: Option<String>,
    pub start_time: Option<u64>,
}

// ============ Order Queries ============

#[tauri::command]
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
        Err(e) => Ok(ApiResponse::error("FETCH_ORDER_LIST_FAILED", e.to_string())),
    }
}

#[tauri::command]
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
        Err(e) => Ok(ApiResponse::error("ORDER_LIST_FAILED", e.to_string())),
    }
}

#[tauri::command]
pub async fn list_open_orders(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
) -> Result<ApiResponse<OrderListData>, String> {
    let bridge = bridge.read().await;
    match bridge.get::<Vec<Order>>("/api/orders/open").await {
        Ok(orders) => Ok(ApiResponse::success(OrderListData { orders })),
        Err(e) => Ok(ApiResponse::error("ORDER_LIST_OPEN_FAILED", e.to_string())),
    }
}

#[tauri::command]
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
        Err(e) => Ok(ApiResponse::error("ORDER_GET_FAILED", e.to_string())),
    }
}

#[tauri::command]
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
        Err(e) => Ok(ApiResponse::error(
            "ORDER_GET_BY_RECEIPT_FAILED",
            e.to_string(),
        )),
    }
}

#[tauri::command]
pub async fn get_last_order(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
) -> Result<ApiResponse<Option<Order>>, String> {
    let bridge = bridge.read().await;
    match bridge.get::<Option<Order>>("/api/orders/last").await {
        Ok(order) => Ok(ApiResponse::success(order)),
        Err(e) => Ok(ApiResponse::error("ORDER_GET_LAST_FAILED", e.to_string())),
    }
}

#[tauri::command]
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
        Err(e) => Ok(ApiResponse::error("ORDER_VERIFY_FAILED", e.to_string())),
    }
}

// ============ Order Mutations ============

#[tauri::command]
pub async fn create_order(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    data: OrderCreate,
) -> Result<ApiResponse<Order>, String> {
    let bridge = bridge.read().await;
    match bridge.post::<Order, _>("/api/orders", &data).await {
        Ok(order) => Ok(ApiResponse::success(order)),
        Err(e) => Ok(ApiResponse::error("ORDER_CREATE_FAILED", e.to_string())),
    }
}

#[tauri::command]
pub async fn add_order_item(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    order_id: String,
    item: OrderAddItem,
) -> Result<ApiResponse<Order>, String> {
    let bridge = bridge.read().await;
    match bridge
        .post::<Order, _>(&format!("/api/orders/{}/items", encode(&order_id)), &item)
        .await
    {
        Ok(order) => Ok(ApiResponse::success(order)),
        Err(e) => Ok(ApiResponse::error("ORDER_ADD_ITEM_FAILED", e.to_string())),
    }
}

#[tauri::command]
pub async fn remove_order_item(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    order_id: String,
    data: OrderRemoveItem,
) -> Result<ApiResponse<Order>, String> {
    let bridge = bridge.read().await;
    match bridge
        .delete_with_body::<Order, _>(&format!("/api/orders/{}/items", encode(&order_id)), &data)
        .await
    {
        Ok(order) => Ok(ApiResponse::success(order)),
        Err(e) => Ok(ApiResponse::error(
            "ORDER_REMOVE_ITEM_FAILED",
            e.to_string(),
        )),
    }
}

#[tauri::command]
pub async fn add_order_payment(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    order_id: String,
    payment: OrderAddPayment,
) -> Result<ApiResponse<Order>, String> {
    let bridge = bridge.read().await;
    match bridge
        .post::<Order, _>(
            &format!("/api/orders/{}/payments", encode(&order_id)),
            &payment,
        )
        .await
    {
        Ok(order) => Ok(ApiResponse::success(order)),
        Err(e) => Ok(ApiResponse::error(
            "ORDER_ADD_PAYMENT_FAILED",
            e.to_string(),
        )),
    }
}

#[tauri::command]
pub async fn update_order_totals(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    order_id: String,
    data: OrderUpdateTotals,
) -> Result<ApiResponse<Order>, String> {
    let bridge = bridge.read().await;
    match bridge
        .put::<Order, _>(&format!("/api/orders/{}/totals", encode(&order_id)), &data)
        .await
    {
        Ok(order) => Ok(ApiResponse::success(order)),
        Err(e) => Ok(ApiResponse::error(
            "ORDER_UPDATE_TOTALS_FAILED",
            e.to_string(),
        )),
    }
}

#[tauri::command]
pub async fn update_order_status(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    order_id: String,
    data: OrderUpdateStatus,
) -> Result<ApiResponse<Order>, String> {
    let bridge = bridge.read().await;
    match bridge
        .put::<Order, _>(&format!("/api/orders/{}/status", encode(&order_id)), &data)
        .await
    {
        Ok(order) => Ok(ApiResponse::success(order)),
        Err(e) => Ok(ApiResponse::error(
            "ORDER_UPDATE_STATUS_FAILED",
            e.to_string(),
        )),
    }
}

#[tauri::command]
pub async fn update_order_hash(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    order_id: String,
    data: OrderUpdateHash,
) -> Result<ApiResponse<Order>, String> {
    let bridge = bridge.read().await;
    match bridge
        .put::<Order, _>(&format!("/api/orders/{}/hash", encode(&order_id)), &data)
        .await
    {
        Ok(order) => Ok(ApiResponse::success(order)),
        Err(e) => Ok(ApiResponse::error(
            "ORDER_UPDATE_HASH_FAILED",
            e.to_string(),
        )),
    }
}

// ============ Order Events ============

#[tauri::command]
pub async fn get_order_events(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    order_id: String,
) -> Result<ApiResponse<Vec<OrderEvent>>, String> {
    let bridge = bridge.read().await;
    match bridge
        .get::<Vec<OrderEvent>>(&format!("/api/orders/{}/events", encode(&order_id)))
        .await
    {
        Ok(events) => Ok(ApiResponse::success(events)),
        Err(e) => Ok(ApiResponse::error("ORDER_GET_EVENTS_FAILED", e.to_string())),
    }
}

#[tauri::command]
pub async fn add_order_event(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    order_id: String,
    data: OrderAddEvent,
) -> Result<OrderEvent, String> {
    let bridge = bridge.read().await;
    bridge
        .post(&format!("/api/orders/{}/events", encode(&order_id)), &data)
        .await
        .map_err(|e| e.to_string())
}
