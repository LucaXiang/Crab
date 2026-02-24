//! Order API Handlers
//!
//! Only provides read-only access to archived orders in SQLite.
//! All order mutations are handled through OrderManager event sourcing.

use crate::core::ServerState;
use crate::db::repository::order;
use crate::utils::time;
use crate::utils::{AppError, AppResult};
use axum::{
    Json,
    extract::{Path, Query, State},
};
use serde::{Deserialize, Serialize};

// =========================================================================
// Order Detail (Archived)
// =========================================================================

/// Order item option for detail view
#[derive(Debug, Serialize)]
pub struct OrderItemOptionDetail {
    pub attribute_name: String,
    pub option_name: String,
    pub price_modifier: f64,
    pub quantity: i32,
}

/// Order item for detail view
#[derive(Debug, Serialize)]
pub struct OrderItemDetail {
    pub id: i64,
    pub instance_id: String,
    pub name: String,
    pub spec_name: Option<String>,
    pub category_name: Option<String>,
    pub price: f64,
    pub quantity: i32,
    pub unpaid_quantity: i32,
    pub unit_price: f64,
    pub line_total: f64,
    pub discount_amount: f64,
    pub surcharge_amount: f64,
    pub rule_discount_amount: f64,
    pub rule_surcharge_amount: f64,
    pub applied_rules: Option<serde_json::Value>,
    pub note: Option<String>,
    pub is_comped: bool,
    pub tax: f64,
    pub tax_rate: i32,
    pub selected_options: Vec<OrderItemOptionDetail>,
}

/// Payment for detail view
#[derive(Debug, Serialize)]
pub struct OrderPaymentDetail {
    pub payment_id: String,
    pub method: String,
    pub amount: f64,
    pub timestamp: i64,
    pub cancelled: bool,
    pub cancel_reason: Option<String>,
    pub tendered: Option<f64>,
    pub change_amount: Option<f64>,
    pub split_type: Option<String>,
    pub split_items: Vec<SplitItemDetail>,
    pub aa_shares: Option<i32>,
    pub aa_total_shares: Option<i32>,
}

/// Split item detail
#[derive(Debug, Serialize, Deserialize)]
pub struct SplitItemDetail {
    pub instance_id: String,
    pub name: String,
    pub quantity: i32,
    pub unit_price: f64,
}

/// Event for detail view
#[derive(Debug, Serialize)]
pub struct OrderEventDetail {
    pub event_id: i64,
    pub event_type: String,
    pub timestamp: i64,
    pub payload: Option<serde_json::Value>,
}

/// Full order detail response
#[derive(Debug, Serialize)]
pub struct OrderDetail {
    pub order_id: i64,
    pub receipt_number: String,
    pub table_name: Option<String>,
    pub zone_name: Option<String>,
    pub status: String,
    pub is_retail: bool,
    pub guest_count: Option<i32>,
    pub total: f64,
    pub paid_amount: f64,
    pub total_discount: f64,
    pub total_surcharge: f64,
    pub comp_total_amount: f64,
    pub order_manual_discount_amount: f64,
    pub order_manual_surcharge_amount: f64,
    pub order_rule_discount_amount: f64,
    pub order_rule_surcharge_amount: f64,
    pub start_time: i64,
    pub end_time: Option<i64>,
    pub operator_name: Option<String>,
    // === Void Metadata ===
    pub void_type: Option<String>,
    pub loss_reason: Option<String>,
    pub loss_amount: Option<f64>,
    pub void_note: Option<String>,
    pub queue_number: Option<i32>,
    pub items: Vec<OrderItemDetail>,
    pub payments: Vec<OrderPaymentDetail>,
    pub timeline: Vec<OrderEventDetail>,
}

