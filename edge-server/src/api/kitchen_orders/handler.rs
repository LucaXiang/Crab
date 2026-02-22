//! Kitchen Orders API Handlers
//!
//! Provides endpoints for kitchen order management:
//! - List kitchen orders (paginated or by order_id)
//! - Get single kitchen order
//! - Reprint kitchen order
//! - Label record management

use std::collections::HashMap;

use axum::{
    Json,
    extract::{Path, Query, State},
};
use serde::{Deserialize, Serialize};

use crate::core::ServerState;
use crate::db::repository::print_destination;
use crate::printing::{KitchenOrder, LabelPrintRecord, PrintExecutor};
use crate::utils::{AppError, AppResult};
use shared::error::ErrorCode;

/// Query params for listing kitchen orders
#[derive(Debug, Deserialize)]
pub struct ListQuery {
    /// Filter by order_id (optional)
    pub order_id: Option<String>,
    /// Page offset (default 0)
    #[serde(default)]
    pub offset: usize,
    /// Page limit (default 20)
    #[serde(default = "default_limit")]
    pub limit: usize,
}

fn default_limit() -> usize {
    20
}

/// Response for kitchen order list
#[derive(Debug, Serialize)]
pub struct KitchenOrderListResponse {
    pub items: Vec<KitchenOrder>,
    pub total: Option<usize>,
}

/// GET /api/kitchen-orders - List kitchen orders
///
/// Query params:
/// - order_id: Filter by order ID (returns all for that order)
/// - offset: Page offset (default 0)
/// - limit: Page limit (default 20)
pub async fn list(
    State(state): State<ServerState>,
    Query(query): Query<ListQuery>,
) -> AppResult<Json<KitchenOrderListResponse>> {
    let service = state.kitchen_print_service();

    let items = if let Some(order_id) = query.order_id {
        // Get all kitchen orders for a specific order
        service.get_kitchen_orders_for_order(&order_id)?
    } else {
        // Get paginated list
        service.get_all_kitchen_orders(query.offset, query.limit)?
    };

    Ok(Json(KitchenOrderListResponse { items, total: None }))
}

/// GET /api/kitchen-orders/:id - Get a single kitchen order
pub async fn get_by_id(
    State(state): State<ServerState>,
    Path(id): Path<String>,
) -> AppResult<Json<KitchenOrder>> {
    let service = state.kitchen_print_service();

    let order = service.get_kitchen_order(&id)?.ok_or_else(|| {
        AppError::with_message(
            ErrorCode::OrderNotFound,
            format!("Kitchen order {} not found", id),
        )
    })?;

    Ok(Json(order))
}

/// POST /api/kitchen-orders/:id/reprint - Reprint a kitchen order
pub async fn reprint(
    State(state): State<ServerState>,
    Path(id): Path<String>,
) -> AppResult<Json<bool>> {
    let service = state.kitchen_print_service();

    // 1. Increment print_count and get the order
    let order = service.reprint_kitchen_order(&id)?;

    tracing::info!(
        kitchen_order_id = %id,
        print_count = order.print_count,
        "Kitchen order reprint requested"
    );

    // 2. Load print destinations
    let destinations = print_destination::find_all(&state.pool)
        .await
        .map_err(|e| AppError::database(e.to_string()))?;

    let dest_map: HashMap<String, _> = destinations
        .into_iter()
        .map(|d| (d.id.to_string(), d))
        .collect();

    // 3. Execute actual printing
    let executor = PrintExecutor::with_config(48, state.config.timezone);
    if let Err(e) = executor.print_kitchen_order(&order, &dest_map).await {
        tracing::warn!(
            kitchen_order_id = %id,
            error = %e,
            "Reprint failed"
        );
        return Ok(Json(false));
    }

    Ok(Json(true))
}

/// Query params for listing label records
#[derive(Debug, Deserialize)]
pub struct LabelListQuery {
    /// Filter by order_id (required)
    pub order_id: String,
}

/// GET /api/label-records - List label records for an order
pub async fn list_labels(
    State(state): State<ServerState>,
    Query(query): Query<LabelListQuery>,
) -> AppResult<Json<Vec<LabelPrintRecord>>> {
    let service = state.kitchen_print_service();

    let records = service.get_label_records_for_order(&query.order_id)?;

    Ok(Json(records))
}

/// GET /api/label-records/:id - Get a single label record
pub async fn get_label_by_id(
    State(state): State<ServerState>,
    Path(id): Path<String>,
) -> AppResult<Json<LabelPrintRecord>> {
    let service = state.kitchen_print_service();

    let record = service.get_label_record(&id)?.ok_or_else(|| {
        AppError::with_message(
            ErrorCode::OrderItemNotFound,
            format!("Label record {} not found", id),
        )
    })?;

    Ok(Json(record))
}

/// POST /api/label-records/:id/reprint - Reprint a label record
///
/// Note: Label reprint currently only increments the counter.
/// Actual label re-printing requires the label template renderer (Windows GDI+),
/// which is only available on the Tauri frontend.
pub async fn reprint_label(
    State(state): State<ServerState>,
    Path(id): Path<String>,
) -> AppResult<Json<bool>> {
    let service = state.kitchen_print_service();

    service.reprint_label_record(&id)?;

    tracing::info!(label_record_id = %id, "Label record reprinted via API");

    Ok(Json(true))
}
