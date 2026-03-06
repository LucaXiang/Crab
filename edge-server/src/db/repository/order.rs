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
    pub original_total: f64,
    pub total: f64,
    pub subtotal: f64,
    pub paid_amount: f64,
    pub total_discount: f64,
    pub total_surcharge: f64,
    pub comp_total_amount: f64,
    pub order_manual_discount_amount: f64,
    pub order_manual_surcharge_amount: f64,
    pub order_rule_discount_amount: f64,
    pub order_rule_surcharge_amount: f64,
    pub member_id: Option<i64>,
    pub member_name: Option<String>,
    pub mg_discount_amount: f64,
    pub marketing_group_name: Option<String>,
    pub start_time: i64,
    pub end_time: Option<i64>,
    pub operator_name: Option<String>,
    pub void_type: Option<String>,
    pub loss_reason: Option<String>,
    pub loss_amount: Option<f64>,
    pub void_note: Option<String>,
    pub queue_number: Option<i32>,
    pub is_voided: bool,
    pub is_upgraded: bool,
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
    pub mg_discount_amount: f64,
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
#[allow(dead_code)]
struct OrderRow {
    order_id: i64,
    receipt_number: String,
    table_name: Option<String>,
    zone_name: Option<String>,
    status: String,
    is_retail: bool,
    guest_count: Option<i32>,
    original_total: f64,
    total_amount: f64,
    subtotal: f64,
    paid_amount: f64,
    discount_amount: f64,
    surcharge_amount: f64,
    comp_total_amount: f64,
    order_manual_discount_amount: f64,
    order_manual_surcharge_amount: f64,
    order_rule_discount_amount: f64,
    order_rule_surcharge_amount: f64,
    member_id: Option<i64>,
    member_name: Option<String>,
    mg_discount_amount: f64,
    marketing_group_name: Option<String>,
    start_time: i64,
    end_time: Option<i64>,
    operator_name: Option<String>,
    void_type: Option<String>,
    loss_reason: Option<String>,
    loss_amount: Option<f64>,
    void_note: Option<String>,
    queue_number: Option<i32>,
    is_voided: bool,
    is_upgraded: bool,
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
    mg_discount_amount: f64,
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
        "SELECT id AS order_id, receipt_number, table_name, zone_name, status, is_retail, guest_count, original_total, total_amount, subtotal, paid_amount, discount_amount, surcharge_amount, comp_total_amount, order_manual_discount_amount, order_manual_surcharge_amount, order_rule_discount_amount, order_rule_surcharge_amount, member_id, member_name, mg_discount_amount, marketing_group_name, start_time, end_time, operator_name, void_type, loss_reason, loss_amount, void_note, queue_number, is_voided, is_upgraded FROM archived_order WHERE id = ?",
    )
    .bind(order_id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| RepoError::NotFound(format!("Order {order_id} not found")))?;

    // 2. Get items
    let item_rows: Vec<ItemRow> = sqlx::query_as::<_, ItemRow>(
        "SELECT id, instance_id, name, spec_name, category_id, category_name, price, quantity, unpaid_quantity, unit_price, line_total, discount_amount, surcharge_amount, rule_discount_amount, rule_surcharge_amount, mg_discount_amount, applied_rules, note, is_comped, tax, tax_rate FROM archived_order_item WHERE order_pk = ? ORDER BY id",
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
            options_map
                .entry(item_pk)
                .or_default()
                .push(OrderDetailOption {
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
                mg_discount_amount: row.mg_discount_amount,
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
        order_id: order.order_id,
        receipt_number: order.receipt_number,
        table_name: order.table_name,
        zone_name: order.zone_name,
        status: order.status,
        is_retail: order.is_retail,
        guest_count: order.guest_count,
        original_total: order.original_total,
        total: order.total_amount,
        subtotal: order.subtotal,
        paid_amount: order.paid_amount,
        total_discount: order.discount_amount,
        total_surcharge: order.surcharge_amount,
        comp_total_amount: order.comp_total_amount,
        order_manual_discount_amount: order.order_manual_discount_amount,
        order_manual_surcharge_amount: order.order_manual_surcharge_amount,
        order_rule_discount_amount: order.order_rule_discount_amount,
        order_rule_surcharge_amount: order.order_rule_surcharge_amount,
        member_id: order.member_id,
        member_name: order.member_name,
        mg_discount_amount: order.mg_discount_amount,
        marketing_group_name: order.marketing_group_name,
        start_time: order.start_time,
        end_time: order.end_time,
        operator_name: order.operator_name,
        void_type: order.void_type,
        loss_reason: order.loss_reason,
        loss_amount: order.loss_amount,
        void_note: order.void_note,
        queue_number: order.queue_number,
        is_voided: order.is_voided,
        is_upgraded: order.is_upgraded,
        items,
        payments,
        timeline,
    })
}