/// Get archived order by id
pub async fn get_by_id(
    State(state): State<ServerState>,
    Path(id): Path<i64>,
) -> AppResult<Json<OrderDetail>> {
    let detail = order::get_order_detail(&state.pool, id).await?;

    // Convert from repo model to API response
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
        comp_total_amount: detail.comp_total_amount,
        order_manual_discount_amount: detail.order_manual_discount_amount,
        order_manual_surcharge_amount: detail.order_manual_surcharge_amount,
        order_rule_discount_amount: detail.order_rule_discount_amount,
        order_rule_surcharge_amount: detail.order_rule_surcharge_amount,
        start_time: detail.start_time,
        end_time: detail.end_time,
        operator_name: detail.operator_name,
        void_type: detail.void_type,
        loss_reason: detail.loss_reason,
        loss_amount: detail.loss_amount,
        void_note: detail.void_note,
        queue_number: detail.queue_number,
        items: detail
            .items
            .into_iter()
            .map(|i| OrderItemDetail {
                id: i.id,
                instance_id: i.instance_id,
                name: i.name,
                spec_name: i.spec_name,
                category_name: i.category_name,
                price: i.price,
                quantity: i.quantity,
                unpaid_quantity: i.unpaid_quantity,
                unit_price: i.unit_price,
                line_total: i.line_total,
                discount_amount: i.discount_amount,
                surcharge_amount: i.surcharge_amount,
                rule_discount_amount: i.rule_discount_amount,
                rule_surcharge_amount: i.rule_surcharge_amount,
                applied_rules: i.applied_rules.and_then(|s| serde_json::from_str(&s).ok()),
                note: i.note,
                is_comped: i.is_comped,
                tax: i.tax,
                tax_rate: i.tax_rate,
                selected_options: i
                    .selected_options
                    .into_iter()
                    .map(|o| OrderItemOptionDetail {
                        attribute_name: o.attribute_name,
                        option_name: o.option_name,
                        price_modifier: o.price_modifier,
                        quantity: o.quantity,
                    })
                    .collect(),
            })
            .collect(),
        payments: detail
            .payments
            .into_iter()
            .map(|p| {
                let split_items: Vec<SplitItemDetail> = p
                    .split_items
                    .and_then(|s| serde_json::from_str(&s).ok())
                    .unwrap_or_default();
                OrderPaymentDetail {
                    payment_id: p.payment_id,
                    method: p.method,
                    amount: p.amount,
                    timestamp: p.timestamp,
                    cancelled: p.cancelled,
                    cancel_reason: p.cancel_reason,
                    tendered: p.tendered,
                    change_amount: p.change_amount,
                    split_type: p.split_type,
                    split_items,
                    aa_shares: p.aa_shares,
                    aa_total_shares: p.aa_total_shares,
                }
            })
            .collect(),
        timeline: detail
            .timeline
            .into_iter()
            .map(|e| OrderEventDetail {
                event_id: e.event_id,
                event_type: e.event_type,
                timestamp: e.timestamp,
                payload: e.payload.and_then(|s| serde_json::from_str(&s).ok()),
            })
            .collect(),
    };

    Ok(Json(response))
}

// =========================================================================
// Order History (Archived)
// =========================================================================

/// Order summary for list view
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct OrderSummary {
    pub order_id: i64,
    pub receipt_number: String,
    pub table_name: Option<String>,
    pub status: String,
    pub is_retail: bool,
    pub total: f64,
    pub guest_count: Option<i32>,
    pub start_time: i64,
    pub end_time: Option<i64>,
    pub void_type: Option<String>,
    pub loss_reason: Option<String>,
    pub loss_amount: Option<f64>,
}

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

