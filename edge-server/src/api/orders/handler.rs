//! Order API Handlers

use axum::{
    Json,
    extract::{Path, Query, State},
};
use serde::{Deserialize, Serialize};

use crate::core::ServerState;
use crate::db::models::{Order, OrderAddItem, OrderAddPayment, OrderDetail, OrderEvent, OrderEventType, OrderSummary};
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

/// Get order by id - uses graph traversal for items/payments/timeline
pub async fn get_by_id(
    State(state): State<ServerState>,
    Path(id): Path<String>,
) -> AppResult<Json<OrderDetail>> {
    let repo = OrderRepository::new(state.db.clone());
    let detail = repo
        .get_order_detail(&id)
        .await
        .map_err(|e| AppError::database(e.to_string()))?;
    Ok(Json(detail))
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
    /// Search by receipt number (partial match)
    pub search: Option<String>,
}

/// Response wrapper for paginated order list
#[derive(Debug, Serialize)]
pub struct OrderListResponse {
    pub orders: Vec<OrderSummary>,
    pub total: i64,
    pub page: i32,
    pub limit: i32,
}

/// Fetch archived order list from SurrealDB with pagination
pub async fn fetch_order_list(
    State(state): State<ServerState>,
    Query(params): Query<OrderHistoryQuery>,
) -> AppResult<Json<OrderListResponse>> {
    let start_datetime = format!("{}T00:00:00Z", params.start_date);
    let end_datetime = format!("{}T23:59:59Z", params.end_date);
    let limit = params.limit.unwrap_or(100);
    let offset = params.offset.unwrap_or(0);
    let page = if limit > 0 { offset / limit + 1 } else { 1 };

    // Build WHERE clause
    let (where_clause, search_bind) = if let Some(ref search) = params.search {
        (
            "WHERE end_time >= <datetime>$start AND end_time <= <datetime>$end AND string::lowercase(receipt_number) CONTAINS $search",
            Some(search.to_lowercase()),
        )
    } else {
        (
            "WHERE end_time >= <datetime>$start AND end_time <= <datetime>$end",
            None,
        )
    };

    // Query 1: Get total count
    let count_query = format!("SELECT count() FROM order {} GROUP ALL", where_clause);
    let mut count_result = if let Some(ref search) = search_bind {
        state.db
            .query(&count_query)
            .bind(("start", start_datetime.clone()))
            .bind(("end", end_datetime.clone()))
            .bind(("search", search.clone()))
            .await
    } else {
        state.db
            .query(&count_query)
            .bind(("start", start_datetime.clone()))
            .bind(("end", end_datetime.clone()))
            .await
    }.map_err(|e| AppError::database(e.to_string()))?;

    #[derive(Deserialize)]
    struct CountResult {
        count: i64,
    }
    let total: i64 = count_result
        .take::<Option<CountResult>>(0)
        .map_err(|e| AppError::database(e.to_string()))?
        .map(|r| r.count)
        .unwrap_or(0);

    // Query 2: Get paginated results (graph model format)
    let data_query = format!(
        "SELECT \
         <string>id AS order_id, \
         receipt_number, \
         table_name, \
         string::uppercase(status) AS status, \
         is_retail, \
         total_amount AS total, \
         guest_count, \
         time::millis(start_time) AS start_time, \
         time::millis(end_time) AS end_time \
         FROM order {} ORDER BY end_time DESC LIMIT $limit START $offset",
        where_clause
    );
    let mut data_result = if let Some(ref search) = search_bind {
        state.db
            .query(&data_query)
            .bind(("start", start_datetime))
            .bind(("end", end_datetime))
            .bind(("search", search.clone()))
            .bind(("limit", limit))
            .bind(("offset", offset))
            .await
    } else {
        state.db
            .query(&data_query)
            .bind(("start", start_datetime))
            .bind(("end", end_datetime))
            .bind(("limit", limit))
            .bind(("offset", offset))
            .await
    }.map_err(|e| AppError::database(e.to_string()))?;

    let orders: Vec<OrderSummary> = data_result.take(0).map_err(|e| AppError::database(e.to_string()))?;

    Ok(Json(OrderListResponse {
        orders,
        total,
        page,
        limit,
    }))
}
