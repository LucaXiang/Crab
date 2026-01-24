//! Order API Handlers

use axum::{
    Json,
    extract::{Path, Query, State},
};
use serde::{Deserialize, Serialize};

use crate::core::ServerState;
use crate::db::models::{Order, OrderAddItem, OrderAddPayment, OrderEvent, OrderEventType};
use crate::db::repository::OrderRepository;
use crate::utils::{AppError, AppResult};

const RESOURCE: &str = "order";

/// Query params for listing orders
#[derive(Debug, Deserialize)]
pub struct ListQuery {
    #[serde(default = "default_limit")]
    pub limit: i32,
    #[serde(default)]
    pub offset: i32,
}

fn default_limit() -> i32 {
    50
}

/// List all orders (paginated)
pub async fn list(
    State(state): State<ServerState>,
    Query(query): Query<ListQuery>,
) -> AppResult<Json<Vec<Order>>> {
    let repo = OrderRepository::new(state.db.clone());
    let orders = repo
        .find_all(query.limit, query.offset)
        .await
        .map_err(|e| AppError::database(e.to_string()))?;
    Ok(Json(orders))
}

/// Get order by id
pub async fn get_by_id(
    State(state): State<ServerState>,
    Path(id): Path<String>,
) -> AppResult<Json<Order>> {
    let repo = OrderRepository::new(state.db.clone());
    let order = repo
        .find_by_id(&id)
        .await
        .map_err(|e| AppError::database(e.to_string()))?
        .ok_or_else(|| AppError::not_found(format!("Order {} not found", id)))?;
    Ok(Json(order))
}

/// Get order by receipt number
pub async fn get_by_receipt(
    State(state): State<ServerState>,
    Path(receipt): Path<String>,
) -> AppResult<Json<Order>> {
    let repo = OrderRepository::new(state.db.clone());
    let order = repo
        .find_by_receipt(&receipt)
        .await
        .map_err(|e| AppError::database(e.to_string()))?
        .ok_or_else(|| AppError::not_found(format!("Order with receipt {} not found", receipt)))?;
    Ok(Json(order))
}

/// Add item to order
pub async fn add_item(
    State(state): State<ServerState>,
    Path(id): Path<String>,
    Json(payload): Json<OrderAddItem>,
) -> AppResult<Json<Order>> {
    let repo = OrderRepository::new(state.db.clone());
    let order = repo
        .add_item(&id, payload)
        .await
        .map_err(|e| AppError::database(e.to_string()))?;

    state
        .broadcast_sync(RESOURCE, "item_added", &id, Some(&order))
        .await;

    Ok(Json(order))
}

/// Remove item request
#[derive(Debug, Deserialize)]
pub struct RemoveItemRequest {
    pub index: usize,
}

/// Remove item from order by index
pub async fn remove_item(
    State(state): State<ServerState>,
    Path(id): Path<String>,
    Json(payload): Json<RemoveItemRequest>,
) -> AppResult<Json<Order>> {
    let repo = OrderRepository::new(state.db.clone());
    let order = repo
        .remove_item(&id, payload.index)
        .await
        .map_err(|e| AppError::database(e.to_string()))?;

    state
        .broadcast_sync(RESOURCE, "item_removed", &id, Some(&order))
        .await;

    Ok(Json(order))
}

/// Add payment to order
pub async fn add_payment(
    State(state): State<ServerState>,
    Path(id): Path<String>,
    Json(payload): Json<OrderAddPayment>,
) -> AppResult<Json<Order>> {
    let repo = OrderRepository::new(state.db.clone());
    let order = repo
        .add_payment(&id, payload)
        .await
        .map_err(|e| AppError::database(e.to_string()))?;

    state
        .broadcast_sync(RESOURCE, "payment_added", &id, Some(&order))
        .await;

    Ok(Json(order))
}

/// Update totals request
#[derive(Debug, Deserialize)]
pub struct UpdateTotalsRequest {
    pub total_amount: i32,
    pub discount_amount: i32,
    pub surcharge_amount: i32,
}

/// Update order totals
pub async fn update_totals(
    State(state): State<ServerState>,
    Path(id): Path<String>,
    Json(payload): Json<UpdateTotalsRequest>,
) -> AppResult<Json<Order>> {
    let repo = OrderRepository::new(state.db.clone());
    let order = repo
        .update_totals(
            &id,
            payload.total_amount,
            payload.discount_amount,
            payload.surcharge_amount,
        )
        .await
        .map_err(|e| AppError::database(e.to_string()))?;

    state
        .broadcast_sync(RESOURCE, "totals_updated", &id, Some(&order))
        .await;

    Ok(Json(order))
}

