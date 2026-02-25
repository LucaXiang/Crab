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
        SELECT id, entity_id, name, address, phone, nif, email, website, business_day_cutoff, device_id, last_sync_at, registered_at
        FROM edge_servers
        WHERE tenant_id = $1
        ORDER BY registered_at DESC
        "#,
    )
    .bind(tenant_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Update store
#[allow(clippy::too_many_arguments)]
pub async fn update_store(
    pool: &PgPool,
    store_id: i64,
    tenant_id: &str,
    name: Option<String>,
    address: Option<String>,
    phone: Option<String>,
    nif: Option<String>,
    email: Option<String>,
    website: Option<String>,
    business_day_cutoff: Option<String>,
) -> Result<(), BoxError> {
    sqlx::query(
        r#"
        UPDATE edge_servers 
        SET 
            name = COALESCE($1, name),
            address = COALESCE($2, address),
            phone = COALESCE($3, phone),
            nif = COALESCE($4, nif),
            email = COALESCE($5, email),
            website = COALESCE($6, website),
            business_day_cutoff = COALESCE($7, business_day_cutoff)
        WHERE id = $8 AND tenant_id = $9
        "#,
    )
    .bind(name)
    .bind(address)
    .bind(phone)
    .bind(nif)
    .bind(email)
    .bind(website)
    .bind(business_day_cutoff)
    .bind(store_id)
    .bind(tenant_id)
    .execute(pool)
    .await?;
    Ok(())
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
    edge_server_id: i64,
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
            WHERE edge_server_id = $1 AND tenant_id = $2 AND status = $3
            ORDER BY end_time DESC NULLS LAST
            LIMIT $4 OFFSET $5
            "#,
        )
        .bind(edge_server_id)
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
            WHERE edge_server_id = $1 AND tenant_id = $2
            ORDER BY end_time DESC NULLS LAST
            LIMIT $3 OFFSET $4
            "#,
        )
        .bind(edge_server_id)
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
    pub total_orders: i32,
    pub completed_orders: i32,
    pub void_orders: i32,
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
    edge_server_id: i64,
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
        JOIN edge_servers es ON es.id = dr.edge_server_id
        WHERE dr.edge_server_id = $1
            AND es.tenant_id = $2
            AND ($3::TEXT IS NULL OR dr.business_date >= $3)
            AND ($4::TEXT IS NULL OR dr.business_date <= $4)
        ORDER BY dr.business_date DESC
        "#,
    )
    .bind(edge_server_id)
    .bind(tenant_id)
    .bind(from)
    .bind(to)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Get order detail from store_archived_orders.detail JSONB column
pub async fn get_order_detail(
    pool: &PgPool,
    edge_server_id: i64,
    tenant_id: &str,
    order_key: &str,
) -> Result<Option<serde_json::Value>, BoxError> {
    let row: Option<(serde_json::Value,)> = sqlx::query_as(
        r#"
        SELECT detail
        FROM store_archived_orders
        WHERE edge_server_id = $1 AND tenant_id = $2 AND order_key = $3
            AND detail IS NOT NULL
        "#,
    )
    .bind(edge_server_id)
    .bind(tenant_id)
    .bind(order_key)
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|r| r.0))
}