/// List archived order IDs not yet synced to cloud
pub async fn list_unsynced_archived_ids(pool: &SqlitePool, limit: i64) -> RepoResult<Vec<i64>> {
    let rows = sqlx::query_scalar::<_, i64>(
        "SELECT id FROM archived_order WHERE cloud_synced = 0 ORDER BY id LIMIT ?",
    )
    .bind(limit)
    .fetch_all(pool)
    .await?;

    Ok(rows)
}

/// Mark archived orders as synced to cloud
pub async fn mark_cloud_synced(pool: &SqlitePool, ids: &[i64]) -> RepoResult<()> {
    if ids.is_empty() {
        return Ok(());
    }
    let placeholders = ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    let sql = format!("UPDATE archived_order SET cloud_synced = 1 WHERE id IN ({placeholders})");
    let mut query = sqlx::query(&sql);
    for id in ids {
        query = query.bind(id);
    }
    query.execute(pool).await?;
    Ok(())
}

/// Slim down ITEMS_ADDED event data for cloud sync.
///
/// The raw event data contains full item snapshots (~1.5 KB each) with runtime fields
/// (instance_id, attribute_id, selected_specification, applied_rules, etc.) that are
/// redundant with the top-level `items` array. This reduces each to ~60 bytes by keeping
/// only `{name, quantity, price}` — sufficient for audit timeline display.
fn slim_items_added_data(data: &Option<String>) -> Option<String> {
    let raw = data.as_deref()?;
    let parsed: serde_json::Value = serde_json::from_str(raw).ok()?;
    let items = parsed.get("items")?.as_array()?;
    let slim_items: Vec<serde_json::Value> = items
        .iter()
        .filter_map(|item| {
            Some(serde_json::json!({
                "name": item.get("name")?,
                "quantity": item.get("quantity")?,
                "price": item.get("price")?,
            }))
        })
        .collect();
    serde_json::to_string(&serde_json::json!({
        "type": "ITEMS_ADDED",
        "items": slim_items,
    }))
    .ok()
}

