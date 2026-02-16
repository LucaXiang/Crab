//! Kitchen Orders API Handlers
//!
//! Provides endpoints for kitchen order management:
//! - List kitchen orders (paginated or by order_id)
//! - Get single kitchen order
//! - Reprint kitchen order
//! - Label record management

use axum::{
    Json,
    extract::{Path, Query, State},
};
use serde::{Deserialize, Serialize};

use crate::core::ServerState;
use crate::printing::{KitchenOrder, LabelPrintRecord};
use crate::utils::{AppError, AppResult};

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

    let order = service
        .get_kitchen_order(&id)?
        .ok_or_else(|| AppError::not_found(format!("Kitchen order {} not found", id)))?;

    Ok(Json(order))
}

/// POST /api/kitchen-orders/:id/reprint - Reprint a kitchen order
pub async fn reprint(
    State(state): State<ServerState>,
    Path(id): Path<String>,
) -> AppResult<Json<bool>> {
    let service = state.kitchen_print_service();

    service.reprint_kitchen_order(&id)?;

    tracing::info!(kitchen_order_id = %id, "Kitchen order reprinted via API");

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

    let record = service
        .get_label_record(&id)?
        .ok_or_else(|| AppError::not_found(format!("Label record {} not found", id)))?;

    Ok(Json(record))
}

/// POST /api/label-records/:id/reprint - Reprint a label record
pub async fn reprint_label(
    State(state): State<ServerState>,
    Path(id): Path<String>,
) -> AppResult<Json<bool>> {
    let service = state.kitchen_print_service();

    service.reprint_label_record(&id)?;

    tracing::info!(label_record_id = %id, "Label record reprinted via API");

    Ok(Json(true))
}
