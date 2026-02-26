//! Kitchen Orders API Handlers
//!
//! Provides endpoints for kitchen order management:
//! - List kitchen orders (paginated or by order_id)
//! - Get single kitchen order
//! - Reprint kitchen order
//! - Label record management
//!
//! For archived orders (redb records cleaned up), falls back to rebuilding
//! from archived_order_event ITEMS_ADDED payloads.

use std::collections::HashMap;

use axum::{
    Json,
    extract::{Path, Query, State},
};
use serde::{Deserialize, Serialize};

use crate::core::ServerState;
use crate::db::repository::{order as order_repo, print_destination};
use crate::printing::{
    KitchenOrder, KitchenOrderItem, LabelPrintRecord, PrintExecutor, PrintItemContext,
};
use crate::utils::{AppError, AppResult};
use shared::error::ErrorCode;
use shared::order::EventPayload;

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
///
/// Falls back to archived events when redb has no records.
pub async fn list(
    State(state): State<ServerState>,
    Query(query): Query<ListQuery>,
) -> AppResult<Json<KitchenOrderListResponse>> {
    let service = state.kitchen_print_service();

    let items = if let Some(order_id) = query.order_id {
        // Try redb first
        let redb_items = service.get_kitchen_orders_for_order(&order_id)?;
        if redb_items.is_empty() {
            // Fallback: rebuild from archived events
            rebuild_kitchen_orders_from_archive(&state, &order_id).await?
        } else {
            redb_items
        }
    } else {
        // Paginated list — redb only (archived orders don't appear here)
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
///
/// Supports both redb records and archived orders.
/// For redb: increments print_count and reprints.
/// For archived: rebuilds from events, reprints without storing.
pub async fn reprint(
    State(state): State<ServerState>,
    Path(id): Path<String>,
) -> AppResult<Json<bool>> {
    let service = state.kitchen_print_service();

    // Try redb first
    let order = if let Some(_existing) = service.get_kitchen_order(&id)? {
        // Redb path: increment count
        service.reprint_kitchen_order(&id)?
    } else {
        // Fallback: the `id` might be an event_id from archived events.
        // We need the order_id to query archived events, but the caller
        // sends the kitchen_order.id (= event_id).
        // Search all archived ITEMS_ADDED events across all orders to find matching event_id.
        // Since this is expensive, we use a direct query.
        let event_row = sqlx::query_as::<_, (i64, i64, Option<String>)>(
            "SELECT e.id, e.timestamp, e.data FROM archived_order_event e \
             WHERE e.event_type = 'ITEMS_ADDED' AND CAST(e.id AS TEXT) = ?",
        )
        .bind(&id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|e| AppError::database(e.to_string()))?;

        if let Some((_event_id, timestamp, data)) = event_row {
            // Get order metadata
            let order_meta =
                sqlx::query_as::<_, (Option<String>, Option<String>, bool, Option<i32>, String)>(
                    "SELECT o.table_name, o.zone_name, o.is_retail, o.queue_number, o.order_key \
                 FROM archived_order o \
                 JOIN archived_order_event e ON e.order_pk = o.id \
                 WHERE CAST(e.id AS TEXT) = ?",
                )
                .bind(&id)
                .fetch_optional(&state.pool)
                .await
                .map_err(|e| AppError::database(e.to_string()))?;

            let (table_name, zone_name, is_retail, queue_number, order_key) = order_meta
                .ok_or_else(|| AppError::not_found(format!("Archived order for event {id}")))?;

            rebuild_single_kitchen_order(
                &state,
                &id,
                &order_key,
                table_name,
                zone_name,
                is_retail,
                queue_number.map(|n| n as u32),
                timestamp,
                data.as_deref(),
            )?
        } else {
            return Err(AppError::not_found(format!("Kitchen order {id}")));
        }
    };

    tracing::info!(
        kitchen_order_id = %id,
        print_count = order.print_count,
        "Kitchen order reprint requested"
    );

    // Load print destinations and execute printing
    let destinations = print_destination::find_all(&state.pool)
        .await
        .map_err(|e| AppError::database(e.to_string()))?;

    let dest_map: HashMap<String, _> = destinations
        .into_iter()
        .map(|d| (d.id.to_string(), d))
        .collect();

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
///
/// Falls back to archived events when redb has no records.
pub async fn list_labels(
    State(state): State<ServerState>,
    Query(query): Query<LabelListQuery>,
) -> AppResult<Json<Vec<LabelPrintRecord>>> {
    let service = state.kitchen_print_service();

    let records = service.get_label_records_for_order(&query.order_id)?;
    if !records.is_empty() {
        return Ok(Json(records));
    }

    // Fallback: rebuild from archived events
    let records = rebuild_label_records_from_archive(&state, &query.order_id).await?;
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

/// POST /api/label-records/:id/reprint - Reprint a single label
///
/// Takes any LabelPrintRecord ID, builds a new label with index="1/1",
/// and sends it to the printer. Increments print_count on the original record.
pub async fn reprint_label(
    State(state): State<ServerState>,
    Path(id): Path<String>,
) -> AppResult<Json<bool>> {
    let service = state.kitchen_print_service();

    // Get original record (redb or archived)
    let record = if let Some(r) = service.get_label_record(&id)? {
        // Increment print_count in redb
        service.reprint_label_record(&id)?;
        r
    } else {
        // Archived label — no redb record to update
        // Return not found for now; archived labels are rebuilt from events
        return Err(AppError::not_found(format!("Label record {id}")));
    };

    // Build a reprint record with index="1/1"
    let mut reprint_context = record.context.clone();
    reprint_context.index = Some("1/1".to_string());
    reprint_context.quantity = 1;

    let reprint_record = LabelPrintRecord {
        id: format!("reprint-{}", uuid::Uuid::new_v4()),
        order_id: record.order_id,
        kitchen_order_id: record.kitchen_order_id,
        table_name: record.table_name,
        queue_number: record.queue_number,
        is_retail: record.is_retail,
        created_at: chrono::Utc::now().timestamp_millis(),
        context: reprint_context,
        print_count: 0,
    };

    // Load destinations and print
    let destinations: HashMap<String, _> = print_destination::find_all(&state.pool)
        .await
        .map_err(|e| AppError::database(e.to_string()))?
        .into_iter()
        .map(|d| (d.id.to_string(), d))
        .collect();

    let executor = PrintExecutor::with_config(48, state.config.timezone);

    #[cfg(windows)]
    {
        use crate::db::repository::label_template;
        let template = match label_template::get_default(&state.pool).await {
            Ok(Some(db_tmpl)) => crate::printing::executor::convert_label_template(&db_tmpl),
            _ => {
                tracing::warn!("No default label template for reprint, using built-in");
                crab_printer::label::LabelTemplate::default()
            }
        };
        if let Err(e) = executor
            .print_label_records(&[reprint_record], &destinations, &template)
            .await
        {
            tracing::warn!(label_record_id = %id, error = %e, "Label reprint failed");
            return Ok(Json(false));
        }
    }

    #[cfg(not(windows))]
    {
        if let Err(e) = executor
            .print_label_records(&[reprint_record], &destinations)
            .await
        {
            tracing::warn!(label_record_id = %id, error = %e, "Label reprint failed");
            return Ok(Json(false));
        }
    }

    tracing::info!(label_record_id = %id, "Label reprinted successfully");
    Ok(Json(true))
}

// ============ Archive Fallback Helpers ============

/// Rebuild KitchenOrder list from archived ITEMS_ADDED events
async fn rebuild_kitchen_orders_from_archive(
    state: &ServerState,
    order_id: &str,
) -> Result<Vec<KitchenOrder>, AppError> {
    let (meta, events) = order_repo::get_items_added_events_by_order_key(&state.pool, order_id)
        .await
        .map_err(|e| AppError::database(e.to_string()))?;

    let Some(meta) = meta else {
        return Ok(vec![]);
    };

    let catalog = &state.catalog_service;
    let mut orders = Vec::new();

    for event in events {
        let Some(data) = event.data else { continue };
        let payload: EventPayload = match serde_json::from_str(&data) {
            Ok(p) => p,
            Err(_) => continue,
        };
        let EventPayload::ItemsAdded { items } = payload else {
            continue;
        };

        let kitchen_items: Vec<KitchenOrderItem> = items
            .iter()
            .map(|item| {
                let context = build_print_context_from_catalog(item, catalog);
                KitchenOrderItem { context }
            })
            .filter(|ki| !ki.context.kitchen_destinations.is_empty())
            .collect();

        if kitchen_items.is_empty() {
            continue;
        }

        orders.push(KitchenOrder {
            id: event.event_id.to_string(),
            order_id: meta.order_key.clone(),
            table_name: meta.table_name.clone(),
            zone_name: meta.zone_name.clone(),
            queue_number: meta.queue_number.map(|n| n as u32),
            is_retail: meta.is_retail,
            created_at: event.timestamp,
            items: kitchen_items,
            print_count: 0, // Archived — no redb counter
        });
    }

    Ok(orders)
}

/// Rebuild LabelPrintRecord list from archived ITEMS_ADDED events
async fn rebuild_label_records_from_archive(
    state: &ServerState,
    order_id: &str,
) -> Result<Vec<LabelPrintRecord>, AppError> {
    let (meta, events) = order_repo::get_items_added_events_by_order_key(&state.pool, order_id)
        .await
        .map_err(|e| AppError::database(e.to_string()))?;

    let Some(meta) = meta else {
        return Ok(vec![]);
    };

    let catalog = &state.catalog_service;
    let mut records = Vec::new();

    for event in events {
        let Some(data) = event.data else { continue };
        let payload: EventPayload = match serde_json::from_str(&data) {
            Ok(p) => p,
            Err(_) => continue,
        };
        let EventPayload::ItemsAdded { items } = payload else {
            continue;
        };

        for item in &items {
            let context = build_print_context_from_catalog(item, catalog);
            if context.label_destinations.is_empty() {
                continue;
            }

            for i in 1..=item.quantity {
                let mut label_context = context.clone();
                label_context.index = Some(format!("{}/{}", i, item.quantity));
                label_context.quantity = 1;

                records.push(LabelPrintRecord {
                    id: format!("archived-{}-{}-{}", event.event_id, item.id, i),
                    order_id: meta.order_key.clone(),
                    kitchen_order_id: event.event_id.to_string(),
                    table_name: meta.table_name.clone(),
                    queue_number: meta.queue_number.map(|n| n as u32),
                    is_retail: meta.is_retail,
                    created_at: event.timestamp,
                    context: label_context,
                    print_count: 0,
                });
            }
        }
    }

    Ok(records)
}

/// Rebuild a single KitchenOrder from archived event data
#[allow(clippy::too_many_arguments)]
fn rebuild_single_kitchen_order(
    state: &ServerState,
    event_id: &str,
    order_key: &str,
    table_name: Option<String>,
    zone_name: Option<String>,
    is_retail: bool,
    queue_number: Option<u32>,
    timestamp: i64,
    data: Option<&str>,
) -> Result<KitchenOrder, AppError> {
    let data = data.ok_or_else(|| AppError::not_found(format!("Kitchen order {event_id}")))?;
    let payload: EventPayload = serde_json::from_str(data)
        .map_err(|e| AppError::database(format!("Failed to parse event payload: {e}")))?;
    let EventPayload::ItemsAdded { items } = payload else {
        return Err(AppError::not_found(format!("Kitchen order {event_id}")));
    };

    let catalog = &state.catalog_service;
    let kitchen_items: Vec<KitchenOrderItem> = items
        .iter()
        .map(|item| KitchenOrderItem {
            context: build_print_context_from_catalog(item, catalog),
        })
        .collect();

    Ok(KitchenOrder {
        id: event_id.to_string(),
        order_id: order_key.to_string(),
        table_name,
        zone_name,
        queue_number,
        is_retail,
        created_at: timestamp,
        items: kitchen_items,
        print_count: 0,
    })
}

/// Build PrintItemContext from CartItemSnapshot using CatalogService
///
/// Mirrors KitchenPrintService::build_print_context but accessible outside the service.
fn build_print_context_from_catalog(
    item: &shared::order::CartItemSnapshot,
    catalog: &crate::services::CatalogService,
) -> PrintItemContext {
    let product = catalog.get_product(item.id);

    let (category_id, category_name) = if let Some(ref p) = product {
        let cat_name = catalog
            .get_category(p.category_id)
            .map(|c| c.name.clone())
            .unwrap_or_default();
        (p.category_id, cat_name)
    } else {
        (0, String::new())
    };

    let kitchen_config = catalog.get_kitchen_print_config(item.id);
    let label_config = catalog.get_label_print_config(item.id);

    let kitchen_destinations = kitchen_config
        .as_ref()
        .filter(|c| c.enabled)
        .map(|c| c.destinations.clone())
        .unwrap_or_default();

    let label_destinations = label_config
        .as_ref()
        .filter(|c| c.enabled)
        .map(|c| c.destinations.clone())
        .unwrap_or_default();

    let kitchen_name = kitchen_config
        .as_ref()
        .and_then(|c| c.kitchen_name.clone())
        .or_else(|| product.as_ref().map(|p| p.name.clone()))
        .unwrap_or_else(|| item.name.clone());

    let external_id = product.as_ref().and_then(|p| p.external_id);

    let options: Vec<String> = item
        .selected_options
        .as_ref()
        .map(|opts| {
            let mut groups: Vec<(String, Vec<String>)> = Vec::new();
            for opt in opts.iter().filter(|o| o.show_on_kitchen_print) {
                let name = opt
                    .kitchen_print_name
                    .as_deref()
                    .unwrap_or(&opt.option_name);
                let display = if opt.quantity > 1 {
                    format!("{}×{}", name, opt.quantity)
                } else {
                    name.to_string()
                };
                if let Some(group) = groups.iter_mut().find(|(a, _)| *a == opt.attribute_name) {
                    group.1.push(display);
                } else {
                    groups.push((opt.attribute_name.clone(), vec![display]));
                }
            }
            groups
                .into_iter()
                .map(|(attr, vals)| format!("{}: {}", attr, vals.join(", ")))
                .collect()
        })
        .unwrap_or_default();

    let label_options: Vec<String> = item
        .selected_options
        .as_ref()
        .map(|opts| {
            opts.iter()
                .filter(|opt| opt.show_on_receipt)
                .map(|opt| {
                    let name = opt.receipt_name.as_deref().unwrap_or(&opt.option_name);
                    if opt.quantity > 1 {
                        format!("{}×{}", name, opt.quantity)
                    } else {
                        name.to_string()
                    }
                })
                .collect()
        })
        .unwrap_or_default();

    let spec_name = item
        .selected_specification
        .as_ref()
        .map(|s| s.name.clone())
        .filter(|n| !n.is_empty());

    PrintItemContext {
        category_id,
        category_name,
        product_id: item.id,
        external_id,
        kitchen_name,
        product_name: item.name.clone(),
        spec_name,
        quantity: item.quantity,
        index: None,
        options,
        label_options,
        note: item.note.clone(),
        kitchen_destinations,
        label_destinations,
    }
}