/// Build full OrderDetailSync from archived tables for cloud sync
pub async fn build_order_detail_sync(
    pool: &SqlitePool,
    order_pk: i64,
) -> RepoResult<shared::cloud::OrderDetailSync> {
    use shared::cloud::{
        OrderDetailPayload, OrderDetailSync, OrderItemOptionSync, OrderItemSync, OrderPaymentSync,
        TaxDesglose,
    };

    // 1. Query archived_order JOIN chain_entry for hash data
    #[derive(sqlx::FromRow)]
    struct SyncOrderRow {
        order_id: i64,
        receipt_number: String,
        status: String,
        total_amount: f64,
        tax: f64,
        end_time: Option<i64>,
        prev_hash: String,
        curr_hash: String,
        created_at: i64,
        zone_name: Option<String>,
        table_name: Option<String>,
        is_retail: bool,
        guest_count: Option<i32>,
        original_total: f64,
        subtotal: f64,
        paid_amount: f64,
        discount_amount: f64,
        surcharge_amount: f64,
        comp_total_amount: f64,
        order_manual_discount_amount: f64,
        order_manual_surcharge_amount: f64,
        order_rule_discount_amount: f64,
        order_rule_surcharge_amount: f64,
        mg_discount_amount: f64,
        marketing_group_name: Option<String>,
        start_time: i64,
        operator_id: Option<i64>,
        operator_name: Option<String>,
        void_type: Option<String>,
        loss_reason: Option<String>,
        loss_amount: Option<f64>,
        void_note: Option<String>,
        member_id: Option<i64>,
        member_name: Option<String>,
        service_type: Option<String>,
        queue_number: Option<i32>,
        shift_id: Option<i64>,
        cloud_synced: bool,
        is_voided: bool,
        is_upgraded: bool,
        customer_nif: Option<String>,
        customer_nombre: Option<String>,
        customer_address: Option<String>,
        customer_email: Option<String>,
        customer_phone: Option<String>,
        order_applied_rules: Option<String>,
    }

    let order: SyncOrderRow = sqlx::query_as::<_, SyncOrderRow>(
        "SELECT ao.id AS order_id, ao.receipt_number, ao.status, ao.total_amount, ao.tax, ao.end_time, \
         ce.prev_hash, ce.curr_hash, ao.created_at, ao.zone_name, ao.table_name, ao.is_retail, ao.guest_count, \
         ao.original_total, ao.subtotal, ao.paid_amount, ao.discount_amount, ao.surcharge_amount, \
         ao.comp_total_amount, ao.order_manual_discount_amount, ao.order_manual_surcharge_amount, \
         ao.order_rule_discount_amount, ao.order_rule_surcharge_amount, ao.mg_discount_amount, ao.marketing_group_name, ao.start_time, \
         ao.operator_id, ao.operator_name, ao.void_type, ao.loss_reason, ao.loss_amount, ao.void_note, \
         ao.member_id, ao.member_name, ao.service_type, ao.queue_number, ao.shift_id, ao.cloud_synced, \
         ao.is_voided, ao.is_upgraded, \
         ao.customer_nif, ao.customer_nombre, ao.customer_address, ao.customer_email, ao.customer_phone, \
         ao.order_applied_rules \
         FROM archived_order ao \
         JOIN chain_entry ce ON ce.entry_type = 'ORDER' AND ce.entry_pk = ao.id \
         WHERE ao.id = ?",
    )
    .bind(order_pk)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| RepoError::NotFound(format!("Order {order_pk} not found")))?;

    // 2. Query items
    #[derive(sqlx::FromRow)]
    struct SyncItemRow {
        id: i64,
        instance_id: String,
        spec: String,
        name: String,
        spec_name: Option<String>,
        category_name: Option<String>,
        price: f64,
        quantity: i32,
        unit_price: f64,
        line_total: f64,
        discount_amount: f64,
        surcharge_amount: f64,
        tax: f64,
        tax_rate: i32,
        is_comped: bool,
        note: Option<String>,
        rule_discount_amount: f64,
        rule_surcharge_amount: f64,
        mg_discount_amount: f64,
        applied_rules: Option<String>,
    }

    let item_rows: Vec<SyncItemRow> = sqlx::query_as::<_, SyncItemRow>(
        "SELECT id, instance_id, spec, name, spec_name, category_name, price, quantity, unit_price, \
         line_total, discount_amount, surcharge_amount, tax, tax_rate, is_comped, note, \
         rule_discount_amount, rule_surcharge_amount, mg_discount_amount, applied_rules \
         FROM archived_order_item WHERE order_pk = ? ORDER BY id",
    )
    .bind(order_pk)
    .fetch_all(pool)
    .await?;

    // Guard: if detail sub-tables were cleaned (cloud_synced + no items), return error
    // so callers know to use the cloud-stored copy instead
    if item_rows.is_empty() && order.cloud_synced {
        return Err(RepoError::NotFound(format!(
            "Order {} detail already cleaned (cloud_synced), use cloud copy",
            order.order_id
        )));
    }

    // Batch load options
    let item_ids: Vec<i64> = item_rows.iter().map(|r| r.id).collect();
    let mut options_map: HashMap<i64, Vec<OrderItemOptionSync>> = HashMap::new();
    if !item_ids.is_empty() {
        let placeholders = item_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let sql = format!(
            "SELECT item_pk, attribute_name, option_name, price, quantity \
             FROM archived_order_item_option WHERE item_pk IN ({placeholders})"
        );
        let mut query = sqlx::query_as::<_, (i64, String, String, f64, i32)>(&sql);
        for id in &item_ids {
            query = query.bind(id);
        }
        let all_options = query.fetch_all(pool).await?;
        for (item_pk, attr, opt, price, qty) in all_options {
            options_map
                .entry(item_pk)
                .or_default()
                .push(OrderItemOptionSync {
                    attribute_name: attr,
                    option_name: opt,
                    price,
                    quantity: qty,
                });
        }
    }

    let items: Vec<OrderItemSync> = item_rows
        .into_iter()
        .map(|row| {
            let options = options_map.remove(&row.id).unwrap_or_default();
            // spec format: "product_id:spec_id"
            let product_source_id = row
                .spec
                .split(':')
                .next()
                .and_then(|s| s.parse::<i64>().ok());
            OrderItemSync {
                instance_id: row.instance_id,
                name: row.name,
                spec_name: row.spec_name,
                category_name: row.category_name,
                product_source_id,
                price: row.price,
                quantity: row.quantity,
                unit_price: row.unit_price,
                line_total: row.line_total,
                discount_amount: row.discount_amount,
                surcharge_amount: row.surcharge_amount,
                rule_discount_amount: row.rule_discount_amount,
                rule_surcharge_amount: row.rule_surcharge_amount,
                mg_discount_amount: row.mg_discount_amount,
                applied_rules: row
                    .applied_rules
                    .and_then(|s| serde_json::from_str(&s).ok())
                    .unwrap_or_default(),
                tax: row.tax,
                tax_rate: row.tax_rate,
                is_comped: row.is_comped,
                note: row.note,
                options,
            }
        })
        .collect();

    // 3. Query events (for Red Flags monitoring)
    let events: Vec<shared::cloud::OrderEventSync> = sqlx::query_as::<
        _,
        (
            i32,
            String,
            i64,
            Option<i64>,
            Option<String>,
            Option<String>,
        ),
    >(
        "SELECT seq, event_type, timestamp, operator_id, operator_name, data \
         FROM archived_order_event WHERE order_pk = ? ORDER BY seq",
    )
    .bind(order_pk)
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(
        |(seq, event_type, timestamp, operator_id, operator_name, data)| {
            // Slim down ITEMS_ADDED event data for cloud sync:
            // Full snapshots contain runtime fields (instance_id, attribute_id, etc.)
            // that are redundant with the top-level items array.
            // Keep only {name, quantity, price} per item for audit/timeline display.
            let slim_data = if event_type == "ITEMS_ADDED" {
                slim_items_added_data(&data)
            } else {
                data
            };
            shared::cloud::OrderEventSync {
                seq,
                event_type,
                timestamp,
                operator_id,
                operator_name,
                data: slim_data,
            }
        },
    )
    .collect();

    // 3b. Query last event hash (for cloud-side hash re-verification)
    let last_event_hash: Option<String> = sqlx::query_scalar::<_, String>(
        "SELECT curr_hash FROM archived_order_event WHERE order_pk = ? ORDER BY seq DESC LIMIT 1",
    )
    .bind(order_pk)
    .fetch_optional(pool)
    .await?;

    // 4. Query payments
    #[derive(sqlx::FromRow)]
    struct SyncPaymentRow {
        seq: i32,
        method: String,
        amount: f64,
        time: i64,
        cancelled: bool,
        cancel_reason: Option<String>,
        tendered: Option<f64>,
        change_amount: Option<f64>,
    }

    let payments: Vec<OrderPaymentSync> = sqlx::query_as::<_, SyncPaymentRow>(
        "SELECT seq, method, amount, time, cancelled, cancel_reason, tendered, change_amount \
         FROM archived_order_payment WHERE order_pk = ? ORDER BY seq",
    )
    .bind(order_pk)
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|p| OrderPaymentSync {
        seq: p.seq,
        method: p.method,
        amount: p.amount,
        timestamp: p.time,
        cancelled: p.cancelled,
        cancel_reason: p.cancel_reason,
        tendered: p.tendered,
        change_amount: p.change_amount,
    })
    .collect();

    // 4. Aggregate desglose from items (GROUP BY tax_rate) using rust_decimal
    use crate::order_money::to_decimal;
    use rust_decimal::Decimal;

    let mut desglose_map: HashMap<i32, (Decimal, Decimal)> = HashMap::new();
    for item in &items {
        // comped items have line_total=0 and tax=0, include them for completeness
        // (matches archived_order.tax which is computed from all items)
        let entry = desglose_map
            .entry(item.tax_rate)
            .or_insert((Decimal::ZERO, Decimal::ZERO));
        let line_total = to_decimal(item.line_total);
        let tax = to_decimal(item.tax);
        entry.0 += line_total - tax; // base_amount (Decimal precision)
        entry.1 += tax; // tax_amount (Decimal precision)
    }
    let desglose: Vec<TaxDesglose> = desglose_map
        .into_iter()
        .map(|(tax_rate, (base_amount, tax_amount))| TaxDesglose {
            tax_rate,
            base_amount,
            tax_amount,
        })
        .collect();

    Ok(OrderDetailSync {
        order_id: order.order_id,
        receipt_number: order.receipt_number,
        status: order.status,
        total_amount: order.total_amount,
        tax: order.tax,
        end_time: order.end_time,
        prev_hash: order.prev_hash,
        curr_hash: order.curr_hash,
        last_event_hash,
        created_at: order.created_at,
        desglose,
        detail: OrderDetailPayload {
            zone_name: order.zone_name,
            table_name: order.table_name,
            is_retail: order.is_retail,
            guest_count: order.guest_count,
            original_total: order.original_total,
            subtotal: order.subtotal,
            paid_amount: order.paid_amount,
            discount_amount: order.discount_amount,
            surcharge_amount: order.surcharge_amount,
            comp_total_amount: order.comp_total_amount,
            order_manual_discount_amount: order.order_manual_discount_amount,
            order_manual_surcharge_amount: order.order_manual_surcharge_amount,
            order_rule_discount_amount: order.order_rule_discount_amount,
            order_rule_surcharge_amount: order.order_rule_surcharge_amount,
            order_applied_rules: order
                .order_applied_rules
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default(),
            mg_discount_amount: order.mg_discount_amount,
            marketing_group_name: order.marketing_group_name,
            start_time: order.start_time,
            operator_id: order.operator_id,
            operator_name: order.operator_name,
            void_type: order.void_type.and_then(|s| s.parse().ok()),
            loss_reason: order.loss_reason.and_then(|s| s.parse().ok()),
            loss_amount: order.loss_amount,
            void_note: order.void_note,
            member_id: order.member_id,
            member_name: order.member_name,
            service_type: order.service_type.and_then(|s| s.parse().ok()),
            queue_number: order.queue_number.map(|n| n.to_string()),
            shift_id: order.shift_id,
            items,
            payments,
            events,
            is_voided: order.is_voided,
            is_upgraded: order.is_upgraded,
            customer_nif: order.customer_nif,
            customer_nombre: order.customer_nombre,
            customer_address: order.customer_address,
            customer_email: order.customer_email,
            customer_phone: order.customer_phone,
        },
    })
}

