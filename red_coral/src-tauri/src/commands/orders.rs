//! 订单 API Commands
//!
//! 通过 ClientBridge -> CrabClient -> EdgeServer REST API

use std::sync::Arc;
use tauri::State;
use tokio::sync::RwLock;

use crate::core::ClientBridge;
use shared::models::{
    Order, OrderCreate, OrderEvent,
    OrderAddItem, OrderAddPayment, OrderRemoveItem,
    OrderUpdateTotals, OrderUpdateStatus, OrderUpdateHash, OrderAddEvent,
};

// ============ Order Queries ============

#[tauri::command]
pub async fn list_orders(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    limit: Option<i32>,
    offset: Option<i32>,
) -> Result<Vec<Order>, String> {
    let bridge = bridge.read().await;
    let limit = limit.unwrap_or(50);
    let offset = offset.unwrap_or(0);
    bridge.get(&format!("/api/orders?limit={}&offset={}", limit, offset))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_open_orders(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
) -> Result<Vec<Order>, String> {
    let bridge = bridge.read().await;
    bridge.get("/api/orders/open").await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_order(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    id: String,
) -> Result<Order, String> {
    let bridge = bridge.read().await;
    bridge.get(&format!("/api/orders/{}", id)).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_order_by_receipt(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    receipt: String,
) -> Result<Order, String> {
    let bridge = bridge.read().await;
    bridge.get(&format!("/api/orders/receipt/{}", receipt)).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_last_order(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
) -> Result<Option<Order>, String> {
    let bridge = bridge.read().await;
    bridge.get("/api/orders/last").await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn verify_order_chain(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    from_hash: Option<String>,
) -> Result<bool, String> {
    let bridge = bridge.read().await;
    let query = match from_hash {
        Some(hash) => format!("/api/orders/verify?from_hash={}", hash),
        None => "/api/orders/verify".to_string(),
    };
    bridge.get(&query).await.map_err(|e| e.to_string())
}

// ============ Order Mutations ============

#[tauri::command]
pub async fn create_order(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    data: OrderCreate,
) -> Result<Order, String> {
    let bridge = bridge.read().await;
    bridge.post("/api/orders", &data).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn add_order_item(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    order_id: String,
    item: OrderAddItem,
) -> Result<Order, String> {
    let bridge = bridge.read().await;
    bridge.post(&format!("/api/orders/{}/items", order_id), &item)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn remove_order_item(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    order_id: String,
    data: OrderRemoveItem,
) -> Result<Order, String> {
    let bridge = bridge.read().await;
    bridge.delete_with_body(&format!("/api/orders/{}/items", order_id), &data)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn add_order_payment(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    order_id: String,
    payment: OrderAddPayment,
) -> Result<Order, String> {
    let bridge = bridge.read().await;
    bridge.post(&format!("/api/orders/{}/payments", order_id), &payment)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_order_totals(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    order_id: String,
    data: OrderUpdateTotals,
) -> Result<Order, String> {
    let bridge = bridge.read().await;
    bridge.put(&format!("/api/orders/{}/totals", order_id), &data)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_order_status(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    order_id: String,
    data: OrderUpdateStatus,
) -> Result<Order, String> {
    let bridge = bridge.read().await;
    bridge.put(&format!("/api/orders/{}/status", order_id), &data)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_order_hash(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    order_id: String,
    data: OrderUpdateHash,
) -> Result<Order, String> {
    let bridge = bridge.read().await;
    bridge.put(&format!("/api/orders/{}/hash", order_id), &data)
        .await
        .map_err(|e| e.to_string())
}

// ============ Order Events ============

#[tauri::command]
pub async fn get_order_events(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    order_id: String,
) -> Result<Vec<OrderEvent>, String> {
    let bridge = bridge.read().await;
    bridge.get(&format!("/api/orders/{}/events", order_id))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn add_order_event(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    order_id: String,
    data: OrderAddEvent,
) -> Result<OrderEvent, String> {
    let bridge = bridge.read().await;
    bridge.post(&format!("/api/orders/{}/events", order_id), &data)
        .await
        .map_err(|e| e.to_string())
}