/// Update hash request
#[derive(Debug, Deserialize)]
pub struct UpdateHashRequest {
    pub prev_hash: String,
    pub curr_hash: String,
}

/// Update order hash
pub async fn update_hash(
    State(state): State<ServerState>,
    Path(id): Path<String>,
    Json(payload): Json<UpdateHashRequest>,
) -> AppResult<Json<Order>> {
    let repo = OrderRepository::new(state.db.clone());
    let order = repo
        .update_hash(&id, payload.prev_hash, payload.curr_hash)
        .await
        .map_err(|e| AppError::database(e.to_string()))?;

    state
        .broadcast_sync(RESOURCE, "hash_updated", &id, Some(&order))
        .await;

    Ok(Json(order))
}

/// Add event request
#[derive(Debug, Deserialize)]
pub struct AddEventRequest {
    pub event_type: OrderEventType,
    pub data: Option<serde_json::Value>,
    pub prev_hash: String,
    pub curr_hash: String,
}

/// Add event to order
pub async fn add_event(
    State(state): State<ServerState>,
    Path(id): Path<String>,
    Json(payload): Json<AddEventRequest>,
) -> AppResult<Json<OrderEvent>> {
    let repo = OrderRepository::new(state.db.clone());
    let event = repo
        .add_event(
            &id,
            payload.event_type,
            payload.data,
            payload.prev_hash,
            payload.curr_hash,
        )
        .await
        .map_err(|e| AppError::database(e.to_string()))?;

    state
        .broadcast_sync(RESOURCE, "event_added", &id, Some(&event))
        .await;

    Ok(Json(event))
}

/// Get order events
pub async fn get_events(
    State(state): State<ServerState>,
    Path(id): Path<String>,
) -> AppResult<Json<Vec<OrderEvent>>> {
    let repo = OrderRepository::new(state.db.clone());
    let events = repo
        .get_events(&id)
        .await
        .map_err(|e| AppError::database(e.to_string()))?;
    Ok(Json(events))
}

/// Get last order (for hash chain)
pub async fn get_last(State(state): State<ServerState>) -> AppResult<Json<Option<Order>>> {
    let repo = OrderRepository::new(state.db.clone());
    let order = repo
        .get_last_order()
        .await
        .map_err(|e| AppError::database(e.to_string()))?;
    Ok(Json(order))
}

/// Verify chain request
#[derive(Debug, Deserialize)]
pub struct VerifyChainQuery {
    pub from_hash: Option<String>,
}

/// Verify hash chain integrity
pub async fn verify_chain(
    State(state): State<ServerState>,
    Query(query): Query<VerifyChainQuery>,
) -> AppResult<Json<bool>> {
    let repo = OrderRepository::new(state.db.clone());
    let valid = repo
        .verify_chain(query.from_hash)
        .await
        .map_err(|e| AppError::database(e.to_string()))?;
    Ok(Json(valid))
}

// =========================================================================
// Order History
// =========================================================================

/// Query params for order history
#[derive(Debug, Deserialize)]
pub struct OrderHistoryQuery {
    pub start_date: String,
    pub end_date: String,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

/// Order summary for history list
#[derive(Debug, Serialize)]
pub struct OrderSummary {
    pub id: Option<String>,
    pub receipt_number: String,
    pub status: String,
    pub zone_name: Option<String>,
    pub table_name: Option<String>,
    pub total_amount: f64,
    pub paid_amount: f64,
    pub start_time: String,
    pub end_time: Option<String>,
    pub guest_count: Option<i32>,
}

/// Fetch archived order list from SurrealDB
pub async fn fetch_order_list(
    State(state): State<ServerState>,
    Query(params): Query<OrderHistoryQuery>,
) -> AppResult<Json<Vec<OrderSummary>>> {
    let repo = OrderRepository::new(state.db.clone());

    let orders = repo
        .find_by_date_range(
            &params.start_date,
            &params.end_date,
            params.limit.unwrap_or(100),
            params.offset.unwrap_or(0),
        )
        .await
        .map_err(|e| AppError::database(e.to_string()))?;

    let summaries: Vec<OrderSummary> = orders
        .into_iter()
        .map(|o| OrderSummary {
            id: o.id.map(|id| id.to_string()),
            receipt_number: o.receipt_number,
            status: format!("{:?}", o.status),
            zone_name: o.zone_name,
            table_name: o.table_name,
            total_amount: o.total_amount,
            paid_amount: o.paid_amount,
            start_time: o.start_time,
            end_time: o.end_time,
            guest_count: o.guest_count,
        })
        .collect();

    Ok(Json(summaries))
}
