//! Tenant management queries
//!
//! All queries enforce tenant_id isolation.

use sqlx::PgPool;

type BoxError = Box<dyn std::error::Error + Send + Sync>;

/// Tenant profile with subscription info
#[derive(serde::Serialize, sqlx::FromRow)]
pub struct TenantProfile {
    pub id: String,
    pub email: String,
    pub name: Option<String>,
    pub status: String,
    pub created_at: i64,
}

pub async fn get_profile(
    pool: &PgPool,
    tenant_id: &str,
) -> Result<Option<TenantProfile>, BoxError> {
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
    pub max_edge_servers: i32,
    pub max_clients: i32,
    pub current_period_end: Option<i64>,
    pub cancel_at_period_end: bool,
    pub billing_interval: Option<String>,
    pub created_at: i64,
}

pub async fn get_subscription(
    pool: &PgPool,
    tenant_id: &str,
) -> Result<Option<SubscriptionInfo>, BoxError> {
    let row: Option<SubscriptionInfo> = sqlx::query_as(
        "SELECT id, status, plan, max_edge_servers, max_clients, current_period_end, cancel_at_period_end, billing_interval, created_at FROM subscriptions WHERE tenant_id = $1 ORDER BY created_at DESC LIMIT 1",
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
    pub name: Option<String>,
    pub address: Option<String>,
    pub phone: Option<String>,
    pub nif: Option<String>,
    pub email: Option<String>,
    pub website: Option<String>,
    pub business_day_cutoff: Option<String>,
    pub device_id: String,
    pub last_sync_at: Option<i64>,
    pub registered_at: i64,
}

pub async fn list_stores(pool: &PgPool, tenant_id: &str) -> Result<Vec<StoreSummary>, BoxError> {
    let rows: Vec<StoreSummary> = sqlx::query_as(
        r#"
        SELECT id, entity_id, name, address, phone, nif, email, website,
               business_day_cutoff, device_id, last_sync_at, registered_at
        FROM stores
        WHERE tenant_id = $1
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
    pub source_id: String,
    pub receipt_number: Option<String>,
    pub status: String,
    pub end_time: Option<i64>,
    pub total: Option<f64>,
    pub synced_at: i64,
}

pub async fn list_orders(
    pool: &PgPool,
    store_id: i64,
    tenant_id: &str,
    status_filter: Option<&str>,
    limit: i32,
    offset: i32,
) -> Result<Vec<ArchivedOrderSummary>, BoxError> {
    let rows: Vec<ArchivedOrderSummary> = if let Some(status) = status_filter {
        sqlx::query_as(
            r#"
            SELECT id, source_id, receipt_number, status, end_time, total, synced_at
            FROM store_archived_orders
            WHERE store_id = $1 AND tenant_id = $2 AND status = $3
            ORDER BY end_time DESC NULLS LAST
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
            SELECT id, source_id, receipt_number, status, end_time, total, synced_at
            FROM store_archived_orders
            WHERE store_id = $1 AND tenant_id = $2
            ORDER BY end_time DESC NULLS LAST
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

/// Daily report entry for Console stats
#[derive(serde::Serialize, sqlx::FromRow)]
pub struct DailyReportEntry {
    pub id: i64,
    pub business_date: String,
    pub total_orders: i64,
    pub completed_orders: i64,
    pub void_orders: i64,
    pub total_sales: f64,
    pub total_paid: f64,
    pub total_unpaid: f64,
    pub void_amount: f64,
    pub total_tax: f64,
    pub total_discount: f64,
    pub total_surcharge: f64,
    pub updated_at: i64,
}

pub async fn list_daily_reports(
    pool: &PgPool,
    store_id: i64,
    tenant_id: &str,
    from: Option<&str>,
    to: Option<&str>,
) -> Result<Vec<DailyReportEntry>, BoxError> {
    let rows: Vec<DailyReportEntry> = sqlx::query_as(
        r#"
        SELECT dr.id, dr.business_date, dr.total_orders, dr.completed_orders, dr.void_orders,
               dr.total_sales, dr.total_paid, dr.total_unpaid, dr.void_amount,
               dr.total_tax, dr.total_discount, dr.total_surcharge, dr.updated_at
        FROM store_daily_reports dr
        JOIN stores s ON s.id = dr.store_id
        WHERE dr.store_id = $1
            AND s.tenant_id = $2
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

/// Daily report detail with all breakdowns
#[derive(Debug, serde::Serialize)]
pub struct DailyReportDetail {
    pub id: i64,
    pub business_date: String,
    pub total_orders: i64,
    pub completed_orders: i64,
    pub void_orders: i64,
    pub total_sales: f64,
    pub total_paid: f64,
    pub total_unpaid: f64,
    pub void_amount: f64,
    pub total_tax: f64,
    pub total_discount: f64,
    pub total_surcharge: f64,
    pub generated_at: Option<i64>,
    pub generated_by_id: Option<i64>,
    pub generated_by_name: Option<String>,
    pub note: Option<String>,
    pub tax_breakdowns: Vec<TaxBreakdownDetail>,
    pub payment_breakdowns: Vec<PaymentBreakdownDetail>,
    pub shift_breakdowns: Vec<ShiftBreakdownDetail>,
}

#[derive(Debug, serde::Serialize, sqlx::FromRow)]
pub struct TaxBreakdownDetail {
    pub tax_rate: i32,
    pub net_amount: f64,
    pub tax_amount: f64,
    pub gross_amount: f64,
    pub order_count: i64,
}

#[derive(Debug, serde::Serialize, sqlx::FromRow)]
pub struct PaymentBreakdownDetail {
    pub method: String,
    pub amount: f64,
    pub count: i64,
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
    tenant_id: &str,
    date: &str,
) -> Result<Option<DailyReportDetail>, BoxError> {
    // Main report
    #[derive(sqlx::FromRow)]
    struct ReportRow {
        id: i64,
        business_date: String,
        total_orders: i64,
        completed_orders: i64,
        void_orders: i64,
        total_sales: f64,
        total_paid: f64,
        total_unpaid: f64,
        void_amount: f64,
        total_tax: f64,
        total_discount: f64,
        total_surcharge: f64,
        generated_at: Option<i64>,
        generated_by_id: Option<i64>,
        generated_by_name: Option<String>,
        note: Option<String>,
    }

    let report: Option<ReportRow> = sqlx::query_as(
        r#"
        SELECT dr.id, dr.business_date, dr.total_orders, dr.completed_orders, dr.void_orders,
               dr.total_sales, dr.total_paid, dr.total_unpaid, dr.void_amount,
               dr.total_tax, dr.total_discount, dr.total_surcharge,
               dr.generated_at, dr.generated_by_id, dr.generated_by_name, dr.note
        FROM store_daily_reports dr
        JOIN stores s ON s.id = dr.store_id
        WHERE dr.store_id = $1 AND s.tenant_id = $2 AND dr.business_date = $3
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

    // Fetch all breakdowns concurrently
    let (tax_breakdowns, payment_breakdowns, shift_breakdowns) = tokio::join!(
        sqlx::query_as::<_, TaxBreakdownDetail>(
            r#"
            SELECT tax_rate, net_amount, tax_amount, gross_amount, order_count
            FROM store_daily_report_tax_breakdown
            WHERE report_id = $1
            ORDER BY tax_rate
            "#,
        )
        .bind(report.id)
        .fetch_all(pool),
        sqlx::query_as::<_, PaymentBreakdownDetail>(
            r#"
            SELECT method, amount, count
            FROM store_daily_report_payment_breakdown
            WHERE report_id = $1
            ORDER BY amount DESC
            "#,
        )
        .bind(report.id)
        .fetch_all(pool),
        sqlx::query_as::<_, ShiftBreakdownDetail>(
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
        .fetch_all(pool),
    );
    let tax_breakdowns = tax_breakdowns?;
    let payment_breakdowns = payment_breakdowns?;
    let shift_breakdowns = shift_breakdowns?;

    Ok(Some(DailyReportDetail {
        id: report.id,
        business_date: report.business_date,
        total_orders: report.total_orders,
        completed_orders: report.completed_orders,
        void_orders: report.void_orders,
        total_sales: report.total_sales,
        total_paid: report.total_paid,
        total_unpaid: report.total_unpaid,
        void_amount: report.void_amount,
        total_tax: report.total_tax,
        total_discount: report.total_discount,
        total_surcharge: report.total_surcharge,
        generated_at: report.generated_at,
        generated_by_id: report.generated_by_id,
        generated_by_name: report.generated_by_name,
        note: report.note,
        tax_breakdowns,
        payment_breakdowns,
        shift_breakdowns,
    }))
}

/// Get order detail from store_archived_orders.detail JSONB column
pub async fn get_order_detail(
    pool: &PgPool,
    store_id: i64,
    tenant_id: &str,
    order_key: &str,
) -> Result<Option<serde_json::Value>, BoxError> {
    let row: Option<(serde_json::Value,)> = sqlx::query_as(
        r#"
        SELECT detail
        FROM store_archived_orders
        WHERE store_id = $1 AND tenant_id = $2 AND order_key = $3
            AND detail IS NOT NULL
        "#,
    )
    .bind(store_id)
    .bind(tenant_id)
    .bind(order_key)
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|r| r.0))
}

/// Get order desglose from store_archived_orders.desglose JSONB column
pub async fn get_order_desglose(
    pool: &PgPool,
    store_id: i64,
    tenant_id: &str,
    order_key: &str,
) -> Result<Vec<shared::cloud::TaxDesglose>, BoxError> {
    let row: Option<(serde_json::Value,)> = sqlx::query_as(
        r#"
        SELECT desglose
        FROM store_archived_orders
        WHERE store_id = $1 AND tenant_id = $2 AND order_key = $3
        "#,
    )
    .bind(store_id)
    .bind(tenant_id)
    .bind(order_key)
    .fetch_optional(pool)
    .await?;

    match row {
        Some((json,)) => Ok(serde_json::from_value(json)?),
        None => Ok(vec![]),
    }
}

// ── Store overview statistics ──

/// Overview statistics for a time range
#[derive(Debug, serde::Serialize)]
pub struct StoreOverview {
    pub revenue: f64,
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
    pub revenue_trend: Vec<RevenueTrendPoint>,
    pub tax_breakdown: Vec<TaxBreakdownEntry>,
    pub payment_breakdown: Vec<PaymentBreakdownEntry>,
    pub top_products: Vec<TopProductEntry>,
    pub category_sales: Vec<CategorySaleEntry>,
    pub tag_sales: Vec<TagSaleEntry>,
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

/// Compute store overview for a single store
pub async fn get_store_overview(
    pool: &PgPool,
    store_id: i64,
    tenant_id: &str,
    from: i64,
    to: i64,
) -> Result<StoreOverview, BoxError> {
    get_overview(pool, tenant_id, Some(store_id), from, to).await
}

/// Compute tenant-wide overview (all stores combined)
pub async fn get_tenant_overview(
    pool: &PgPool,
    tenant_id: &str,
    from: i64,
    to: i64,
) -> Result<StoreOverview, BoxError> {
    get_overview(pool, tenant_id, None, from, to).await
}

/// Parameterized overview: store_id=None → tenant-wide, Some → single store.
/// All queries enforce tenant_id isolation.
async fn get_overview(
    pool: &PgPool,
    tenant_id: &str,
    store_id: Option<i64>,
    from: i64,
    to: i64,
) -> Result<StoreOverview, BoxError> {
    // 1. Basic aggregation from store_archived_orders
    #[allow(clippy::type_complexity)]
    let overview: (f64, i64, i64, f64, f64, i64, f64, f64, i64, f64, f64) = sqlx::query_as(
        r#"
        SELECT
            COALESCE(SUM(CASE WHEN status = 'COMPLETED' THEN total ELSE 0 END), 0)::DOUBLE PRECISION,
            COUNT(*) FILTER (WHERE status = 'COMPLETED'),
            COUNT(*) FILTER (WHERE status = 'VOID' AND (void_type IS NULL OR void_type != 'LOSS_SETTLED')),
            COALESCE(SUM(CASE WHEN status = 'VOID' THEN COALESCE(total, 0) ELSE 0 END), 0)::DOUBLE PRECISION,
            COALESCE(SUM(CASE WHEN status = 'COMPLETED' THEN COALESCE(tax, 0) ELSE 0 END), 0)::DOUBLE PRECISION,
            COALESCE(SUM(CASE WHEN status = 'COMPLETED' THEN COALESCE(guest_count, 0) ELSE 0 END), 0)::BIGINT,
            COALESCE(SUM(CASE WHEN status = 'COMPLETED' THEN COALESCE(discount_amount, 0) ELSE 0 END), 0)::DOUBLE PRECISION,
            COALESCE(AVG(CASE WHEN status = 'COMPLETED' AND start_time IS NOT NULL AND end_time IS NOT NULL
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
    let (
        revenue_trend_r,
        tax_breakdown_r,
        payment_breakdown_r,
        top_products_r,
        category_sales_r,
        tag_sales_r,
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
            GROUP BY hour
            ORDER BY hour
            "#,
        )
        .bind(tenant_id)
        .bind(store_id)
        .bind(from)
        .bind(to)
        .fetch_all(pool),
        // 3. Tax breakdown from desglose JSONB
        sqlx::query_as::<_, TaxBreakdownEntry>(
            r#"
            SELECT
                (d->>'tax_rate')::INTEGER AS tax_rate,
                COALESCE(SUM((d->>'base_amount')::DOUBLE PRECISION), 0) AS base_amount,
                COALESCE(SUM((d->>'tax_amount')::DOUBLE PRECISION), 0) AS tax_amount
            FROM store_archived_orders,
                 jsonb_array_elements(desglose) AS d
            WHERE tenant_id = $1
                AND ($2::BIGINT IS NULL OR store_id = $2)
                AND end_time >= $3 AND end_time < $4
                AND status = 'COMPLETED'
                AND desglose IS NOT NULL AND jsonb_typeof(desglose) = 'array'
            GROUP BY tax_rate
            ORDER BY tax_rate
            "#,
        )
        .bind(tenant_id)
        .bind(store_id)
        .bind(from)
        .bind(to)
        .fetch_all(pool),
        // 4. Payment breakdown from detail JSONB
        sqlx::query_as::<_, PaymentBreakdownEntry>(
            r#"
            SELECT
                p->>'method' AS method,
                COALESCE(SUM((p->>'amount')::DOUBLE PRECISION), 0) AS amount,
                COUNT(*) AS count
            FROM store_archived_orders o
            CROSS JOIN jsonb_array_elements(o.detail->'payments') AS p
            WHERE o.tenant_id = $1
                AND ($2::BIGINT IS NULL OR o.store_id = $2)
                AND o.end_time >= $3 AND o.end_time < $4
                AND o.status = 'COMPLETED'
                AND o.detail IS NOT NULL
                AND (p->>'cancelled')::BOOLEAN IS NOT TRUE
            GROUP BY method
            ORDER BY amount DESC
            "#,
        )
        .bind(tenant_id)
        .bind(store_id)
        .bind(from)
        .bind(to)
        .fetch_all(pool),
        // 5. Top products from detail JSONB
        sqlx::query_as::<_, TopProductEntry>(
            r#"
            SELECT
                i->>'name' AS name,
                COALESCE(SUM((i->>'quantity')::BIGINT), 0) AS quantity,
                COALESCE(SUM((i->>'line_total')::DOUBLE PRECISION), 0) AS revenue
            FROM store_archived_orders o
            CROSS JOIN jsonb_array_elements(o.detail->'items') AS i
            WHERE o.tenant_id = $1
                AND ($2::BIGINT IS NULL OR o.store_id = $2)
                AND o.end_time >= $3 AND o.end_time < $4
                AND o.status = 'COMPLETED'
                AND o.detail IS NOT NULL
            GROUP BY name
            ORDER BY quantity DESC
            LIMIT 10
            "#,
        )
        .bind(tenant_id)
        .bind(store_id)
        .bind(from)
        .bind(to)
        .fetch_all(pool),
        // 6. Category sales from detail JSONB
        sqlx::query_as::<_, CategorySaleEntry>(
            r#"
            SELECT
                COALESCE(i->>'category_name', 'Sin categoría') AS name,
                COALESCE(SUM((i->>'line_total')::DOUBLE PRECISION), 0) AS revenue
            FROM store_archived_orders o
            CROSS JOIN jsonb_array_elements(o.detail->'items') AS i
            WHERE o.tenant_id = $1
                AND ($2::BIGINT IS NULL OR o.store_id = $2)
                AND o.end_time >= $3 AND o.end_time < $4
                AND o.status = 'COMPLETED'
                AND o.detail IS NOT NULL
            GROUP BY name
            ORDER BY revenue DESC
            LIMIT 10
            "#,
        )
        .bind(tenant_id)
        .bind(store_id)
        .bind(from)
        .bind(to)
        .fetch_all(pool),
        // 7. Tag sales — JSONB items → store_products → store_product_tag → store_tags
        sqlx::query_as::<_, TagSaleEntry>(
            r#"
            SELECT
                t.name,
                t.color,
                COALESCE(SUM((i->>'line_total')::DOUBLE PRECISION), 0) AS revenue,
                COALESCE(SUM((i->>'quantity')::BIGINT), 0) AS quantity
            FROM store_archived_orders o
            CROSS JOIN jsonb_array_elements(o.detail->'items') AS i
            JOIN store_products p ON p.source_id = (i->>'product_source_id')::BIGINT
                AND p.store_id = o.store_id
            JOIN store_product_tag pt ON pt.product_id = p.id
            JOIN store_tags t ON t.source_id = pt.tag_source_id
                AND t.store_id = o.store_id
            WHERE o.tenant_id = $1
                AND ($2::BIGINT IS NULL OR o.store_id = $2)
                AND o.end_time >= $3 AND o.end_time < $4
                AND o.status = 'COMPLETED'
                AND o.detail IS NOT NULL
                AND i->>'product_source_id' IS NOT NULL
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
    );

    let revenue_trend = revenue_trend_r?;
    let tax_breakdown = tax_breakdown_r.unwrap_or_default();
    let payment_breakdown = payment_breakdown_r.unwrap_or_default();
    let top_products = top_products_r.unwrap_or_default();
    let category_sales = category_sales_r.unwrap_or_default();
    let tag_sales = tag_sales_r.unwrap_or_default();

    Ok(StoreOverview {
        revenue,
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
        revenue_trend,
        tax_breakdown,
        payment_breakdown,
        top_products,
        category_sales,
        tag_sales,
    })
}

// ── Red Flags 监控 ──

#[derive(Debug, serde::Serialize)]
pub struct RedFlagsSummary {
    pub item_removals: i64,
    pub item_comps: i64,
    pub order_voids: i64,
    pub order_discounts: i64,
    pub price_modifications: i64,
}

#[derive(Debug, serde::Serialize)]
pub struct OperatorRedFlags {
    pub operator_id: Option<i64>,
    pub operator_name: Option<String>,
    pub item_removals: i64,
    pub item_comps: i64,
    pub order_voids: i64,
    pub order_discounts: i64,
    pub price_modifications: i64,
    pub total_flags: i64,
}

#[derive(Debug, serde::Serialize)]
pub struct RedFlagsResponse {
    pub summary: RedFlagsSummary,
    pub operator_breakdown: Vec<OperatorRedFlags>,
}

pub async fn get_red_flags(
    pool: &PgPool,
    store_id: i64,
    tenant_id: &str,
    from: i64,
    to: i64,
) -> Result<RedFlagsResponse, BoxError> {
    #[derive(sqlx::FromRow)]
    struct Row {
        operator_id: Option<i64>,
        operator_name: Option<String>,
        item_removals: i64,
        item_comps: i64,
        order_voids: i64,
        order_discounts: i64,
        price_modifications: i64,
    }

    let rows: Vec<Row> = sqlx::query_as(
        r#"
        SELECT
            (e->>'operator_id')::BIGINT AS operator_id,
            e->>'operator_name' AS operator_name,
            COUNT(*) FILTER (WHERE e->>'event_type' = 'ITEM_REMOVED') AS item_removals,
            COUNT(*) FILTER (WHERE e->>'event_type' = 'ITEM_COMPED') AS item_comps,
            COUNT(*) FILTER (WHERE e->>'event_type' = 'ORDER_VOIDED') AS order_voids,
            COUNT(*) FILTER (WHERE e->>'event_type' = 'ORDER_DISCOUNT_APPLIED') AS order_discounts,
            COUNT(*) FILTER (WHERE e->>'event_type' = 'ITEM_MODIFIED') AS price_modifications
        FROM store_archived_orders o
        CROSS JOIN jsonb_array_elements(o.detail->'events') AS e
        WHERE o.store_id = $1 AND o.tenant_id = $2
            AND o.detail IS NOT NULL
            AND (e->>'timestamp')::BIGINT >= $3 AND (e->>'timestamp')::BIGINT < $4
            AND e->>'event_type' IN ('ITEM_REMOVED','ITEM_COMPED','ORDER_VOIDED','ORDER_DISCOUNT_APPLIED','ITEM_MODIFIED')
        GROUP BY operator_id, operator_name
        ORDER BY COUNT(*) DESC
        "#,
    )
    .bind(store_id)
    .bind(tenant_id)
    .bind(from)
    .bind(to)
    .fetch_all(pool)
    .await?;

    let mut summary = RedFlagsSummary {
        item_removals: 0,
        item_comps: 0,
        order_voids: 0,
        order_discounts: 0,
        price_modifications: 0,
    };

    let mut operator_breakdown = Vec::new();
    for row in rows {
        summary.item_removals += row.item_removals;
        summary.item_comps += row.item_comps;
        summary.order_voids += row.order_voids;
        summary.order_discounts += row.order_discounts;
        summary.price_modifications += row.price_modifications;

        let total_flags = row.item_removals
            + row.item_comps
            + row.order_voids
            + row.order_discounts
            + row.price_modifications;
        operator_breakdown.push(OperatorRedFlags {
            operator_id: row.operator_id,
            operator_name: row.operator_name,
            item_removals: row.item_removals,
            item_comps: row.item_comps,
            order_voids: row.order_voids,
            order_discounts: row.order_discounts,
            price_modifications: row.price_modifications,
            total_flags,
        });
    }

    Ok(RedFlagsResponse {
        summary,
        operator_breakdown,
    })
}

/// Verify edge-server belongs to tenant, return store_id
pub async fn verify_store_ownership(
    pool: &PgPool,
    store_id: i64,
    tenant_id: &str,
) -> Result<Option<i64>, BoxError> {
    let row: Option<(i64,)> =
        sqlx::query_as("SELECT id FROM stores WHERE id = $1 AND tenant_id = $2")
            .bind(store_id)
            .bind(tenant_id)
            .fetch_optional(pool)
            .await?;
    Ok(row.map(|r| r.0))
}
