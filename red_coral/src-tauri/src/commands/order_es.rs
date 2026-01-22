//! Order Event Sourcing Commands
//!
//! Tauri commands for the event-sourced order system.
//! These commands use the new event sourcing architecture via OrdersManager.

use shared::order::{
    CommandResponse, OrderCommand, OrderCommandPayload, OrderSnapshot, SyncResponse,
};
use std::sync::Arc;
use tauri::State;
use tokio::sync::RwLock;

use crate::core::response::ErrorCode;
use crate::core::{ApiResponse, ClientBridge, OrderEventListData, OrderSnapshotListData};

// ============ Order Commands ============

/// Execute an order command
///
/// This is the unified entry point for all order mutations.
/// The command is processed by OrdersManager, which generates events
/// and broadcasts them via MessageBus.
#[tauri::command(rename_all = "snake_case")]
pub async fn order_execute_command(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    command: OrderCommand,
) -> Result<ApiResponse<CommandResponse>, String> {
    let bridge = bridge.read().await;
    match bridge.execute_order_command(command).await {
        Ok(response) => Ok(ApiResponse::success(response)),
        Err(e) => Ok(ApiResponse::error_with_code(
            ErrorCode::DatabaseError,
            e.to_string(),
        )),
    }
}

/// Execute an order command with raw payload
///
/// Convenience wrapper that constructs the OrderCommand from parts.
#[tauri::command(rename_all = "snake_case")]
pub async fn order_execute(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    operator_id: String,
    operator_name: String,
    payload: OrderCommandPayload,
) -> Result<ApiResponse<CommandResponse>, String> {
    let bridge = bridge.read().await;
    let command = OrderCommand::new(operator_id, operator_name, payload);
    match bridge.execute_order_command(command).await {
        Ok(response) => Ok(ApiResponse::success(response)),
        Err(e) => Ok(ApiResponse::error_with_code(
            ErrorCode::DatabaseError,
            e.to_string(),
        )),
    }
}

// ============ Order Queries ============

/// Get all active order snapshots
///
/// Returns the current state of all active (non-completed, non-voided) orders.
#[tauri::command(rename_all = "snake_case")]
pub async fn order_get_active_orders(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
) -> Result<ApiResponse<OrderSnapshotListData>, String> {
    let bridge = bridge.read().await;
    match bridge.get_active_orders().await {
        Ok(snapshots) => Ok(ApiResponse::success(OrderSnapshotListData { snapshots })),
        Err(e) => Ok(ApiResponse::error_with_code(
            ErrorCode::DatabaseError,
            e.to_string(),
        )),
    }
}

/// Get a single order snapshot by ID
#[tauri::command(rename_all = "snake_case")]
pub async fn order_get_snapshot(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    order_id: String,
) -> Result<ApiResponse<Option<OrderSnapshot>>, String> {
    let bridge = bridge.read().await;
    match bridge.get_order_snapshot(&order_id).await {
        Ok(snapshot) => Ok(ApiResponse::success(snapshot)),
        Err(e) => Ok(ApiResponse::error_with_code(
            ErrorCode::OrderNotFound,
            e.to_string(),
        )),
    }
}

// ============ Sync Commands ============

/// Sync orders since a given sequence
///
/// Used for reconnection to get missed events and current state.
#[tauri::command(rename_all = "snake_case")]
pub async fn order_sync_since(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    since_sequence: u64,
) -> Result<ApiResponse<SyncResponse>, String> {
    let bridge = bridge.read().await;
    match bridge.sync_orders_since(since_sequence).await {
        Ok(response) => Ok(ApiResponse::success(response)),
        Err(e) => Ok(ApiResponse::error_with_code(
            ErrorCode::DatabaseError,
            e.to_string(),
        )),
    }
}

/// Get events for active orders since a given sequence
///
/// More efficient than full sync when only recent events are needed.
#[tauri::command(rename_all = "snake_case")]
pub async fn order_get_events_since(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    since_sequence: u64,
) -> Result<ApiResponse<OrderEventListData>, String> {
    let bridge = bridge.read().await;
    match bridge.get_active_events_since(since_sequence).await {
        Ok(events) => Ok(ApiResponse::success(OrderEventListData { events })),
        Err(e) => Ok(ApiResponse::error_with_code(
            ErrorCode::DatabaseError,
            e.to_string(),
        )),
    }
}

/// Get all events for a specific order
///
/// Used to reconstruct full order history including timeline for history details view.
#[tauri::command(rename_all = "snake_case")]
pub async fn order_get_events_for_order(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    order_id: String,
) -> Result<ApiResponse<OrderEventListData>, String> {
    let bridge = bridge.read().await;
    match bridge.get_events_for_order(&order_id).await {
        Ok(events) => Ok(ApiResponse::success(OrderEventListData { events })),
        Err(e) => Ok(ApiResponse::error_with_code(
            ErrorCode::DatabaseError,
            e.to_string(),
        )),
    }
}