/// Fetch archived order list from SQLite with pagination
pub async fn fetch_order_list(
    State(state): State<ServerState>,
    Query(params): Query<OrderHistoryQuery>,
) -> AppResult<Json<OrderListResponse>> {
    // Resolve start/end millis: prefer direct millis, fallback to date string
    let tz = state.config.timezone;
    let start_millis = params.start_time.unwrap_or_else(|| {
        params
            .start_date
            .as_deref()
            .and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
            .map(|d| time::day_start_millis(d, tz))
            .unwrap_or(0)
    });
    let end_millis = params.end_time.unwrap_or_else(|| {
        params
            .end_date
            .as_deref()
            .and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
            .map(|d| time::day_end_millis(d, tz))
            .unwrap_or(0)
    });
    let limit = params.limit.unwrap_or(100);
    let offset = params.offset.unwrap_or(0);
    let page = if limit > 0 { offset / limit + 1 } else { 1 };

    let (orders, total) = if let Some(ref search) = params.search {
        let search_pattern = format!("%{}%", search.to_lowercase());
        let total: i64 = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM archived_order WHERE end_time >= ?1 AND end_time < ?2 AND LOWER(receipt_number) LIKE ?3",
            start_millis, end_millis, search_pattern,
        )
        .fetch_one(&state.pool)
        .await
        .map_err(|e| AppError::database(e.to_string()))?;

        let rows = sqlx::query_as::<_, OrderSummary>(
            "SELECT id AS order_id, receipt_number, table_name, UPPER(status) AS status, is_retail, total_amount AS total, guest_count, start_time, end_time, void_type, loss_reason, loss_amount FROM archived_order WHERE end_time >= ?1 AND end_time < ?2 AND LOWER(receipt_number) LIKE ?3 ORDER BY end_time DESC, start_time DESC LIMIT ?4 OFFSET ?5",
        )
        .bind(start_millis)
        .bind(end_millis)
        .bind(&search_pattern)
        .bind(limit)
        .bind(offset)
        .fetch_all(&state.pool)
        .await
        .map_err(|e| AppError::database(e.to_string()))?;

        (rows, total)
    } else {
        let total: i64 = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM archived_order WHERE end_time >= ?1 AND end_time < ?2",
            start_millis,
            end_millis,
        )
        .fetch_one(&state.pool)
        .await
        .map_err(|e| AppError::database(e.to_string()))?;

        let rows = sqlx::query_as::<_, OrderSummary>(
            "SELECT id AS order_id, receipt_number, table_name, UPPER(status) AS status, is_retail, total_amount AS total, guest_count, start_time, end_time, void_type, loss_reason, loss_amount FROM archived_order WHERE end_time >= ?1 AND end_time < ?2 ORDER BY end_time DESC, start_time DESC LIMIT ?3 OFFSET ?4",
        )
        .bind(start_millis)
        .bind(end_millis)
        .bind(limit)
        .bind(offset)
        .fetch_all(&state.pool)
        .await
        .map_err(|e| AppError::database(e.to_string()))?;

        (rows, total)
    };

    Ok(Json(OrderListResponse {
        orders,
        total,
        page,
        limit,
    }))
}

// =========================================================================
// Member Spending History (Archived)
// =========================================================================

/// Query params for member spending history
#[derive(Debug, Deserialize)]
pub struct MemberHistoryQuery {
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

/// Member spending summary (aggregated stats from archived_order)
#[derive(Debug, Serialize)]
pub struct MemberSpendingResponse {
    pub orders: Vec<OrderSummary>,
    pub total: i64,
    pub page: i32,
    pub limit: i32,
}

/// Fetch archived orders for a specific member
pub async fn fetch_member_history(
    State(state): State<ServerState>,
    Path(member_id): Path<i64>,
    Query(params): Query<MemberHistoryQuery>,
) -> AppResult<Json<MemberSpendingResponse>> {
    let limit = params.limit.unwrap_or(50);
    let offset = params.offset.unwrap_or(0);
    let page = if limit > 0 { offset / limit + 1 } else { 1 };

    let total: i64 = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM archived_order WHERE member_id = ?1 AND UPPER(status) = 'COMPLETED'",
        member_id,
    )
    .fetch_one(&state.pool)
    .await
    .map_err(|e| AppError::database(e.to_string()))?;

    let rows = sqlx::query_as::<_, OrderSummary>(
        "SELECT id AS order_id, receipt_number, table_name, UPPER(status) AS status, is_retail, total_amount AS total, guest_count, start_time, end_time, void_type, loss_reason, loss_amount FROM archived_order WHERE member_id = ?1 AND UPPER(status) = 'COMPLETED' ORDER BY end_time DESC LIMIT ?2 OFFSET ?3",
    )
    .bind(member_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| AppError::database(e.to_string()))?;

    Ok(Json(MemberSpendingResponse {
        orders: rows,
        total,
        page,
        limit,
    }))
}
