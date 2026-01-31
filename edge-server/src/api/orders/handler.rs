//! Order API Handlers
//!
//! Only provides read-only access to archived orders in SurrealDB.
//! All order mutations are handled through OrderManager event sourcing.

use axum::{
    Json,
    extract::{Path, Query, State},
};
use serde::{Deserialize, Serialize};
use crate::core::ServerState;
use crate::db::models::OrderSummary;
use crate::db::repository::OrderRepository;
use crate::utils::AppResult;

// =========================================================================
// Order Detail (Archived)
// =========================================================================

/// Order item option for detail view
#[derive(Debug, Serialize)]
pub struct OrderItemOptionDetail {
    pub attribute_name: String,
    pub option_name: String,
    pub price_modifier: f64,
}

/// Order item for detail view
#[derive(Debug, Serialize)]
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
#[derive(Debug, Serialize)]
pub struct OrderPaymentDetail {
    pub payment_id: Option<String>,
    pub method: String,
    pub amount: f64,
    pub timestamp: i64,
    pub cancelled: bool,
    pub cancel_reason: Option<String>,
    pub split_type: Option<String>,
    pub split_items: Vec<SplitItemDetail>,
    pub aa_shares: Option<i32>,
    pub aa_total_shares: Option<i32>,
}

/// Split item detail
#[derive(Debug, Serialize)]
pub struct SplitItemDetail {
    pub instance_id: String,
    pub name: String,
    pub quantity: i32,
    pub unit_price: f64,
}

/// Event for detail view
#[derive(Debug, Serialize)]
pub struct OrderEventDetail {
    pub event_id: String,
    pub event_type: String,
    pub timestamp: i64,
    pub payload: Option<serde_json::Value>,
}

/// Full order detail response
#[derive(Debug, Serialize)]
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
    // === Void Metadata ===
    pub void_type: Option<String>,
    pub loss_reason: Option<String>,
    pub loss_amount: Option<f64>,
    pub void_note: Option<String>,
    pub items: Vec<OrderItemDetail>,
    pub payments: Vec<OrderPaymentDetail>,
    pub timeline: Vec<OrderEventDetail>,
}

/// Get archived order by id - uses graph traversal for items/payments/timeline
pub async fn get_by_id(
    State(state): State<ServerState>,
    Path(id): Path<String>,
) -> AppResult<Json<OrderDetail>> {
    let repo = OrderRepository::new(state.db.clone());
    let detail = repo
        .get_order_detail(&id)
        .await
        ?;

    // Convert from db model to API response
    let response = OrderDetail {
        order_id: detail.order_id,
        receipt_number: detail.receipt_number,
        table_name: detail.table_name,
        zone_name: detail.zone_name,
        status: detail.status,
        is_retail: detail.is_retail,
        guest_count: detail.guest_count,
        total: detail.total,
        paid_amount: detail.paid_amount,
        total_discount: detail.total_discount,
        total_surcharge: detail.total_surcharge,
        start_time: detail.start_time,
        end_time: detail.end_time,
        operator_name: detail.operator_name,
        void_type: detail.void_type,
        loss_reason: detail.loss_reason,
        loss_amount: detail.loss_amount,
        void_note: detail.void_note,
        items: detail.items.into_iter().map(|i| OrderItemDetail {
            id: i.id,
            instance_id: i.instance_id,
            name: i.name,
            spec_name: i.spec_name,
            price: i.price,
            quantity: i.quantity,
            unpaid_quantity: i.unpaid_quantity,
            unit_price: i.unit_price,
            line_total: i.line_total,
            discount_amount: i.discount_amount,
            surcharge_amount: i.surcharge_amount,
            note: i.note,
            selected_options: i.selected_options.into_iter().map(|o| OrderItemOptionDetail {
                attribute_name: o.attribute_name,
                option_name: o.option_name,
                price_modifier: o.price_modifier,
            }).collect(),
        }).collect(),
        payments: detail.payments.into_iter().map(|p| OrderPaymentDetail {
            payment_id: p.payment_id,
            method: p.method,
            amount: p.amount,
            timestamp: p.timestamp,
            cancelled: p.cancelled,
            cancel_reason: p.cancel_reason,
            split_type: p.split_type,
            split_items: p.split_items.into_iter().map(|s| SplitItemDetail {
                instance_id: s.instance_id,
                name: s.name,
                quantity: s.quantity,
                unit_price: s.unit_price,
            }).collect(),
            aa_shares: p.aa_shares,
            aa_total_shares: p.aa_total_shares,
        }).collect(),
        timeline: detail.timeline.into_iter().map(|e| OrderEventDetail {
            event_id: e.event_id,
            event_type: e.event_type,
            timestamp: e.timestamp,
            payload: e.payload,
        }).collect(),
    };

    Ok(Json(response))
}

