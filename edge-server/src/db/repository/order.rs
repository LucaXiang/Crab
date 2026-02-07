//! Order Repository (Archive)
//!
//! Read-only access to archived orders in SQLite.
//! All order mutations go through OrderManager event sourcing.

use super::{RepoError, RepoResult};
use sqlx::SqlitePool;

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
    pub selected_options: Vec<OrderDetailOption>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct OrderDetailOption {
    pub attribute_name: String,
    pub option_name: String,
    pub price_modifier: f64,
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
struct ItemRow {
    id: i64,
    instance_id: String,
    name: String,
    spec_name: Option<String>,
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
}

/// Get full order detail by reconstructing from archived tables
pub async fn get_order_detail(pool: &SqlitePool, order_id: i64) -> RepoResult<OrderDetail> {
    // 1. Get order
    let order: OrderRow = sqlx::query_as(
        "SELECT id, receipt_number, table_name, zone_name, status, is_retail, guest_count, total_amount, paid_amount, discount_amount, surcharge_amount, comp_total_amount, order_manual_discount_amount, order_manual_surcharge_amount, order_rule_discount_amount, order_rule_surcharge_amount, start_time, end_time, operator_name, void_type, loss_reason, loss_amount, void_note FROM archived_order WHERE id = ?",
    )
    .bind(order_id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| RepoError::NotFound(format!("Order {order_id} not found")))?;

    // 2. Get items
    let item_rows: Vec<ItemRow> = sqlx::query_as(
        "SELECT id, instance_id, name, spec_name, category_name, price, quantity, unpaid_quantity, unit_price, line_total, discount_amount, surcharge_amount, rule_discount_amount, rule_surcharge_amount, applied_rules, note, is_comped FROM archived_order_item WHERE order_pk = ? ORDER BY id",
    )
    .bind(order_id)
    .fetch_all(pool)
    .await?;

    let mut items = Vec::new();
    for row in item_rows {
        let options: Vec<(String, String, f64)> = sqlx::query_as(
            "SELECT attribute_name, option_name, price FROM archived_order_item_option WHERE item_pk = ?",
        )
        .bind(row.id)
        .fetch_all(pool)
        .await?;

        items.push(OrderDetailItem {
            id: row.id,
            instance_id: row.instance_id,
            name: row.name,
            spec_name: row.spec_name,
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
            selected_options: options
                .into_iter()
                .map(|(a, o, p)| OrderDetailOption {
                    attribute_name: a,
                    option_name: o,
                    price_modifier: p,
                })
                .collect(),
        });
    }

    // 3. Get payments
    let payments: Vec<OrderDetailPayment> = sqlx::query_as::<_, (i32, String, String, f64, i64, bool, Option<String>, Option<String>, Option<String>, Option<i32>, Option<i32>)>(
        "SELECT seq, payment_id, method, amount, time, cancelled, cancel_reason, split_type, split_items, aa_shares, aa_total_shares FROM archived_order_payment WHERE order_pk = ? ORDER BY seq, time",
    )
    .bind(order_id)
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|r| OrderDetailPayment {
        seq: r.0,
        payment_id: r.1,
        method: r.2,
        amount: r.3,
        timestamp: r.4,
        cancelled: r.5,
        cancel_reason: r.6,
        split_type: r.7,
        split_items: r.8,
        aa_shares: r.9,
        aa_total_shares: r.10,
    })
    .collect();

    // 4. Get events
    let timeline: Vec<OrderDetailEvent> = sqlx::query_as::<_, (i32, i64, String, i64, Option<String>)>(
        "SELECT seq, id, event_type, timestamp, data FROM archived_order_event WHERE order_pk = ? ORDER BY seq, timestamp",
    )
    .bind(order_id)
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|r| OrderDetailEvent {
        seq: r.0,
        event_id: r.1,
        event_type: r.2,
        timestamp: r.3,
        payload: r.4,
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