/// Delete detail sub-table rows for orders that have been synced to cloud
/// and are older than `cutoff_millis` (Unix ms).
/// Keeps the archived_order summary row intact.
/// Returns the total number of detail rows deleted.
pub async fn cleanup_synced_order_details(
    pool: &SqlitePool,
    cutoff_millis: i64,
) -> RepoResult<u64> {
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| RepoError::Database(e.to_string()))?;

    let mut total: u64 = 0;

    // 1. Delete item options (child of items)
    let r = sqlx::query(
        "DELETE FROM archived_order_item_option WHERE item_pk IN (\
            SELECT i.id FROM archived_order_item i \
            JOIN archived_order o ON i.order_pk = o.id \
            WHERE o.cloud_synced = 1 AND o.end_time < ?1\
        )",
    )
    .bind(cutoff_millis)
    .execute(&mut *tx)
    .await
    .map_err(|e| RepoError::Database(e.to_string()))?;
    total += r.rows_affected();

    // 2. Delete items
    let r = sqlx::query(
        "DELETE FROM archived_order_item WHERE order_pk IN (\
            SELECT id FROM archived_order WHERE cloud_synced = 1 AND end_time < ?1\
        )",
    )
    .bind(cutoff_millis)
    .execute(&mut *tx)
    .await
    .map_err(|e| RepoError::Database(e.to_string()))?;
    total += r.rows_affected();

    // 3. Delete payments
    let r = sqlx::query(
        "DELETE FROM archived_order_payment WHERE order_pk IN (\
            SELECT id FROM archived_order WHERE cloud_synced = 1 AND end_time < ?1\
        )",
    )
    .bind(cutoff_millis)
    .execute(&mut *tx)
    .await
    .map_err(|e| RepoError::Database(e.to_string()))?;
    total += r.rows_affected();

    // 4. Delete events
    let r = sqlx::query(
        "DELETE FROM archived_order_event WHERE order_pk IN (\
            SELECT id FROM archived_order WHERE cloud_synced = 1 AND end_time < ?1\
        )",
    )
    .bind(cutoff_millis)
    .execute(&mut *tx)
    .await
    .map_err(|e| RepoError::Database(e.to_string()))?;
    total += r.rows_affected();

    tx.commit()
        .await
        .map_err(|e| RepoError::Database(e.to_string()))?;

    Ok(total)
}