// =========================================================================
// Order History (Archived)
// =========================================================================

/// Query params for order history
#[derive(Debug, Deserialize)]
pub struct OrderHistoryQuery {
    /// Start time as UTC milliseconds (preferred) or date string "YYYY-MM-DD" (legacy)
    pub start_time: Option<i64>,
    pub start_date: Option<String>,
    /// End time as UTC milliseconds (preferred) or date string "YYYY-MM-DD" (legacy)
    pub end_time: Option<i64>,
    pub end_date: Option<String>,
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
    // Resolve start/end millis: prefer direct millis, fallback to date string
    let start_millis = params.start_time.unwrap_or_else(|| {
        params.start_date.as_deref()
            .and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
            .map(|d| d.and_hms_opt(0, 0, 0).unwrap().and_utc().timestamp_millis())
            .unwrap_or(0)
    });
    let end_millis = params.end_time.unwrap_or_else(|| {
        params.end_date.as_deref()
            .and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
            .map(|d| d.and_hms_opt(23, 59, 59).unwrap().and_utc().timestamp_millis())
            .unwrap_or(0)
    });
    let limit = params.limit.unwrap_or(100);
    let offset = params.offset.unwrap_or(0);
    let page = if limit > 0 { offset / limit + 1 } else { 1 };

    // Build WHERE clause
    let (where_clause, search_bind) = if let Some(ref search) = params.search {
        (
            "WHERE end_time >= $start AND end_time <= $end AND string::lowercase(receipt_number) CONTAINS $search",
            Some(search.to_lowercase()),
        )
    } else {
        (
            "WHERE end_time >= $start AND end_time <= $end",
            None,
        )
    };

    // Fetch all sorted results, paginate in Rust.
    // Workaround: SurrealDB Rust SDK embedded mode (kv-rocksdb) has a bug where
    // LIMIT drops the first record when combined with computed fields like <string>id.
    // The dataset is bounded by time range (typically 7 days), so in-memory pagination is fine.
    let data_query = format!(
        "SELECT \
         <string>id AS order_id, \
         receipt_number, \
         table_name, \
         string::uppercase(status) AS status, \
         is_retail, \
         total_amount AS total, \
         guest_count, \
         start_time, \
         end_time, \
         void_type, \
         loss_reason, \
         loss_amount \
         FROM order {} ORDER BY end_time DESC, start_time DESC",
        where_clause
    );
    let mut data_result = if let Some(ref search) = search_bind {
        state.db
            .query(&data_query)
            .bind(("start", start_millis))
            .bind(("end", end_millis))
            .bind(("search", search.clone()))
            .await
    } else {
        state.db
            .query(&data_query)
            .bind(("start", start_millis))
            .bind(("end", end_millis))
            .await
    }.map_err(crate::db::repository::surreal_err_to_app)?;

    let all_orders: Vec<OrderSummary> = data_result.take(0).map_err(crate::db::repository::surreal_err_to_app)?;
    let total = all_orders.len() as i64;
    let orders: Vec<OrderSummary> = all_orders
        .into_iter()
        .skip(offset as usize)
        .take(limit as usize)
        .collect();

    Ok(Json(OrderListResponse {
        orders,
        total,
        page,
        limit,
    }))
}