/// Get order desglose from store_archived_orders.desglose JSONB column
pub async fn get_order_desglose(
    pool: &PgPool,
    edge_server_id: i64,
    tenant_id: &str,
    order_key: &str,
) -> Result<Vec<shared::cloud::TaxDesglose>, BoxError> {
    let row: Option<(serde_json::Value,)> = sqlx::query_as(
        r#"
        SELECT desglose
        FROM store_archived_orders
        WHERE edge_server_id = $1 AND tenant_id = $2 AND order_key = $3
        "#,
    )
    .bind(edge_server_id)
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
    edge_server_id: i64,
    tenant_id: &str,
    from: i64,
    to: i64,
) -> Result<StoreOverview, BoxError> {
    get_overview(pool, tenant_id, Some(edge_server_id), from, to).await
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

/// Parameterized overview: edge_server_id=None → tenant-wide, Some → single store.
/// All queries enforce tenant_id isolation.
async fn get_overview(
    pool: &PgPool,
    tenant_id: &str,
    edge_server_id: Option<i64>,
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
            AND ($2::BIGINT IS NULL OR edge_server_id = $2)
            AND end_time >= $3 AND end_time < $4
        "#,
    )
    .bind(tenant_id)
    .bind(edge_server_id)
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

    // 2. Revenue trend (by hour of day)
    let revenue_trend: Vec<RevenueTrendPoint> = sqlx::query_as(
        r#"
        SELECT
            EXTRACT(HOUR FROM TO_TIMESTAMP(end_time / 1000.0))::INTEGER AS hour,
            COALESCE(SUM(total), 0)::DOUBLE PRECISION AS revenue,
            COUNT(*) AS orders
        FROM store_archived_orders
        WHERE tenant_id = $1
            AND ($2::BIGINT IS NULL OR edge_server_id = $2)
            AND end_time >= $3 AND end_time < $4
            AND status = 'COMPLETED'
        GROUP BY hour
        ORDER BY hour
        "#,
    )
    .bind(tenant_id)
    .bind(edge_server_id)
    .bind(from)
    .bind(to)
    .fetch_all(pool)
    .await?;

    // 3. Tax breakdown from desglose JSONB (already aggregated per order)
    let tax_breakdown: Vec<TaxBreakdownEntry> = sqlx::query_as(
        r#"
        SELECT
            (d->>'tax_rate')::INTEGER AS tax_rate,
            COALESCE(SUM((d->>'base_amount')::DOUBLE PRECISION), 0) AS base_amount,
            COALESCE(SUM((d->>'tax_amount')::DOUBLE PRECISION), 0) AS tax_amount
        FROM store_archived_orders,
             jsonb_array_elements(desglose) AS d
        WHERE tenant_id = $1
            AND ($2::BIGINT IS NULL OR edge_server_id = $2)
            AND end_time >= $3 AND end_time < $4
            AND status = 'COMPLETED'
            AND desglose IS NOT NULL AND jsonb_typeof(desglose) = 'array'
        GROUP BY tax_rate
        ORDER BY tax_rate
        "#,
    )
    .bind(tenant_id)
    .bind(edge_server_id)
    .bind(from)
    .bind(to)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    // 4. Payment breakdown from detail JSONB
    let payment_breakdown: Vec<PaymentBreakdownEntry> = sqlx::query_as(
        r#"
        SELECT
            p->>'method' AS method,
            COALESCE(SUM((p->>'amount')::DOUBLE PRECISION), 0) AS amount,
            COUNT(*) AS count
        FROM store_archived_orders o
        CROSS JOIN jsonb_array_elements(o.detail->'payments') AS p
        WHERE o.tenant_id = $1
            AND ($2::BIGINT IS NULL OR o.edge_server_id = $2)
            AND o.end_time >= $3 AND o.end_time < $4
            AND o.status = 'COMPLETED'
            AND o.detail IS NOT NULL
            AND (p->>'cancelled')::BOOLEAN IS NOT TRUE
        GROUP BY method
        ORDER BY amount DESC
        "#,
    )
    .bind(tenant_id)
    .bind(edge_server_id)
    .bind(from)
    .bind(to)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    // 5. Top products from detail JSONB
    let top_products: Vec<TopProductEntry> = sqlx::query_as(
        r#"
        SELECT
            i->>'name' AS name,
            COALESCE(SUM((i->>'quantity')::BIGINT), 0) AS quantity,
            COALESCE(SUM((i->>'line_total')::DOUBLE PRECISION), 0) AS revenue
        FROM store_archived_orders o
        CROSS JOIN jsonb_array_elements(o.detail->'items') AS i
        WHERE o.tenant_id = $1
            AND ($2::BIGINT IS NULL OR o.edge_server_id = $2)
            AND o.end_time >= $3 AND o.end_time < $4
            AND o.status = 'COMPLETED'
            AND o.detail IS NOT NULL
        GROUP BY name
        ORDER BY quantity DESC
        LIMIT 10
        "#,
    )
    .bind(tenant_id)
    .bind(edge_server_id)
    .bind(from)
    .bind(to)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    // 6. Category sales from detail JSONB
    let category_sales: Vec<CategorySaleEntry> = sqlx::query_as(
        r#"
        SELECT
            COALESCE(i->>'category_name', 'Sin categoría') AS name,
            COALESCE(SUM((i->>'line_total')::DOUBLE PRECISION), 0) AS revenue
        FROM store_archived_orders o
        CROSS JOIN jsonb_array_elements(o.detail->'items') AS i
        WHERE o.tenant_id = $1
            AND ($2::BIGINT IS NULL OR o.edge_server_id = $2)
            AND o.end_time >= $3 AND o.end_time < $4
            AND o.status = 'COMPLETED'
            AND o.detail IS NOT NULL
        GROUP BY name
        ORDER BY revenue DESC
        LIMIT 10
        "#,
    )
    .bind(tenant_id)
    .bind(edge_server_id)
    .bind(from)
    .bind(to)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    // 7. Tag sales — JSONB items → store_product_tag → store_tags
    let tag_sales: Vec<TagSaleEntry> = sqlx::query_as(
        r#"
        SELECT
            t.name,
            t.color,
            COALESCE(SUM((i->>'line_total')::DOUBLE PRECISION), 0) AS revenue,
            COALESCE(SUM((i->>'quantity')::BIGINT), 0) AS quantity
        FROM store_archived_orders o
        CROSS JOIN jsonb_array_elements(o.detail->'items') AS i
        JOIN store_product_tag pt ON pt.product_source_id = (i->>'product_source_id')::BIGINT
            AND pt.store_id = o.edge_server_id
        JOIN store_tags t ON t.source_id = pt.tag_source_id
            AND t.store_id = o.edge_server_id
        WHERE o.tenant_id = $1
            AND ($2::BIGINT IS NULL OR o.edge_server_id = $2)
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
    .bind(edge_server_id)
    .bind(from)
    .bind(to)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

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
    edge_server_id: i64,
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
        WHERE o.edge_server_id = $1 AND o.tenant_id = $2
            AND o.detail IS NOT NULL
            AND (e->>'timestamp')::BIGINT >= $3 AND (e->>'timestamp')::BIGINT < $4
            AND e->>'event_type' IN ('ITEM_REMOVED','ITEM_COMPED','ORDER_VOIDED','ORDER_DISCOUNT_APPLIED','ITEM_MODIFIED')
        GROUP BY operator_id, operator_name
        ORDER BY COUNT(*) DESC
        "#,
    )
    .bind(edge_server_id)
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

/// Verify edge-server belongs to tenant, return edge_server_id
pub async fn verify_store_ownership(
    pool: &PgPool,
    store_id: i64,
    tenant_id: &str,
) -> Result<Option<i64>, BoxError> {
    let row: Option<(i64,)> =
        sqlx::query_as("SELECT id FROM edge_servers WHERE id = $1 AND tenant_id = $2")
            .bind(store_id)
            .bind(tenant_id)
            .fetch_optional(pool)
            .await?;
    Ok(row.map(|r| r.0))
}