/// Archived event row for ITEMS_ADDED events (for kitchen reprint fallback)
#[derive(Debug, sqlx::FromRow)]
pub struct ArchivedItemsAddedEvent {
    pub event_id: i64,
    pub timestamp: i64,
    pub data: Option<String>,
}

/// Archived order metadata for rebuilding kitchen orders
#[derive(Debug, sqlx::FromRow)]
pub struct ArchivedOrderMeta {
    pub order_id: i64,
    pub receipt_number: String,
    pub table_name: Option<String>,
    pub zone_name: Option<String>,
    pub is_retail: bool,
    pub queue_number: Option<i32>,
}

/// Get ITEMS_ADDED events for an archived order by order_id (snowflake i64)
pub async fn get_items_added_events_by_order_id(
    pool: &SqlitePool,
    order_id: i64,
) -> RepoResult<(Option<ArchivedOrderMeta>, Vec<ArchivedItemsAddedEvent>)> {
    // 1. Find order pk and metadata by order_id
    let meta = sqlx::query_as::<_, ArchivedOrderMeta>(
        "SELECT id AS order_id, receipt_number, table_name, zone_name, is_retail, queue_number FROM archived_order WHERE id = ?",
    )
    .bind(order_id)
    .fetch_optional(pool)
    .await?;

    let Some(meta) = meta else {
        return Ok((None, vec![]));
    };

    let order_pk = sqlx::query_scalar::<_, i64>("SELECT id FROM archived_order WHERE id = ?")
        .bind(order_id)
        .fetch_one(pool)
        .await?;

    // 2. Get ITEMS_ADDED events
    let events = sqlx::query_as::<_, ArchivedItemsAddedEvent>(
        "SELECT id as event_id, timestamp, data FROM archived_order_event \
         WHERE order_pk = ? AND event_type = 'ITEMS_ADDED' ORDER BY seq",
    )
    .bind(order_pk)
    .fetch_all(pool)
    .await?;

    Ok((Some(meta), events))
}
