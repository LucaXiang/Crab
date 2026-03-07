//! Tenant management queries
//!
//! All queries enforce tenant_id isolation.

use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use sqlx::PgPool;

type BoxError = Box<dyn std::error::Error + Send + Sync>;

/// Convert Decimal to f64 for JSON serialization (NUMERIC → f64 boundary)
fn d(d: Decimal) -> f64 {
    d.to_f64().unwrap_or_default()
}

/// Tenant profile with subscription info
#[derive(serde::Serialize, sqlx::FromRow)]
pub struct TenantProfile {
    pub id: i64,
    pub email: String,
    pub name: Option<String>,
    pub status: String,
    pub created_at: i64,
}

pub async fn get_profile(pool: &PgPool, tenant_id: i64) -> Result<Option<TenantProfile>, BoxError> {
    let row: Option<TenantProfile> =
        sqlx::query_as("SELECT id, email, name, status, created_at FROM tenants WHERE id = $1")
            .bind(tenant_id)
            .fetch_optional(pool)
            .await?;
    Ok(row)
}

/// Subscription info
#[derive(serde::Serialize, sqlx::FromRow)]
pub struct SubscriptionInfo {
    pub id: String,
    pub status: String,
    pub plan: String,
    pub max_stores: i32,
    pub current_period_end: Option<i64>,
    pub cancel_at_period_end: bool,
    pub billing_interval: Option<String>,
    pub created_at: i64,
}

