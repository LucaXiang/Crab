//! Order Repository (Archive)
//!
//! Read-only access to archived orders in SQLite.
//! All order mutations go through OrderManager event sourcing.

use super::{RepoError, RepoResult};
use sqlx::SqlitePool;
use std::collections::HashMap;

/// Archived order detail (for API response)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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
    pub void_type: Option<String>,
    pub loss_reason: Option<String>,
    pub loss_amount: Option<f64>,
    pub void_note: Option<String>,
    pub items: Vec<OrderDetailItem>,
    pub payments: Vec<OrderDetailPayment>,
    pub timeline: Vec<OrderDetailEvent>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct OrderDetailItem {
    pub id: i64,
    pub instance_id: String,
    pub name: String,
    pub spec_name: Option<String>,
    pub category_id: Option<i64>,
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
    pub applied_rules: Option<String>,
    pub note: Option<String>,
    pub is_comped: bool,
    pub tax: f64,
    pub tax_rate: i32,
    pub selected_options: Vec<OrderDetailOption>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct OrderDetailOption {
    pub attribute_name: String,
    pub option_name: String,
    pub price_modifier: f64,
    pub quantity: i32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct OrderDetailPayment {
    pub seq: i32,
    pub payment_id: String,
    pub method: String,
    pub amount: f64,
    pub timestamp: i64,
    pub cancelled: bool,
    pub cancel_reason: Option<String>,
    pub tendered: Option<f64>,
    pub change_amount: Option<f64>,
    pub split_type: Option<String>,
    pub split_items: Option<String>,
    pub aa_shares: Option<i32>,
    pub aa_total_shares: Option<i32>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct OrderDetailEvent {
    pub seq: i32,
    pub event_id: i64,
    pub event_type: String,
    pub timestamp: i64,
    pub payload: Option<String>,
}

// Internal FromRow types for sqlx (sqlx tuples max 16 fields)
#[derive(sqlx::FromRow)]
struct OrderRow {
    id: i64,
    receipt_number: String,
    table_name: Option<String>,
    zone_name: Option<String>,
    status: String,
    is_retail: bool,
    guest_count: Option<i32>,
    total_amount: f64,
    paid_amount: f64,
    discount_amount: f64,
    surcharge_amount: f64,
    comp_total_amount: f64,
    order_manual_discount_amount: f64,
    order_manual_surcharge_amount: f64,
    order_rule_discount_amount: f64,
    order_rule_surcharge_amount: f64,
    start_time: i64,
    end_time: Option<i64>,
    operator_name: Option<String>,
    void_type: Option<String>,
    loss_reason: Option<String>,
    loss_amount: Option<f64>,
    void_note: Option<String>,
}

#[derive(sqlx::FromRow)]
struct PaymentRow {
    seq: i32,
    payment_id: String,
    method: String,
    amount: f64,
    time: i64,
    cancelled: bool,
    cancel_reason: Option<String>,
    tendered: Option<f64>,
    change_amount: Option<f64>,
    split_type: Option<String>,
    split_items: Option<String>,
    aa_shares: Option<i32>,
    aa_total_shares: Option<i32>,
}

#[derive(sqlx::FromRow)]
struct EventRow {
    seq: i32,
    id: i64,
    event_type: String,
    timestamp: i64,
    data: Option<String>,
}

#[derive(sqlx::FromRow)]
struct ItemRow {
    id: i64,
    instance_id: String,
    name: String,
    spec_name: Option<String>,
    category_id: Option<i64>,
    category_name: Option<String>,
    price: f64,
    quantity: i32,
    unpaid_quantity: i32,
    unit_price: f64,
    line_total: f64,
    discount_amount: f64,
    surcharge_amount: f64,
    rule_discount_amount: f64,
    rule_surcharge_amount: f64,
    applied_rules: Option<String>,
    note: Option<String>,
    is_comped: bool,
    tax: f64,
    tax_rate: i32,
}

/// Get full order detail by reconstructing from archived tables
pub async fn get_order_detail(pool: &SqlitePool, order_id: i64) -> RepoResult<OrderDetail> {
    // 1. Get order
    let order: OrderRow = sqlx::query_as::<_, OrderRow>(
        "SELECT id, receipt_number, table_name, zone_name, status, is_retail, guest_count, total_amount, paid_amount, discount_amount, surcharge_amount, comp_total_amount, order_manual_discount_amount, order_manual_surcharge_amount, order_rule_discount_amount, order_rule_surcharge_amount, start_time, end_time, operator_name, void_type, loss_reason, loss_amount, void_note FROM archived_order WHERE id = ?",
    )
    .bind(order_id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| RepoError::NotFound(format!("Order {order_id} not found")))?;

    // 2. Get items
    let item_rows: Vec<ItemRow> = sqlx::query_as::<_, ItemRow>(
        "SELECT id, instance_id, name, spec_name, category_id, category_name, price, quantity, unpaid_quantity, unit_price, line_total, discount_amount, surcharge_amount, rule_discount_amount, rule_surcharge_amount, applied_rules, note, is_comped, tax, tax_rate FROM archived_order_item WHERE order_pk = ? ORDER BY id",
    )
    .bind(order_id)
    .fetch_all(pool)
    .await?;

    // Batch load all options for all items (eliminates N+1)
    // Dynamic query: variable number of IN placeholders — keep as runtime query
    let item_ids: Vec<i64> = item_rows.iter().map(|r| r.id).collect();
    let mut options_map: HashMap<i64, Vec<OrderDetailOption>> = HashMap::new();
    if !item_ids.is_empty() {
        let placeholders = item_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let sql = format!(
            "SELECT item_pk, attribute_name, option_name, price, quantity FROM archived_order_item_option WHERE item_pk IN ({placeholders})"
        );
        let mut query = sqlx::query_as::<_, (i64, String, String, f64, i32)>(&sql);
        for id in &item_ids {
            query = query.bind(id);
        }
        let all_options = query.fetch_all(pool).await?;
        for (item_pk, attr, opt, price, qty) in all_options {
            options_map.entry(item_pk).or_default().push(OrderDetailOption {
                attribute_name: attr,
                option_name: opt,
                price_modifier: price,
                quantity: qty,
            });
        }
    }

    let items: Vec<OrderDetailItem> = item_rows
        .into_iter()
        .map(|row| {
            let selected_options = options_map.remove(&row.id).unwrap_or_default();
            OrderDetailItem {
                id: row.id,
                instance_id: row.instance_id,
                name: row.name,
                spec_name: row.spec_name,
                category_id: row.category_id,
                category_name: row.category_name,
                price: row.price,
                quantity: row.quantity,
                unpaid_quantity: row.unpaid_quantity,
                unit_price: row.unit_price,
                line_total: row.line_total,
                discount_amount: row.discount_amount,
                surcharge_amount: row.surcharge_amount,
                rule_discount_amount: row.rule_discount_amount,
                rule_surcharge_amount: row.rule_surcharge_amount,
                applied_rules: row.applied_rules,
                note: row.note,
                is_comped: row.is_comped,
                tax: row.tax,
                tax_rate: row.tax_rate,
                selected_options,
            }
        })
        .collect();

    // 3. Get payments
    let payments: Vec<OrderDetailPayment> = sqlx::query_as::<_, PaymentRow>(
        "SELECT seq, payment_id, method, amount, time, cancelled, cancel_reason, tendered, change_amount, split_type, split_items, aa_shares, aa_total_shares FROM archived_order_payment WHERE order_pk = ? ORDER BY seq",
    )
    .bind(order_id)
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|r| OrderDetailPayment {
        seq: r.seq,
        payment_id: r.payment_id,
        method: r.method,
        amount: r.amount,
        timestamp: r.time,
        cancelled: r.cancelled,
        cancel_reason: r.cancel_reason,
        tendered: r.tendered,
        change_amount: r.change_amount,
        split_type: r.split_type,
        split_items: r.split_items,
        aa_shares: r.aa_shares,
        aa_total_shares: r.aa_total_shares,
    })
    .collect();

    // 4. Get events
    let timeline: Vec<OrderDetailEvent> = sqlx::query_as::<_, EventRow>(
        "SELECT seq, id, event_type, timestamp, data FROM archived_order_event WHERE order_pk = ? ORDER BY seq",
    )
    .bind(order_id)
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|r| OrderDetailEvent {
        seq: r.seq,
        event_id: r.id,
        event_type: r.event_type,
        timestamp: r.timestamp,
        payload: r.data,
    })
    .collect();

    Ok(OrderDetail {
        order_id: order.id,
        receipt_number: order.receipt_number,
        table_name: order.table_name,
        zone_name: order.zone_name,
        status: order.status,
        is_retail: order.is_retail,
        guest_count: order.guest_count,
        total: order.total_amount,
        paid_amount: order.paid_amount,
        total_discount: order.discount_amount,
        total_surcharge: order.surcharge_amount,
        comp_total_amount: order.comp_total_amount,
        order_manual_discount_amount: order.order_manual_discount_amount,
        order_manual_surcharge_amount: order.order_manual_surcharge_amount,
        order_rule_discount_amount: order.order_rule_discount_amount,
        order_rule_surcharge_amount: order.order_rule_surcharge_amount,
        start_time: order.start_time,
        end_time: order.end_time,
        operator_name: order.operator_name,
        void_type: order.void_type,
        loss_reason: order.loss_reason,
        loss_amount: order.loss_amount,
        void_note: order.void_note,
        items,
        payments,
        timeline,
    })
}