pub async fn get_subscription(
    pool: &PgPool,
    tenant_id: i64,
) -> Result<Option<SubscriptionInfo>, BoxError> {
    let row: Option<SubscriptionInfo> = sqlx::query_as(
        "SELECT id, status, plan, max_stores, current_period_end, cancel_at_period_end, billing_interval, created_at FROM subscriptions WHERE tenant_id = $1 ORDER BY created_at DESC LIMIT 1",
    )
    .bind(tenant_id)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

/// Store (edge-server) summary for tenant
#[derive(serde::Serialize, sqlx::FromRow)]
pub struct StoreSummary {
    pub id: i64,
    pub entity_id: String,
    pub alias: String,
    pub name: Option<String>,
    pub address: Option<String>,
    pub phone: Option<String>,
    pub nif: Option<String>,
    pub email: Option<String>,
    pub website: Option<String>,
    pub business_day_cutoff: Option<i32>,
    pub device_id: String,
    pub last_sync_at: Option<i64>,
    pub registered_at: i64,
    pub status: String,
}

pub async fn list_stores(pool: &PgPool, tenant_id: i64) -> Result<Vec<StoreSummary>, BoxError> {
    let rows: Vec<StoreSummary> = sqlx::query_as(
        r#"
        SELECT id, entity_id, alias, name, address, phone, nif, email, website,
               business_day_cutoff, device_id, last_sync_at, registered_at, status
        FROM stores
        WHERE tenant_id = $1 AND status = 'active'
        ORDER BY registered_at DESC
        "#,
    )
    .bind(tenant_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Archived order summary
#[derive(serde::Serialize, sqlx::FromRow)]
pub struct ArchivedOrderSummary {
    pub id: i64,
    pub source_id: i64,
    pub receipt_number: Option<String>,
    pub status: String,
    pub end_time: Option<i64>,
    pub total: Option<Decimal>,
    pub synced_at: i64,
}

pub async fn list_orders(
    pool: &PgPool,
    store_id: i64,
    tenant_id: i64,
    status_filter: Option<&str>,
    limit: i32,
    offset: i32,
) -> Result<Vec<ArchivedOrderSummary>, BoxError> {
    let rows: Vec<ArchivedOrderSummary> = if let Some(status) = status_filter {
        sqlx::query_as(
            r#"
            SELECT o.id, o.source_id, o.receipt_number, o.status, o.end_time, o.total, o.synced_at
            FROM store_archived_orders o
            WHERE o.store_id = $1 AND o.tenant_id = $2 AND o.status = $3
            ORDER BY o.end_time DESC NULLS LAST
            LIMIT $4 OFFSET $5
            "#,
        )
        .bind(store_id)
        .bind(tenant_id)
        .bind(status)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query_as(
            r#"
            SELECT o.id, o.source_id, o.receipt_number, o.status, o.end_time, o.total, o.synced_at
            FROM store_archived_orders o
            WHERE o.store_id = $1 AND o.tenant_id = $2
            ORDER BY o.end_time DESC NULLS LAST
            LIMIT $3 OFFSET $4
            "#,
        )
        .bind(store_id)
        .bind(tenant_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await?
    };
    Ok(rows)
}

/// Unified chain entry item — driven by store_chain_entries with JOINs for business data
#[derive(serde::Serialize, sqlx::FromRow)]
pub struct ChainEntryItem {
    pub entry_type: String,
    pub entry_id: i64,
    pub display_number: String,
    pub status: String,
    pub amount: Option<Decimal>,
    pub created_at: i64,
    pub original_order_id: Option<i64>,
    pub original_receipt: Option<String>,
}

pub async fn list_chain_entries(
    pool: &PgPool,
    store_id: i64,
    tenant_id: i64,
    limit: i32,
    offset: i32,
) -> Result<Vec<ChainEntryItem>, BoxError> {
    let rows: Vec<ChainEntryItem> = sqlx::query_as(
        r#"
        SELECT
            ce.entry_type,
            ce.entry_pk AS entry_id,
            CASE
                WHEN ce.entry_type = 'ORDER' OR ce.entry_type = 'ANULACION' OR ce.entry_type = 'UPGRADE'
                    THEN COALESCE(o.receipt_number, CAST(ce.entry_pk AS TEXT))
                WHEN ce.entry_type = 'CREDIT_NOTE'
                    THEN COALESCE(cn.credit_note_number, CAST(ce.entry_pk AS TEXT))
                WHEN ce.entry_type = 'BREAK'
                    THEN '#' || CAST(ce.id % 10000 AS TEXT)
                ELSE CAST(ce.entry_pk AS TEXT)
            END AS display_number,
            CASE
                WHEN ce.entry_type = 'ORDER' AND o.is_voided IS TRUE THEN 'ANULADA'
                WHEN ce.entry_type = 'ORDER' AND o.status = 'VOID' AND o.void_type = 'LOSS_SETTLED' THEN 'LOSS'
                WHEN ce.entry_type = 'ORDER' THEN COALESCE(o.status, 'UNKNOWN')
                WHEN ce.entry_type = 'CREDIT_NOTE' THEN 'REFUND'
                WHEN ce.entry_type = 'ANULACION' THEN 'ANULADA'
                WHEN ce.entry_type = 'UPGRADE' THEN 'UPGRADED'
                WHEN ce.entry_type = 'BREAK' THEN 'BREAK'
                ELSE 'UNKNOWN'
            END AS status,
            CASE
                WHEN ce.entry_type IN ('ORDER', 'ANULACION', 'UPGRADE') THEN o.total
                WHEN ce.entry_type = 'CREDIT_NOTE' THEN cn.total_credit
                ELSE NULL
            END AS amount,
            ce.created_at,
            CASE
                WHEN ce.entry_type = 'CREDIT_NOTE' THEN cn.original_order_id
                WHEN ce.entry_type = 'ANULACION' THEN ce.entry_pk
                ELSE NULL::BIGINT
            END AS original_order_id,
            CASE
                WHEN ce.entry_type = 'CREDIT_NOTE' THEN cn.original_receipt
                WHEN ce.entry_type = 'ANULACION' THEN o.receipt_number
                ELSE NULL::TEXT
            END AS original_receipt
        FROM store_chain_entries ce
        LEFT JOIN store_archived_orders o
            ON ce.entry_type IN ('ORDER', 'ANULACION', 'UPGRADE')
            AND o.store_id = ce.store_id AND o.tenant_id = ce.tenant_id AND o.order_id = ce.entry_pk
        LEFT JOIN store_credit_notes cn
            ON ce.entry_type = 'CREDIT_NOTE'
            AND cn.store_id = ce.store_id AND cn.tenant_id = ce.tenant_id AND cn.source_id = ce.entry_pk
        WHERE ce.store_id = $1 AND ce.tenant_id = $2
        ORDER BY ce.created_at DESC
        LIMIT $3 OFFSET $4
        "#,
    )
    .bind(store_id)
    .bind(tenant_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Credit note detail with items
#[derive(Debug, serde::Serialize)]
pub struct CreditNoteDetail {
    pub source_id: i64,
    pub credit_note_number: String,
    pub original_order_id: i64,
    pub original_receipt: String,
    pub subtotal_credit: Decimal,
    pub tax_credit: Decimal,
    pub total_credit: Decimal,
    pub refund_method: String,
    pub reason: String,
    pub note: Option<String>,
    pub operator_name: String,
    pub authorizer_name: Option<String>,
    pub created_at: i64,
    pub items: Vec<CreditNoteItemDetail>,
}

#[derive(Debug, serde::Serialize, sqlx::FromRow)]
pub struct CreditNoteItemDetail {
    pub original_instance_id: String,
    pub item_name: String,
    pub quantity: i32,
    pub unit_price: Decimal,
    pub line_credit: Decimal,
    pub tax_rate: i32,
    pub tax_credit: Decimal,
}

pub async fn get_credit_note_detail(
    pool: &PgPool,
    store_id: i64,
    tenant_id: i64,
    source_id: i64,
) -> Result<Option<CreditNoteDetail>, BoxError> {
    #[derive(sqlx::FromRow)]
    struct HeaderRow {
        id: i64,
        source_id: i64,
        credit_note_number: String,
        original_order_id: i64,
        original_receipt: String,
        subtotal_credit: Decimal,
        tax_credit: Decimal,
        total_credit: Decimal,
        refund_method: String,
        reason: String,
        note: Option<String>,
        operator_name: String,
        authorizer_name: Option<String>,
        created_at: i64,
    }

    let header = sqlx::query_as::<_, HeaderRow>(
        r#"
        SELECT id, source_id, credit_note_number, original_order_id, original_receipt,
               subtotal_credit, tax_credit, total_credit, refund_method, reason, note,
               operator_name, authorizer_name, created_at
        FROM store_credit_notes
        WHERE store_id = $1 AND tenant_id = $2 AND source_id = $3
        "#,
    )
    .bind(store_id)
    .bind(tenant_id)
    .bind(source_id)
    .fetch_optional(pool)
    .await?;

    let header = match header {
        Some(h) => h,
        None => return Ok(None),
    };

    let items = sqlx::query_as::<_, CreditNoteItemDetail>(
        r#"
        SELECT original_instance_id, item_name, quantity, unit_price, line_credit, tax_rate, tax_credit
        FROM store_credit_note_items
        WHERE credit_note_id = $1
        ORDER BY id
        "#,
    )
    .bind(header.id)
    .fetch_all(pool)
    .await?;

    Ok(Some(CreditNoteDetail {
        source_id: header.source_id,
        credit_note_number: header.credit_note_number,
        original_order_id: header.original_order_id,
        original_receipt: header.original_receipt,
        subtotal_credit: header.subtotal_credit,
        tax_credit: header.tax_credit,
        total_credit: header.total_credit,
        refund_method: header.refund_method,
        reason: header.reason,
        note: header.note,
        operator_name: header.operator_name,
        authorizer_name: header.authorizer_name,
        created_at: header.created_at,
        items,
    }))
}

/// Anulacion detail (order-layer: queried from store_archived_orders)
#[derive(Debug, serde::Serialize, sqlx::FromRow)]
pub struct AnulacionDetail {
    pub order_id: i64,
    pub receipt_number: String,
    pub total_amount: Decimal,
    pub is_voided: bool,
    pub created_at: i64,
}

pub async fn get_anulacion_detail(
    pool: &PgPool,
    store_id: i64,
    tenant_id: i64,
    order_id: i64,
) -> Result<Option<AnulacionDetail>, BoxError> {
    let row: Option<AnulacionDetail> = sqlx::query_as(
        r#"
        SELECT order_id, receipt_number, total AS total_amount, is_voided,
               COALESCE(end_time, synced_at) AS created_at
        FROM store_archived_orders
        WHERE store_id = $1 AND tenant_id = $2 AND order_id = $3 AND is_voided = true
        "#,
    )
    .bind(store_id)
    .bind(tenant_id)
    .bind(order_id)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

/// Upgrade detail (order-layer: queried from store_archived_orders)
#[derive(Debug, serde::Serialize, sqlx::FromRow)]
pub struct UpgradeDetail {
    pub order_id: i64,
    pub receipt_number: String,
    pub total_amount: Decimal,
    pub tax: Decimal,
    pub is_upgraded: bool,
    pub customer_nif: Option<String>,
    pub customer_nombre: Option<String>,
    pub customer_address: Option<String>,
    pub customer_email: Option<String>,
    pub customer_phone: Option<String>,
    pub created_at: i64,
}

pub async fn get_upgrade_detail(
    pool: &PgPool,
    store_id: i64,
    tenant_id: i64,
    order_id: i64,
) -> Result<Option<UpgradeDetail>, BoxError> {
    let row: Option<UpgradeDetail> = sqlx::query_as(
        r#"
        SELECT order_id, receipt_number, total AS total_amount, tax,
               is_upgraded, customer_nif, customer_nombre, customer_address,
               customer_email, customer_phone,
               COALESCE(end_time, synced_at) AS created_at
        FROM store_archived_orders
        WHERE store_id = $1 AND tenant_id = $2 AND order_id = $3 AND is_upgraded = true
        "#,
    )
    .bind(store_id)
    .bind(tenant_id)
    .bind(order_id)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

/// Daily report entry for Console stats
#[derive(serde::Serialize, sqlx::FromRow)]
pub struct DailyReportEntry {
    pub id: i64,
    pub business_date: String,
    pub net_revenue: f64,
    pub total_orders: i64,
    pub refund_amount: f64,
    pub refund_count: i64,
    pub auto_generated: bool,
    pub updated_at: i64,
}

pub async fn list_daily_reports(
    pool: &PgPool,
    store_id: i64,
    tenant_id: i64,
    from: Option<&str>,
    to: Option<&str>,
) -> Result<Vec<DailyReportEntry>, BoxError> {
    let rows: Vec<DailyReportEntry> = sqlx::query_as(
        r#"
        SELECT dr.id, dr.business_date, dr.net_revenue, dr.total_orders,
               dr.refund_amount, dr.refund_count, dr.auto_generated, dr.updated_at
        FROM store_daily_reports dr
        WHERE dr.store_id = $1 AND dr.tenant_id = $2
            AND ($3::TEXT IS NULL OR dr.business_date >= $3)
            AND ($4::TEXT IS NULL OR dr.business_date <= $4)
        ORDER BY dr.business_date DESC
        "#,
    )
    .bind(store_id)
    .bind(tenant_id)
    .bind(from)
    .bind(to)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Daily report detail with shift breakdowns
#[derive(Debug, serde::Serialize)]
pub struct DailyReportDetail {
    pub id: i64,
    pub business_date: String,
    pub net_revenue: f64,
    pub total_orders: i64,
    pub refund_amount: f64,
    pub refund_count: i64,
    pub auto_generated: bool,
    pub generated_at: Option<i64>,
    pub generated_by_id: Option<i64>,
    pub generated_by_name: Option<String>,
    pub note: Option<String>,
    pub shift_breakdowns: Vec<ShiftBreakdownDetail>,
    pub payment_breakdown: Vec<PaymentBreakdownEntry>,
}

#[derive(Debug, serde::Serialize, sqlx::FromRow)]
pub struct ShiftBreakdownDetail {
    pub shift_source_id: i64,
    pub operator_id: i64,
    pub operator_name: String,
    pub status: String,
    pub start_time: i64,
    pub end_time: Option<i64>,
    pub starting_cash: f64,
    pub expected_cash: f64,
    pub actual_cash: Option<f64>,
    pub cash_variance: Option<f64>,
    pub abnormal_close: bool,
    pub total_orders: i64,
    pub completed_orders: i64,
    pub void_orders: i64,
    pub total_sales: f64,
    pub total_paid: f64,
    pub void_amount: f64,
    pub total_tax: f64,
    pub total_discount: f64,
    pub total_surcharge: f64,
}

pub async fn get_daily_report_detail(
    pool: &PgPool,
    store_id: i64,
    tenant_id: i64,
    date: &str,
) -> Result<Option<DailyReportDetail>, BoxError> {
    // Main report
    #[derive(sqlx::FromRow)]
    struct ReportRow {
        id: i64,
        business_date: String,
        net_revenue: f64,
        total_orders: i64,
        refund_amount: f64,
        refund_count: i64,
        auto_generated: bool,
        generated_at: Option<i64>,
        generated_by_id: Option<i64>,
        generated_by_name: Option<String>,
        note: Option<String>,
    }

    let report: Option<ReportRow> = sqlx::query_as(
        r#"
        SELECT dr.id, dr.business_date, dr.net_revenue, dr.total_orders,
               dr.refund_amount, dr.refund_count, dr.auto_generated,
               dr.generated_at, dr.generated_by_id, dr.generated_by_name, dr.note
        FROM store_daily_reports dr
        WHERE dr.store_id = $1 AND dr.tenant_id = $2 AND dr.business_date = $3
        "#,
    )
    .bind(store_id)
    .bind(tenant_id)
    .bind(date)
    .fetch_optional(pool)
    .await?;

    let report = match report {
        Some(r) => r,
        None => return Ok(None),
    };

    let shift_breakdowns = sqlx::query_as::<_, ShiftBreakdownDetail>(
        r#"
        SELECT shift_source_id, operator_id, operator_name, status,
               start_time, end_time, starting_cash, expected_cash,
               actual_cash, cash_variance, abnormal_close,
               total_orders, completed_orders, void_orders,
               total_sales, total_paid, void_amount,
               total_tax, total_discount, total_surcharge
        FROM store_daily_report_shift_breakdown
        WHERE report_id = $1
        ORDER BY start_time
        "#,
    )
    .bind(report.id)
    .fetch_all(pool)
    .await?;

    // Derive time range from shifts for payment query
    let payment_breakdown = if shift_breakdowns.is_empty() {
        vec![]
    } else {
        let from = shift_breakdowns.iter().map(|s| s.start_time).min().unwrap();
        let to = shift_breakdowns
            .iter()
            .filter_map(|s| s.end_time)
            .max()
            .unwrap_or_else(shared::util::now_millis);
        sqlx::query_as::<_, PaymentBreakdownEntry>(
            r#"
            SELECT
                p.method,
                COALESCE(SUM(p.amount), 0)::DOUBLE PRECISION AS amount,
                COUNT(*) AS count
            FROM store_order_payments p
            JOIN store_archived_orders o ON o.id = p.order_id
            WHERE o.store_id = $1 AND o.tenant_id = $2
                AND o.end_time >= $3 AND o.end_time <= $4
                AND o.status = 'COMPLETED'
                AND o.is_voided IS NOT TRUE
                AND p.cancelled IS NOT TRUE
            GROUP BY p.method
            ORDER BY amount DESC
            "#,
        )
        .bind(store_id)
        .bind(tenant_id)
        .bind(from)
        .bind(to)
        .fetch_all(pool)
        .await
        .unwrap_or_default()
    };

    Ok(Some(DailyReportDetail {
        id: report.id,
        business_date: report.business_date,
        net_revenue: report.net_revenue,
        total_orders: report.total_orders,
        refund_amount: report.refund_amount,
        refund_count: report.refund_count,
        auto_generated: report.auto_generated,
        generated_at: report.generated_at,
        generated_by_id: report.generated_by_id,
        generated_by_name: report.generated_by_name,
        note: report.note,
        shift_breakdowns,
        payment_breakdown,
    }))
}

/// Get order detail by assembling from relational tables
pub async fn get_order_detail(
    pool: &PgPool,
    store_id: i64,
    tenant_id: i64,
    order_id: i64,
) -> Result<Option<shared::cloud::OrderDetailPayload>, BoxError> {
    // 1. Header (promoted scalar columns)
    #[derive(sqlx::FromRow)]
    struct HeaderRow {
        id: i64,
        start_time: Option<i64>,
        zone_name: Option<String>,
        table_name: Option<String>,
        is_retail: bool,
        original_total: Decimal,
        subtotal: Decimal,
        paid_amount: Decimal,
        surcharge_amount: Decimal,
        comp_total_amount: Decimal,
        order_manual_discount_amount: Decimal,
        order_manual_surcharge_amount: Decimal,
        order_rule_discount_amount: Decimal,
        order_rule_surcharge_amount: Decimal,
        operator_name: Option<String>,
        loss_reason: Option<String>,
        void_note: Option<String>,
        member_name: Option<String>,
        // from order header
        guest_count: Option<i32>,
        discount_amount: Decimal,
        void_type: Option<String>,
        loss_amount: Option<Decimal>,
        is_voided: Option<bool>,
        is_upgraded: Option<bool>,
        customer_nif: Option<String>,
        customer_nombre: Option<String>,
        customer_address: Option<String>,
        customer_email: Option<String>,
        customer_phone: Option<String>,
        mg_discount_amount: Decimal,
        marketing_group_name: Option<String>,
    }

    let header = sqlx::query_as::<_, HeaderRow>(
        r#"
        SELECT id, start_time, zone_name, table_name, is_retail,
               original_total, subtotal, paid_amount, surcharge_amount, comp_total_amount,
               order_manual_discount_amount, order_manual_surcharge_amount,
               order_rule_discount_amount, order_rule_surcharge_amount,
               operator_name, loss_reason, void_note, member_name,
               guest_count, discount_amount, void_type, loss_amount,
               is_voided, is_upgraded, customer_nif, customer_nombre,
               customer_address, customer_email, customer_phone,
               mg_discount_amount, marketing_group_name
        FROM store_archived_orders
        WHERE store_id = $1 AND tenant_id = $2 AND order_id = $3
        "#,
    )
    .bind(store_id)
    .bind(tenant_id)
    .bind(order_id)
    .fetch_optional(pool)
    .await?;

    let header = match header {
        Some(h) => h,
        None => return Ok(None),
    };
    let order_pk = header.id;

    // 2. Fetch children concurrently
    #[derive(sqlx::FromRow)]
    struct ItemRow {
        id: i64,
        instance_id: String,
        name: String,
        spec_name: Option<String>,
        category_name: Option<String>,
        product_source_id: Option<i64>,
        price: Decimal,
        quantity: i32,
        unit_price: Decimal,
        line_total: Decimal,
        discount_amount: Decimal,
        surcharge_amount: Decimal,
        tax: Decimal,
        tax_rate: i32,
        is_comped: bool,
        note: Option<String>,
        rule_discount_amount: Decimal,
        rule_surcharge_amount: Decimal,
        mg_discount_amount: Decimal,
    }

    #[derive(sqlx::FromRow)]
    struct OptionRow {
        item_id: i64,
        attribute_name: String,
        option_name: String,
        price: Decimal,
        quantity: i32,
    }

    #[derive(sqlx::FromRow)]
    struct PaymentRow {
        seq: i32,
        method: String,
        amount: Decimal,
        timestamp: i64,
        cancelled: bool,
    }

    #[derive(sqlx::FromRow)]
    struct EventRow {
        seq: i32,
        event_type: String,
        timestamp: i64,
        operator_id: Option<i64>,
        operator_name: Option<String>,
        data: Option<String>,
    }

    #[derive(sqlx::FromRow)]
    struct AdjustmentRow {
        item_id: Option<i64>,
        source_type: String,
        direction: String,
        rule_id: Option<i64>,
        rule_name: Option<String>,
        rule_receipt_name: Option<String>,
        adjustment_type: Option<String>,
        amount: Decimal,
        skipped: bool,
    }

    let (items_r, options_r, payments_r, events_r, adjustments_r) = tokio::join!(
        sqlx::query_as::<_, ItemRow>(
            r#"
            SELECT id, instance_id, name, spec_name, category_name, product_source_id,
                   price, quantity, unit_price, line_total, discount_amount,
                   surcharge_amount, tax, tax_rate, is_comped, note,
                   rule_discount_amount, rule_surcharge_amount, mg_discount_amount
            FROM store_order_items
            WHERE order_id = $1
            ORDER BY id
            "#,
        )
        .bind(order_pk)
        .fetch_all(pool),
        sqlx::query_as::<_, OptionRow>(
            r#"
            SELECT o.item_id, o.attribute_name, o.option_name, o.price, o.quantity
            FROM store_order_item_options o
            JOIN store_order_items i ON i.id = o.item_id
            WHERE i.order_id = $1
            ORDER BY o.id
            "#,
        )
        .bind(order_pk)
        .fetch_all(pool),
        sqlx::query_as::<_, PaymentRow>(
            r#"
            SELECT seq, method, amount, timestamp, cancelled
            FROM store_order_payments
            WHERE order_id = $1
            ORDER BY seq
            "#,
        )
        .bind(order_pk)
        .fetch_all(pool),
        sqlx::query_as::<_, EventRow>(
            r#"
            SELECT seq, event_type, timestamp, operator_id, operator_name, data
            FROM store_order_events
            WHERE order_id = $1
            ORDER BY seq
            "#,
        )
        .bind(order_pk)
        .fetch_all(pool),
        sqlx::query_as::<_, AdjustmentRow>(
            r#"
            SELECT item_id, source_type, direction, rule_id, rule_name,
                   rule_receipt_name, adjustment_type, amount, skipped
            FROM store_order_adjustments
            WHERE order_id = $1
            ORDER BY id
            "#,
        )
        .bind(order_pk)
        .fetch_all(pool),
    );

    let item_rows = items_r?;
    let option_rows = options_r?;
    let payments = payments_r?;
    let event_rows = events_r?;
    let adjustment_rows = adjustments_r?;

    // Group options by item_id
    let mut options_map: std::collections::HashMap<i64, Vec<shared::cloud::OrderItemOptionSync>> =
        std::collections::HashMap::new();
    for o in option_rows {
        options_map
            .entry(o.item_id)
            .or_default()
            .push(shared::cloud::OrderItemOptionSync {
                attribute_name: o.attribute_name,
                option_name: o.option_name,
                price: d(o.price),
                quantity: o.quantity,
            });
    }

    // Group PRICE_RULE adjustments by item_id → Vec<AppliedRule>
    // item_id = Some → item-level, item_id = None → order-level
    use shared::models::price_rule::{AdjustmentType, ProductScope, RuleType};
    let mut rules_map: std::collections::HashMap<
        i64,
        Vec<shared::order::applied_rule::AppliedRule>,
    > = std::collections::HashMap::new();
    let mut order_applied_rules: Vec<shared::order::applied_rule::AppliedRule> = Vec::new();

    let build_applied_rule = |adj: &AdjustmentRow| -> shared::order::applied_rule::AppliedRule {
        shared::order::applied_rule::AppliedRule {
            rule_id: adj.rule_id.unwrap_or(0),
            name: adj.rule_name.clone().unwrap_or_default(),
            receipt_name: adj.rule_receipt_name.clone(),
            rule_type: if adj.direction == "SURCHARGE" {
                RuleType::Surcharge
            } else {
                RuleType::Discount
            },
            adjustment_type: match adj.adjustment_type.as_deref() {
                Some("FIXED_AMOUNT") => AdjustmentType::FixedAmount,
                _ => AdjustmentType::Percentage,
            },
            product_scope: ProductScope::Global,
            zone_scope: "all".to_string(),
            adjustment_value: 0.0,
            calculated_amount: d(adj.amount),
            is_stackable: true,
            is_exclusive: false,
            skipped: adj.skipped,
        }
    };

    for adj in &adjustment_rows {
        if adj.source_type == "PRICE_RULE" {
            if let Some(item_id) = adj.item_id {
                rules_map
                    .entry(item_id)
                    .or_default()
                    .push(build_applied_rule(adj));
            } else {
                order_applied_rules.push(build_applied_rule(adj));
            }
        }
    }

    // Assemble items with options + applied_rules
    let items: Vec<shared::cloud::OrderItemSync> = item_rows
        .into_iter()
        .map(|i| {
            let item_id = i.id;
            shared::cloud::OrderItemSync {
                instance_id: i.instance_id,
                name: i.name,
                spec_name: i.spec_name,
                category_name: i.category_name,
                product_source_id: i.product_source_id,
                price: d(i.price),
                quantity: i.quantity,
                unit_price: d(i.unit_price),
                line_total: d(i.line_total),
                discount_amount: d(i.discount_amount),
                surcharge_amount: d(i.surcharge_amount),
                rule_discount_amount: d(i.rule_discount_amount),
                rule_surcharge_amount: d(i.rule_surcharge_amount),
                mg_discount_amount: d(i.mg_discount_amount),
                applied_rules: rules_map.remove(&item_id).unwrap_or_default(),
                tax: d(i.tax),
                tax_rate: i.tax_rate,
                is_comped: i.is_comped,
                note: i.note,
                options: options_map.remove(&item_id).unwrap_or_default(),
            }
        })
        .collect();

    let payments: Vec<shared::cloud::OrderPaymentSync> = payments
        .into_iter()
        .map(|p| shared::cloud::OrderPaymentSync {
            seq: p.seq,
            method: p.method,
            amount: d(p.amount),
            timestamp: p.timestamp,
            cancelled: p.cancelled,
            cancel_reason: None,
            tendered: None,
            change_amount: None,
        })
        .collect();

    let events: Vec<shared::cloud::OrderEventSync> = event_rows
        .into_iter()
        .map(|e| shared::cloud::OrderEventSync {
            seq: e.seq,
            event_type: e.event_type,
            timestamp: e.timestamp,
            operator_id: e.operator_id,
            operator_name: e.operator_name,
            data: e.data,
        })
        .collect();

    Ok(Some(shared::cloud::OrderDetailPayload {
        zone_name: header.zone_name,
        table_name: header.table_name,
        is_retail: header.is_retail,
        guest_count: header.guest_count,
        original_total: d(header.original_total),
        subtotal: d(header.subtotal),
        paid_amount: d(header.paid_amount),
        discount_amount: d(header.discount_amount),
        surcharge_amount: d(header.surcharge_amount),
        comp_total_amount: d(header.comp_total_amount),
        order_manual_discount_amount: d(header.order_manual_discount_amount),
        order_manual_surcharge_amount: d(header.order_manual_surcharge_amount),
        order_rule_discount_amount: d(header.order_rule_discount_amount),
        order_rule_surcharge_amount: d(header.order_rule_surcharge_amount),
        order_applied_rules,
        mg_discount_amount: d(header.mg_discount_amount),
        marketing_group_name: header.marketing_group_name,
        start_time: header.start_time.unwrap_or(0),
        operator_name: header.operator_name,
        void_type: header.void_type.and_then(|s| s.parse().ok()),
        loss_reason: header.loss_reason.and_then(|s| s.parse().ok()),
        loss_amount: header.loss_amount.map(d),
        void_note: header.void_note,
        member_name: header.member_name,
        service_type: None,
        operator_id: None,
        member_id: None,
        queue_number: None,
        shift_id: None,
        items,
        payments,
        events,
        is_voided: header.is_voided.unwrap_or(false),
        is_upgraded: header.is_upgraded.unwrap_or(false),
        customer_nif: header.customer_nif,
        customer_nombre: header.customer_nombre,
        customer_address: header.customer_address,
        customer_email: header.customer_email,
        customer_phone: header.customer_phone,
    }))
}

/// Get order desglose from store_order_desglose table (NUMERIC columns → Decimal direct)
pub async fn get_order_desglose(
    pool: &PgPool,
    store_id: i64,
    tenant_id: i64,
    order_id: i64,
) -> Result<Vec<shared::cloud::TaxDesglose>, BoxError> {
    #[derive(sqlx::FromRow)]
    struct DesgloseRow {
        tax_rate: i32,
        base_amount: rust_decimal::Decimal,
        tax_amount: rust_decimal::Decimal,
    }

    let rows = sqlx::query_as::<_, DesgloseRow>(
        r#"
        SELECT d.tax_rate, d.base_amount, d.tax_amount
        FROM store_order_desglose d
        JOIN store_archived_orders o ON o.id = d.order_id
        WHERE o.store_id = $1 AND o.tenant_id = $2 AND o.order_id = $3
        ORDER BY d.tax_rate
        "#,
    )
    .bind(store_id)
    .bind(tenant_id)
    .bind(order_id)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|r| shared::cloud::TaxDesglose {
            tax_rate: r.tax_rate,
            base_amount: r.base_amount,
            tax_amount: r.tax_amount,
        })
        .collect())
}

// ── Store overview statistics ──

/// Overview statistics for a time range
#[derive(Debug, serde::Serialize)]
pub struct StoreOverview {
    pub revenue: f64,
    pub net_revenue: f64,
    pub orders: i64,
    pub guests: i64,
    pub average_order_value: f64,
    pub per_guest_spend: f64,
    pub average_dining_minutes: f64,
    pub total_tax: f64,
    pub total_discount: f64,
    pub voided_orders: i64,
    pub voided_amount: f64,
    pub loss_orders: i64,
    pub loss_amount: f64,
    pub anulacion_count: i64,
    pub anulacion_amount: f64,
    pub refund_count: i64,
    pub refund_amount: f64,
    pub revenue_trend: Vec<RevenueTrendPoint>,
    pub tax_breakdown: Vec<TaxBreakdownEntry>,
    pub payment_breakdown: Vec<PaymentBreakdownEntry>,
    pub top_products: Vec<TopProductEntry>,
    pub category_sales: Vec<CategorySaleEntry>,
    pub tag_sales: Vec<TagSaleEntry>,
    pub refund_method_breakdown: Vec<RefundMethodEntry>,
    pub daily_trend: Vec<DailyTrendPoint>,
    pub service_type_breakdown: Vec<ServiceTypeEntry>,
    pub zone_sales: Vec<ZoneSaleEntry>,
    pub total_surcharge: f64,
    pub avg_items_per_order: f64,
    pub discount_breakdown: Vec<AdjustmentEntry>,
    pub surcharge_breakdown: Vec<AdjustmentEntry>,
}

/// Discount or surcharge line item in the breakdown
#[derive(Debug, serde::Serialize)]
pub struct AdjustmentEntry {
    /// Display name (rule name, or source key like "item_manual")
    pub name: String,
    /// Source type: "item_manual", "item_rule", "mg", "order_manual", "order_rule"
    pub source: String,
    pub amount: f64,
    pub order_count: i64,
}

#[derive(Debug, serde::Serialize, sqlx::FromRow)]
pub struct RevenueTrendPoint {
    pub hour: i32,
    pub revenue: f64,
    pub orders: i64,
}

#[derive(Debug, serde::Serialize, sqlx::FromRow)]
pub struct TaxBreakdownEntry {
    pub tax_rate: i32,
    pub base_amount: f64,
    pub tax_amount: f64,
}

#[derive(Debug, serde::Serialize, sqlx::FromRow)]
pub struct PaymentBreakdownEntry {
    pub method: String,
    pub amount: f64,
    pub count: i64,
}

#[derive(Debug, serde::Serialize, sqlx::FromRow)]
pub struct TopProductEntry {
    pub name: String,
    pub quantity: i64,
    pub revenue: f64,
}

#[derive(Debug, serde::Serialize, sqlx::FromRow)]
pub struct CategorySaleEntry {
    pub name: String,
    pub revenue: f64,
}

#[derive(Debug, serde::Serialize, sqlx::FromRow)]
pub struct TagSaleEntry {
    pub name: String,
    pub color: Option<String>,
    pub revenue: f64,
    pub quantity: i64,
}

#[derive(Debug, serde::Serialize, sqlx::FromRow)]
pub struct RefundMethodEntry {
    pub method: String,
    pub amount: f64,
    pub count: i64,
}

#[derive(Debug, serde::Serialize, sqlx::FromRow)]
pub struct DailyTrendPoint {
    pub date: String,
    pub revenue: f64,
    pub orders: i64,
}

#[derive(Debug, serde::Serialize, sqlx::FromRow)]
pub struct ServiceTypeEntry {
    pub service_type: String,
    pub revenue: f64,
    pub orders: i64,
}

#[derive(Debug, serde::Serialize, sqlx::FromRow)]
pub struct ZoneSaleEntry {
    pub zone_name: String,
    pub is_retail: bool,
    pub revenue: f64,
    pub orders: i64,
    pub guests: i64,
}

#[derive(Debug, serde::Serialize, sqlx::FromRow)]
pub struct AdjustmentBreakdownEntry {
    pub source_type: String,
    pub direction: String,
    pub rule_name: Option<String>,
    pub amount: f64,
    pub count: i64,
}

/// Compute store overview for a single store
pub async fn get_store_overview(
    pool: &PgPool,
    store_id: i64,
    tenant_id: i64,
    from: i64,
    to: i64,
) -> Result<StoreOverview, BoxError> {
    get_overview(pool, tenant_id, Some(store_id), from, to).await
}

/// Compute tenant-wide overview (all stores combined)
pub async fn get_tenant_overview(
    pool: &PgPool,
    tenant_id: i64,
    from: i64,
    to: i64,
) -> Result<StoreOverview, BoxError> {
    get_overview(pool, tenant_id, None, from, to).await
}

/// Parameterized overview: store_id=None → tenant-wide, Some → single store.
/// All queries enforce tenant_id isolation.
async fn get_overview(
    pool: &PgPool,
    tenant_id: i64,
    store_id: Option<i64>,
    from: i64,
    to: i64,
) -> Result<StoreOverview, BoxError> {
    // 1. Basic aggregation from store_archived_orders
    #[allow(clippy::type_complexity)]
    let overview: (f64, i64, i64, f64, f64, i64, f64, f64, i64, f64, f64) = sqlx::query_as(
        r#"
        SELECT
            COALESCE(SUM(CASE WHEN status = 'COMPLETED' AND is_voided IS NOT TRUE THEN total ELSE 0 END), 0)::DOUBLE PRECISION,
            COUNT(*) FILTER (WHERE status = 'COMPLETED' AND is_voided IS NOT TRUE),
            COUNT(*) FILTER (WHERE status = 'VOID' AND (void_type IS NULL OR void_type != 'LOSS_SETTLED')),
            COALESCE(SUM(CASE WHEN status = 'VOID' THEN COALESCE(total, 0) ELSE 0 END), 0)::DOUBLE PRECISION,
            COALESCE(SUM(CASE WHEN status = 'COMPLETED' AND is_voided IS NOT TRUE THEN COALESCE(tax, 0) ELSE 0 END), 0)::DOUBLE PRECISION,
            COALESCE(SUM(CASE WHEN status = 'COMPLETED' AND is_voided IS NOT TRUE THEN COALESCE(guest_count, 0) ELSE 0 END), 0)::BIGINT,
            COALESCE(SUM(CASE WHEN status = 'COMPLETED' AND is_voided IS NOT TRUE THEN COALESCE(discount_amount, 0) ELSE 0 END), 0)::DOUBLE PRECISION,
            COALESCE(AVG(CASE WHEN status = 'COMPLETED' AND is_voided IS NOT TRUE AND start_time IS NOT NULL AND end_time IS NOT NULL
                THEN (end_time - start_time) / 60000.0 END), 0)::DOUBLE PRECISION,
            COUNT(*) FILTER (WHERE status = 'VOID' AND void_type = 'LOSS_SETTLED'),
            COALESCE(SUM(CASE WHEN status = 'VOID' AND void_type = 'LOSS_SETTLED' THEN COALESCE(loss_amount, 0) ELSE 0 END), 0)::DOUBLE PRECISION,
            COALESCE(SUM(CASE WHEN status = 'VOID' AND (void_type IS NULL OR void_type != 'LOSS_SETTLED') THEN COALESCE(total, 0) ELSE 0 END), 0)::DOUBLE PRECISION
        FROM store_archived_orders
        WHERE tenant_id = $1
            AND ($2::BIGINT IS NULL OR store_id = $2)
            AND end_time >= $3 AND end_time < $4
        "#,
    )
    .bind(tenant_id)
    .bind(store_id)
    .bind(from)
    .bind(to)
    .fetch_one(pool)
    .await?;

    let (
        revenue,
        orders,
        voided_orders,
        _voided_amount_all,
        total_tax,
        guests,
        total_discount,
        average_dining_minutes,
        loss_orders,
        loss_amount,
        voided_amount,
    ) = overview;
    let average_order_value = if orders > 0 {
        revenue / orders as f64
    } else {
        0.0
    };
    let per_guest_spend = if guests > 0 {
        revenue / guests as f64
    } else {
        0.0
    };

    // 2-7. Run all independent queries concurrently
    // Determine if range spans multiple days (> 24h)
    let multi_day = (to - from) > 86_400_000;

    let (
        revenue_trend_r,
        tax_breakdown_r,
        payment_breakdown_r,
        top_products_r,
        category_sales_r,
        tag_sales_r,
        refund_agg_r,
        refund_method_r,
        daily_trend_r,
        service_type_r,
        zone_sales_r,
        surcharge_r,
        avg_items_r,
        anulacion_agg_r,
        adjustment_breakdown_r,
    ) = tokio::join!(
        // 2. Revenue trend (by hour of day)
        sqlx::query_as::<_, RevenueTrendPoint>(
            r#"
            SELECT
                EXTRACT(HOUR FROM TO_TIMESTAMP(end_time / 1000.0))::INTEGER AS hour,
                COALESCE(SUM(total), 0)::DOUBLE PRECISION AS revenue,
                COUNT(*) AS orders
            FROM store_archived_orders
            WHERE tenant_id = $1
                AND ($2::BIGINT IS NULL OR store_id = $2)
                AND end_time >= $3 AND end_time < $4
                AND status = 'COMPLETED'
                AND is_voided IS NOT TRUE
            GROUP BY hour
            ORDER BY hour
            "#,
        )
        .bind(tenant_id)
        .bind(store_id)
        .bind(from)
        .bind(to)
        .fetch_all(pool),
        // 3. Tax breakdown from store_order_desglose
        sqlx::query_as::<_, TaxBreakdownEntry>(
            r#"
            SELECT
                d.tax_rate,
                COALESCE(SUM(d.base_amount), 0)::DOUBLE PRECISION AS base_amount,
                COALESCE(SUM(d.tax_amount), 0)::DOUBLE PRECISION AS tax_amount
            FROM store_order_desglose d
            JOIN store_archived_orders o ON o.id = d.order_id
            WHERE o.tenant_id = $1
                AND ($2::BIGINT IS NULL OR o.store_id = $2)
                AND o.end_time >= $3 AND o.end_time < $4
                AND o.status = 'COMPLETED'
                AND o.is_voided IS NOT TRUE
            GROUP BY d.tax_rate
            ORDER BY d.tax_rate
            "#,
        )
        .bind(tenant_id)
        .bind(store_id)
        .bind(from)
        .bind(to)
        .fetch_all(pool),
        // 4. Payment breakdown from store_order_payments
        sqlx::query_as::<_, PaymentBreakdownEntry>(
            r#"
            SELECT
                p.method,
                COALESCE(SUM(p.amount), 0)::DOUBLE PRECISION AS amount,
                COUNT(*) AS count
            FROM store_order_payments p
            JOIN store_archived_orders o ON o.id = p.order_id
            WHERE o.tenant_id = $1
                AND ($2::BIGINT IS NULL OR o.store_id = $2)
                AND o.end_time >= $3 AND o.end_time < $4
                AND o.status = 'COMPLETED'
                AND o.is_voided IS NOT TRUE
                AND p.cancelled IS NOT TRUE
            GROUP BY p.method
            ORDER BY amount DESC
            "#,
        )
        .bind(tenant_id)
        .bind(store_id)
        .bind(from)
        .bind(to)
        .fetch_all(pool),
        // 5. Top products from store_order_items
        sqlx::query_as::<_, TopProductEntry>(
            r#"
            SELECT
                i.name,
                COALESCE(SUM(i.quantity), 0)::BIGINT AS quantity,
                COALESCE(SUM(i.line_total), 0)::DOUBLE PRECISION AS revenue
            FROM store_order_items i
            JOIN store_archived_orders o ON o.id = i.order_id
            WHERE o.tenant_id = $1
                AND ($2::BIGINT IS NULL OR o.store_id = $2)
                AND o.end_time >= $3 AND o.end_time < $4
                AND o.status = 'COMPLETED'
                AND o.is_voided IS NOT TRUE
            GROUP BY i.name
            ORDER BY quantity DESC
            LIMIT 10
            "#,
        )
        .bind(tenant_id)
        .bind(store_id)
        .bind(from)
        .bind(to)
        .fetch_all(pool),
        // 6. Category sales from store_order_items
        sqlx::query_as::<_, CategorySaleEntry>(
            r#"
            SELECT
                COALESCE(i.category_name, 'Sin categoría') AS name,
                COALESCE(SUM(i.line_total), 0)::DOUBLE PRECISION AS revenue
            FROM store_order_items i
            JOIN store_archived_orders o ON o.id = i.order_id
            WHERE o.tenant_id = $1
                AND ($2::BIGINT IS NULL OR o.store_id = $2)
                AND o.end_time >= $3 AND o.end_time < $4
                AND o.status = 'COMPLETED'
                AND o.is_voided IS NOT TRUE
            GROUP BY i.category_name
            ORDER BY revenue DESC
            LIMIT 10
            "#,
        )
        .bind(tenant_id)
        .bind(store_id)
        .bind(from)
        .bind(to)
        .fetch_all(pool),
        // 7. Tag sales — store_order_items → store_products → store_product_tag → store_tags
        sqlx::query_as::<_, TagSaleEntry>(
            r#"
            SELECT
                t.name,
                t.color,
                COALESCE(SUM(i.line_total), 0)::DOUBLE PRECISION AS revenue,
                COALESCE(SUM(i.quantity), 0)::BIGINT AS quantity
            FROM store_order_items i
            JOIN store_archived_orders o ON o.id = i.order_id
            JOIN store_products p ON p.source_id = i.product_source_id
                AND p.store_id = o.store_id
            JOIN store_product_tag pt ON pt.product_id = p.id
            JOIN store_tags t ON t.source_id = pt.tag_source_id
                AND t.store_id = o.store_id
            WHERE o.tenant_id = $1
                AND ($2::BIGINT IS NULL OR o.store_id = $2)
                AND o.end_time >= $3 AND o.end_time < $4
                AND o.status = 'COMPLETED'
                AND o.is_voided IS NOT TRUE
                AND i.product_source_id IS NOT NULL
            GROUP BY t.name, t.color
            ORDER BY revenue DESC
            LIMIT 10
            "#,
        )
        .bind(tenant_id)
        .bind(store_id)
        .bind(from)
        .bind(to)
        .fetch_all(pool),
        // 8. Refund aggregation from store_credit_notes
        sqlx::query_as::<_, (i64, f64)>(
            r#"
            SELECT
                COUNT(*),
                COALESCE(SUM(total_credit), 0)::DOUBLE PRECISION
            FROM store_credit_notes
            WHERE tenant_id = $1
                AND ($2::BIGINT IS NULL OR store_id = $2)
                AND created_at >= $3 AND created_at < $4
            "#,
        )
        .bind(tenant_id)
        .bind(store_id)
        .bind(from)
        .bind(to)
        .fetch_one(pool),
        // 9. Refund method breakdown
        sqlx::query_as::<_, RefundMethodEntry>(
            r#"
            SELECT
                refund_method AS method,
                COALESCE(SUM(total_credit), 0)::DOUBLE PRECISION AS amount,
                COUNT(*) AS count
            FROM store_credit_notes
            WHERE tenant_id = $1
                AND ($2::BIGINT IS NULL OR store_id = $2)
                AND created_at >= $3 AND created_at < $4
            GROUP BY refund_method
            ORDER BY amount DESC
            "#,
        )
        .bind(tenant_id)
        .bind(store_id)
        .bind(from)
        .bind(to)
        .fetch_all(pool),
        // 10. Daily trend from store_daily_reports (respects business_day_cutoff)
        // Use epoch ms directly in SQL: convert to date at DB level, ±1 day buffer
        // to handle timezone differences between client and server
        async {
            if !multi_day {
                return Ok::<Vec<DailyTrendPoint>, sqlx::Error>(vec![]);
            }
            sqlx::query_as::<_, DailyTrendPoint>(
                r#"
                SELECT
                    dr.business_date AS date,
                    COALESCE(SUM(dr.net_revenue), 0)::DOUBLE PRECISION AS revenue,
                    COALESCE(SUM(dr.total_orders), 0)::BIGINT AS orders
                FROM store_daily_reports dr
                WHERE dr.tenant_id = $1
                    AND ($2::BIGINT IS NULL OR dr.store_id = $2)
                    AND dr.business_date >= TO_CHAR(TO_TIMESTAMP(($3::BIGINT - 86400000) / 1000.0), 'YYYY-MM-DD')
                    AND dr.business_date <= TO_CHAR(TO_TIMESTAMP(($4::BIGINT + 86400000) / 1000.0), 'YYYY-MM-DD')
                GROUP BY dr.business_date
                ORDER BY dr.business_date
                "#,
            )
            .bind(tenant_id)
            .bind(store_id)
            .bind(from)
            .bind(to)
            .fetch_all(pool)
            .await
        },
        // 11. Service type breakdown from promoted column
        sqlx::query_as::<_, ServiceTypeEntry>(
            r#"
            SELECT
                COALESCE(service_type, 'DINE_IN') AS service_type,
                COALESCE(SUM(total), 0)::DOUBLE PRECISION AS revenue,
                COUNT(*) AS orders
            FROM store_archived_orders
            WHERE tenant_id = $1
                AND ($2::BIGINT IS NULL OR store_id = $2)
                AND end_time >= $3 AND end_time < $4
                AND status = 'COMPLETED'
                AND is_voided IS NOT TRUE
            GROUP BY COALESCE(service_type, 'DINE_IN')
            ORDER BY revenue DESC
            "#,
        )
        .bind(tenant_id)
        .bind(store_id)
        .bind(from)
        .bind(to)
        .fetch_all(pool),
        // 12. Zone sales from promoted column
        sqlx::query_as::<_, ZoneSaleEntry>(
            r#"
            SELECT
                COALESCE(zone_name, '') AS zone_name,
                COALESCE(BOOL_OR(is_retail), false) AS is_retail,
                COALESCE(SUM(total), 0)::DOUBLE PRECISION AS revenue,
                COUNT(*) AS orders,
                COALESCE(SUM(guest_count), 0)::BIGINT AS guests
            FROM store_archived_orders
            WHERE tenant_id = $1
                AND ($2::BIGINT IS NULL OR store_id = $2)
                AND end_time >= $3 AND end_time < $4
                AND status = 'COMPLETED'
                AND is_voided IS NOT TRUE
                AND zone_name IS NOT NULL
                AND zone_name != ''
            GROUP BY zone_name
            ORDER BY revenue DESC
            "#,
        )
        .bind(tenant_id)
        .bind(store_id)
        .bind(from)
        .bind(to)
        .fetch_all(pool),
        // 13. Total surcharge from promoted columns
        sqlx::query_as::<_, (f64,)>(
            r#"
            SELECT
                COALESCE(SUM(
                    COALESCE(order_manual_surcharge_amount, 0) +
                    COALESCE(order_rule_surcharge_amount, 0)
                ), 0)::DOUBLE PRECISION
            FROM store_archived_orders
            WHERE tenant_id = $1
                AND ($2::BIGINT IS NULL OR store_id = $2)
                AND end_time >= $3 AND end_time < $4
                AND status = 'COMPLETED'
                AND is_voided IS NOT TRUE
            "#,
        )
        .bind(tenant_id)
        .bind(store_id)
        .bind(from)
        .bind(to)
        .fetch_one(pool),
        // 14. Average items per order from store_order_items
        sqlx::query_as::<_, (f64,)>(
            r#"
            SELECT
                COALESCE(AVG(item_count), 0)::DOUBLE PRECISION
            FROM (
                SELECT COUNT(*) AS item_count
                FROM store_order_items i
                JOIN store_archived_orders o ON o.id = i.order_id
                WHERE o.tenant_id = $1
                    AND ($2::BIGINT IS NULL OR o.store_id = $2)
                    AND o.end_time >= $3 AND o.end_time < $4
                    AND o.status = 'COMPLETED'
                    AND o.is_voided IS NOT TRUE
                GROUP BY o.id
            ) sub
            "#,
        )
        .bind(tenant_id)
        .bind(store_id)
        .bind(from)
        .bind(to)
        .fetch_one(pool),
        // 15. Anulacion aggregate from store_anulaciones
        sqlx::query_as::<_, (i64, f64)>(
            r#"
            SELECT
                COUNT(*),
                COALESCE(SUM(o.total), 0)::DOUBLE PRECISION
            FROM store_anulaciones a
            JOIN store_archived_orders o ON o.store_id = a.store_id AND o.order_id = a.original_order_id
            WHERE a.tenant_id = $1
                AND ($2::BIGINT IS NULL OR a.store_id = $2)
                AND a.created_at >= $3 AND a.created_at < $4
            "#,
        )
        .bind(tenant_id)
        .bind(store_id)
        .bind(from)
        .bind(to)
        .fetch_one(pool),
        // 16. Adjustment breakdown from store_order_adjustments
        // Map raw source_type + item-level/order-level to frontend keys
        sqlx::query_as::<_, AdjustmentBreakdownEntry>(
            r#"
            SELECT
                CASE
                    WHEN a.source_type = 'MANUAL' AND a.item_id IS NULL THEN 'order_manual'
                    WHEN a.source_type = 'MANUAL' AND a.item_id IS NOT NULL THEN 'item_manual'
                    WHEN a.source_type = 'PRICE_RULE' AND a.item_id IS NULL THEN 'order_rule'
                    WHEN a.source_type = 'PRICE_RULE' AND a.item_id IS NOT NULL THEN 'item_rule'
                    WHEN a.source_type = 'MEMBER_GROUP' THEN 'mg'
                    WHEN a.source_type = 'COMP' THEN 'comp'
                    ELSE a.source_type
                END AS source_type,
                a.direction,
                a.rule_name,
                COALESCE(SUM(a.amount), 0)::DOUBLE PRECISION AS amount,
                COUNT(DISTINCT a.order_id)::BIGINT AS count
            FROM store_order_adjustments a
            JOIN store_archived_orders o ON o.id = a.order_id
            WHERE o.tenant_id = $1
                AND ($2::BIGINT IS NULL OR o.store_id = $2)
                AND o.end_time >= $3 AND o.end_time < $4
                AND o.status = 'COMPLETED'
                AND o.is_voided IS NOT TRUE
                AND a.skipped IS NOT TRUE
            GROUP BY 1, a.direction, a.rule_name
            ORDER BY amount DESC
            "#,
        )
        .bind(tenant_id)
        .bind(store_id)
        .bind(from)
        .bind(to)
        .fetch_all(pool),
    );

    let revenue_trend = revenue_trend_r?;
    let tax_breakdown = tax_breakdown_r.unwrap_or_default();
    let payment_breakdown = payment_breakdown_r.unwrap_or_default();
    let top_products = top_products_r.unwrap_or_default();
    let category_sales = category_sales_r.unwrap_or_default();
    let tag_sales = tag_sales_r.unwrap_or_default();
    let (refund_count, refund_amount) = refund_agg_r.unwrap_or((0, 0.0));
    let refund_method_breakdown = refund_method_r.unwrap_or_default();
    let daily_trend = daily_trend_r.unwrap_or_default();
    let service_type_breakdown = service_type_r.unwrap_or_default();
    let zone_sales = zone_sales_r.unwrap_or_default();
    let total_surcharge = surcharge_r.map(|(v,)| v).unwrap_or(0.0);
    let avg_items_per_order = avg_items_r.map(|(v,)| v).unwrap_or(0.0);
    let (anulacion_count, anulacion_amount) = anulacion_agg_r.unwrap_or((0, 0.0));
    let raw_adjustments = adjustment_breakdown_r.unwrap_or_default();
    let mut discount_breakdown = Vec::new();
    let mut surcharge_breakdown = Vec::new();
    for a in raw_adjustments {
        let entry = AdjustmentEntry {
            name: a.rule_name.unwrap_or_else(|| a.source_type.clone()),
            source: a.source_type,
            amount: a.amount,
            order_count: a.count,
        };
        if a.direction == "SURCHARGE" {
            surcharge_breakdown.push(entry);
        } else {
            discount_breakdown.push(entry);
        }
    }
    let net_revenue = revenue - refund_amount - anulacion_amount;

    Ok(StoreOverview {
        revenue,
        net_revenue,
        orders,
        guests,
        average_order_value,
        per_guest_spend,
        average_dining_minutes,
        total_tax,
        total_discount,
        voided_orders,
        voided_amount,
        loss_orders,
        loss_amount,
        anulacion_count,
        anulacion_amount,
        refund_count,
        refund_amount,
        revenue_trend,
        tax_breakdown,
        payment_breakdown,
        top_products,
        category_sales,
        tag_sales,
        refund_method_breakdown,
        daily_trend,
        service_type_breakdown,
        zone_sales,
        total_surcharge,
        avg_items_per_order,
        discount_breakdown,
        surcharge_breakdown,
    })
}

// ── Red Flags 监控 (Grouped) ──

#[derive(Debug, serde::Serialize)]
pub struct ItemFlags {
    pub removals: i64,
    pub comps: i64,
    pub uncomps: i64,
    pub price_modifications: i64,
}

#[derive(Debug, serde::Serialize)]
pub struct OrderFlags {
    pub voids: i64,
    pub discounts: i64,
    pub surcharges: i64,
    pub rule_skips: i64,
}

#[derive(Debug, serde::Serialize)]
pub struct PaymentFlags {
    pub cancellations: i64,
    pub refund_count: i64,
    pub refund_amount: f64,
}

#[derive(Debug, serde::Serialize)]
pub struct OperatorRedFlags {
    pub operator_id: i64,
    pub operator_name: String,
    pub removals: i64,
    pub comps: i64,
    pub uncomps: i64,
    pub price_modifications: i64,
    pub voids: i64,
    pub discounts: i64,
    pub surcharges: i64,
    pub rule_skips: i64,
    pub cancellations: i64,
    pub refund_count: i64,
    pub refund_amount: f64,
    pub total_flags: i64,
}

#[derive(Debug, serde::Serialize)]
pub struct RedFlagsResponse {
    pub item_flags: ItemFlags,
    pub order_flags: OrderFlags,
    pub payment_flags: PaymentFlags,
    pub operator_breakdown: Vec<OperatorRedFlags>,
}

pub async fn get_red_flags(
    pool: &PgPool,
    store_id: i64,
    tenant_id: i64,
    from: i64,
    to: i64,
) -> Result<RedFlagsResponse, BoxError> {
    // 1. Event counts by operator (9 event types)
    #[derive(sqlx::FromRow)]
    struct EventRow {
        operator_id: Option<i64>,
        operator_name: Option<String>,
        removals: i64,
        comps: i64,
        uncomps: i64,
        price_modifications: i64,
        voids: i64,
        discounts: i64,
        surcharges: i64,
        rule_skips: i64,
        cancellations: i64,
    }

    let event_rows: Vec<EventRow> = sqlx::query_as(
        r#"
        SELECT
            e.operator_id,
            e.operator_name,
            COUNT(*) FILTER (WHERE e.event_type = 'ITEM_REMOVED') AS removals,
            COUNT(*) FILTER (WHERE e.event_type = 'ITEM_COMPED') AS comps,
            COUNT(*) FILTER (WHERE e.event_type = 'ITEM_UNCOMPED') AS uncomps,
            COUNT(*) FILTER (WHERE e.event_type = 'ITEM_MODIFIED') AS price_modifications,
            COUNT(*) FILTER (WHERE e.event_type = 'ORDER_VOIDED') AS voids,
            COUNT(*) FILTER (WHERE e.event_type = 'ORDER_DISCOUNT_APPLIED') AS discounts,
            COUNT(*) FILTER (WHERE e.event_type = 'ORDER_SURCHARGE_APPLIED') AS surcharges,
            COUNT(*) FILTER (WHERE e.event_type = 'RULE_SKIP_TOGGLED') AS rule_skips,
            COUNT(*) FILTER (WHERE e.event_type = 'PAYMENT_CANCELLED') AS cancellations
        FROM store_order_events e
        JOIN store_archived_orders o ON o.id = e.order_id
        WHERE o.store_id = $1 AND o.tenant_id = $2
            AND o.end_time >= $3 AND o.end_time < $4
            AND e.event_type IN (
                'ITEM_REMOVED','ITEM_COMPED','ITEM_UNCOMPED','ITEM_MODIFIED',
                'ORDER_VOIDED','ORDER_DISCOUNT_APPLIED','ORDER_SURCHARGE_APPLIED',
                'RULE_SKIP_TOGGLED','PAYMENT_CANCELLED'
            )
        GROUP BY e.operator_id, e.operator_name
        ORDER BY COUNT(*) DESC
        "#,
    )
    .bind(store_id)
    .bind(tenant_id)
    .bind(from)
    .bind(to)
    .fetch_all(pool)
    .await?;

    // 2. Refund counts by operator_name (credit_notes have operator_name but not always operator_id)
    #[derive(sqlx::FromRow)]
    struct RefundRow {
        operator_name: String,
        refund_count: i64,
        refund_amount: f64,
    }

    let refund_rows: Vec<RefundRow> = sqlx::query_as(
        r#"
        SELECT
            COALESCE(cn.operator_name, '') AS operator_name,
            COUNT(*) AS refund_count,
            COALESCE(SUM(cn.total_credit), 0.0) AS refund_amount
        FROM store_credit_notes cn
        WHERE cn.store_id = $1 AND cn.tenant_id = $2
            AND cn.created_at >= $3 AND cn.created_at < $4
        GROUP BY cn.operator_name
        "#,
    )
    .bind(store_id)
    .bind(tenant_id)
    .bind(from)
    .bind(to)
    .fetch_all(pool)
    .await?;

    // 3. Build grouped summary + operator breakdown
    let mut item_flags = ItemFlags {
        removals: 0,
        comps: 0,
        uncomps: 0,
        price_modifications: 0,
    };
    let mut order_flags = OrderFlags {
        voids: 0,
        discounts: 0,
        surcharges: 0,
        rule_skips: 0,
    };
    let mut payment_flags = PaymentFlags {
        cancellations: 0,
        refund_count: 0,
        refund_amount: 0.0,
    };

    // Map: operator_name → OperatorRedFlags (use name as key since credit_notes may lack operator_id)
    use std::collections::HashMap;
    let mut op_map: HashMap<String, OperatorRedFlags> = HashMap::new();

    for row in &event_rows {
        item_flags.removals += row.removals;
        item_flags.comps += row.comps;
        item_flags.uncomps += row.uncomps;
        item_flags.price_modifications += row.price_modifications;
        order_flags.voids += row.voids;
        order_flags.discounts += row.discounts;
        order_flags.surcharges += row.surcharges;
        order_flags.rule_skips += row.rule_skips;
        payment_flags.cancellations += row.cancellations;

        let name = row.operator_name.clone().unwrap_or_default();
        let entry = op_map
            .entry(name.clone())
            .or_insert_with(|| OperatorRedFlags {
                operator_id: row.operator_id.unwrap_or(0),
                operator_name: name,
                removals: 0,
                comps: 0,
                uncomps: 0,
                price_modifications: 0,
                voids: 0,
                discounts: 0,
                surcharges: 0,
                rule_skips: 0,
                cancellations: 0,
                refund_count: 0,
                refund_amount: 0.0,
                total_flags: 0,
            });
        entry.removals += row.removals;
        entry.comps += row.comps;
        entry.uncomps += row.uncomps;
        entry.price_modifications += row.price_modifications;
        entry.voids += row.voids;
        entry.discounts += row.discounts;
        entry.surcharges += row.surcharges;
        entry.rule_skips += row.rule_skips;
        entry.cancellations += row.cancellations;
    }

    for row in &refund_rows {
        payment_flags.refund_count += row.refund_count;
        payment_flags.refund_amount += row.refund_amount;

        let entry = op_map
            .entry(row.operator_name.clone())
            .or_insert_with(|| OperatorRedFlags {
                operator_id: 0,
                operator_name: row.operator_name.clone(),
                removals: 0,
                comps: 0,
                uncomps: 0,
                price_modifications: 0,
                voids: 0,
                discounts: 0,
                surcharges: 0,
                rule_skips: 0,
                cancellations: 0,
                refund_count: 0,
                refund_amount: 0.0,
                total_flags: 0,
            });
        entry.refund_count += row.refund_count;
        entry.refund_amount += row.refund_amount;
    }

    // Calculate total_flags and collect
    let mut operator_breakdown: Vec<OperatorRedFlags> = op_map
        .into_values()
        .map(|mut op| {
            op.total_flags = op.removals
                + op.comps
                + op.uncomps
                + op.price_modifications
                + op.voids
                + op.discounts
                + op.surcharges
                + op.rule_skips
                + op.cancellations
                + op.refund_count;
            op
        })
        .collect();
    operator_breakdown.sort_by(|a, b| b.total_flags.cmp(&a.total_flags));

    Ok(RedFlagsResponse {
        item_flags,
        order_flags,
        payment_flags,
        operator_breakdown,
    })
}

// ── Red Flags Event Log ──

#[derive(Debug, serde::Serialize)]
pub struct RedFlagLogEntry {
    pub timestamp: i64,
    pub event_type: String,
    pub operator_id: i64,
    pub operator_name: String,
    pub receipt_number: String,
    pub order_id: i64,
    pub detail: Option<String>,
}

#[derive(Debug, serde::Serialize)]
pub struct RedFlagLogResponse {
    pub entries: Vec<RedFlagLogEntry>,
    pub total: i64,
    pub page: i32,
    pub per_page: i32,
}

#[allow(clippy::too_many_arguments)]
pub async fn get_red_flag_log(
    pool: &PgPool,
    store_id: i64,
    tenant_id: i64,
    from: i64,
    to: i64,
    event_type: Option<&str>,
    operator_id: Option<i64>,
    page: i32,
) -> Result<RedFlagLogResponse, BoxError> {
    let per_page: i32 = 50;
    let offset = (page.max(1) - 1) * per_page;

    let mut entries: Vec<RedFlagLogEntry> = Vec::new();

    // 1. Order events (unless only REFUND requested)
    if event_type.is_none_or(|et| et != "REFUND") {
        let mut sql = String::from(
            r#"SELECT e.timestamp, e.event_type,
                      COALESCE(e.operator_id, 0) AS operator_id,
                      COALESCE(e.operator_name, '') AS operator_name,
                      COALESCE(o.receipt_number, '') AS receipt_number,
                      o.source_id AS order_id,
                      e.data AS detail
               FROM store_order_events e
               JOIN store_archived_orders o ON o.id = e.order_id
               WHERE o.store_id = $1 AND o.tenant_id = $2
                 AND o.end_time >= $3 AND o.end_time < $4
                 AND e.event_type IN (
                     'ITEM_REMOVED','ITEM_COMPED','ITEM_UNCOMPED','ITEM_MODIFIED',
                     'ORDER_VOIDED','ORDER_DISCOUNT_APPLIED','ORDER_SURCHARGE_APPLIED',
                     'RULE_SKIP_TOGGLED','PAYMENT_CANCELLED'
                 )"#,
        );
        if let Some(et) = event_type {
            sql.push_str(&format!(" AND e.event_type = '{et}'"));
        }
        if let Some(op) = operator_id {
            sql.push_str(&format!(" AND e.operator_id = {op}"));
        }

        #[allow(clippy::type_complexity)]
        let rows: Vec<(i64, String, i64, String, String, i64, Option<String>)> =
            sqlx::query_as(&sql)
                .bind(store_id)
                .bind(tenant_id)
                .bind(from)
                .bind(to)
                .fetch_all(pool)
                .await?;

        for (ts, etype, op_id, op_name, receipt, oid, data) in rows {
            entries.push(RedFlagLogEntry {
                timestamp: ts,
                event_type: etype,
                operator_id: op_id,
                operator_name: op_name,
                receipt_number: receipt,
                order_id: oid,
                detail: data,
            });
        }
    }

    // 2. Refunds from credit_notes
    if event_type.is_none_or(|et| et == "REFUND") {
        let mut sql = String::from(
            r#"SELECT cn.created_at, 0::BIGINT AS operator_id,
                      COALESCE(cn.operator_name, '') AS operator_name,
                      COALESCE(o.receipt_number, '') AS receipt_number,
                      o.source_id AS order_id,
                      cn.total_credit, cn.reason
               FROM store_credit_notes cn
               JOIN store_archived_orders o ON o.id = cn.original_order_id
               WHERE cn.store_id = $1 AND cn.tenant_id = $2
                 AND cn.created_at >= $3 AND cn.created_at < $4"#,
        );
        if let Some(op) = operator_id {
            sql.push_str(&format!(" AND cn.operator_name = (SELECT name FROM store_employees WHERE source_id = {op} LIMIT 1)"));
        }

        let rows: Vec<(i64, i64, String, String, i64, f64, String)> = sqlx::query_as(&sql)
            .bind(store_id)
            .bind(tenant_id)
            .bind(from)
            .bind(to)
            .fetch_all(pool)
            .await?;

        for (ts, op_id, op_name, receipt, oid, amount, reason) in rows {
            entries.push(RedFlagLogEntry {
                timestamp: ts,
                event_type: "REFUND".to_string(),
                operator_id: op_id,
                operator_name: op_name,
                receipt_number: receipt,
                order_id: oid,
                detail: Some(format!("{amount:.2} - {reason}")),
            });
        }
    }

    // Sort by timestamp DESC, then paginate
    entries.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    let total = entries.len() as i64;
    let paginated: Vec<RedFlagLogEntry> = entries
        .into_iter()
        .skip(offset as usize)
        .take(per_page as usize)
        .collect();

    Ok(RedFlagLogResponse {
        entries: paginated,
        total,
        page: page.max(1),
        per_page,
    })
}

/// Verify edge-server belongs to tenant, return store_id
pub async fn verify_store_ownership(
    pool: &PgPool,
    store_id: i64,
    tenant_id: i64,
) -> Result<Option<i64>, BoxError> {
    let row: Option<(i64,)> =
        sqlx::query_as("SELECT id FROM stores WHERE id = $1 AND tenant_id = $2")
            .bind(store_id)
            .bind(tenant_id)
            .fetch_optional(pool)
            .await?;
    Ok(row.map(|r| r.0))
}

/// Soft-delete a store and deactivate its associated server activation
pub async fn soft_delete_store(
    pool: &PgPool,
    store_id: i64,
    tenant_id: i64,
    now: i64,
) -> Result<(), BoxError> {
    let mut tx = pool.begin().await?;

    // 获取 entity_id
    let entity: Option<(String,)> = sqlx::query_as(
        "SELECT entity_id FROM stores WHERE id = $1 AND tenant_id = $2 AND status = 'active'",
    )
    .bind(store_id)
    .bind(tenant_id)
    .fetch_optional(&mut *tx)
    .await?;

    let entity_id = entity.ok_or("Store not found")?.0;

    // 软删除门店
    sqlx::query("UPDATE stores SET status = 'deleted', deleted_at = $1 WHERE id = $2")
        .bind(now)
        .bind(store_id)
        .execute(&mut *tx)
        .await?;

    // 停用关联 server activation
    sqlx::query("UPDATE activations SET status = 'deactivated', deactivated_at = $1 WHERE entity_id = $2 AND status = 'active'")
        .bind(now)
        .bind(&entity_id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;
    Ok(())
}

/// Get entity_id for a store
pub async fn get_store_entity_id(
    pool: &PgPool,
    store_id: i64,
    tenant_id: i64,
) -> Result<Option<String>, BoxError> {
    let row: Option<(String,)> =
        sqlx::query_as("SELECT entity_id FROM stores WHERE id = $1 AND tenant_id = $2")
            .bind(store_id)
            .bind(tenant_id)
            .fetch_optional(pool)
            .await?;
    Ok(row.map(|r| r.0))
}

/// Credit note summary for order detail view
#[derive(serde::Serialize, sqlx::FromRow)]
pub struct CreditNoteSummary {
    pub source_id: i64,
    pub credit_note_number: String,
    pub total_credit: Decimal,
    pub refund_method: String,
    pub reason: String,
    pub operator_name: String,
    pub created_at: i64,
}

pub async fn list_credit_notes_by_order(
    pool: &PgPool,
    store_id: i64,
    tenant_id: i64,
    order_id: i64,
) -> Result<Vec<CreditNoteSummary>, BoxError> {
    let rows: Vec<CreditNoteSummary> = sqlx::query_as(
        r#"
        SELECT source_id, credit_note_number, total_credit, refund_method, reason, operator_name, created_at
        FROM store_credit_notes
        WHERE store_id = $1 AND tenant_id = $2 AND original_order_id = $3
        ORDER BY created_at DESC
        "#,
    )
    .bind(store_id)
    .bind(tenant_id)
    .bind(order_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

// ── Shifts ──

/// Shift entry returned to console
#[derive(serde::Serialize, sqlx::FromRow)]
pub struct ShiftEntry {
    pub source_id: i64,
    pub operator_id: i64,
    pub operator_name: String,
    pub status: String,
    pub start_time: i64,
    pub end_time: Option<i64>,
    pub starting_cash: f64,
    pub expected_cash: f64,
    pub actual_cash: Option<f64>,
    pub cash_variance: Option<f64>,
    pub abnormal_close: bool,
    pub note: Option<String>,
}

pub async fn list_shifts(
    pool: &PgPool,
    store_id: i64,
    tenant_id: i64,
) -> Result<Vec<ShiftEntry>, BoxError> {
    let rows: Vec<ShiftEntry> = sqlx::query_as(
        r#"
        SELECT source_id, operator_id, operator_name, status,
               start_time, end_time, starting_cash, expected_cash,
               actual_cash, cash_variance, abnormal_close, note
        FROM store_shifts
        WHERE store_id = $1 AND tenant_id = $2
        ORDER BY start_time DESC
        LIMIT 100
        "#,
    )
    .bind(store_id)
    .bind(tenant_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}
