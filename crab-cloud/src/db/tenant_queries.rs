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
    pub created_at: i64,
}

pub async fn get_subscription(
    pool: &PgPool,
    tenant_id: &str,
) -> Result<Option<SubscriptionInfo>, BoxError> {
    let row: Option<SubscriptionInfo> = sqlx::query_as(
        "SELECT id, status, plan, max_edge_servers, max_clients, current_period_end, created_at FROM subscriptions WHERE tenant_id = $1 ORDER BY created_at DESC LIMIT 1",
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
    pub device_id: String,
    pub last_sync_at: Option<i64>,
    pub registered_at: i64,
}

pub async fn list_stores(pool: &PgPool, tenant_id: &str) -> Result<Vec<StoreSummary>, BoxError> {
    let rows: Vec<StoreSummary> = sqlx::query_as(
        r#"
        SELECT id, entity_id, device_id, last_sync_at, registered_at
        FROM cloud_edge_servers
        WHERE tenant_id = $1
        ORDER BY registered_at DESC
        "#,
    )
    .bind(tenant_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Get store info data for a specific edge-server
pub async fn get_store_info(
    pool: &PgPool,
    edge_server_id: i64,
    tenant_id: &str,
) -> Result<Option<serde_json::Value>, BoxError> {
    let row: Option<(serde_json::Value,)> = sqlx::query_as(
        "SELECT data FROM cloud_store_info WHERE edge_server_id = $1 AND tenant_id = $2",
    )
    .bind(edge_server_id)
    .bind(tenant_id)
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|r| r.0))
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
            FROM cloud_archived_orders
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
            FROM cloud_archived_orders
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

/// Daily report stats with date range
#[derive(serde::Serialize, sqlx::FromRow)]
pub struct DailyReportEntry {
    pub id: i64,
    pub source_id: String,
    pub data: serde_json::Value,
    pub synced_at: i64,
}

pub async fn list_daily_reports(
    pool: &PgPool,
    edge_server_id: i64,
    tenant_id: &str,
    from: Option<i64>,
    to: Option<i64>,
) -> Result<Vec<DailyReportEntry>, BoxError> {
    let rows: Vec<DailyReportEntry> = sqlx::query_as(
        r#"
        SELECT id, source_id, data, synced_at
        FROM cloud_daily_reports
        WHERE edge_server_id = $1 AND tenant_id = $2
            AND ($3::BIGINT IS NULL OR synced_at >= $3)
            AND ($4::BIGINT IS NULL OR synced_at <= $4)
        ORDER BY synced_at DESC
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

/// Product list for a store
#[derive(serde::Serialize, sqlx::FromRow)]
pub struct ProductEntry {
    pub id: i64,
    pub source_id: String,
    pub data: serde_json::Value,
    pub synced_at: i64,
}

pub async fn list_products(
    pool: &PgPool,
    edge_server_id: i64,
    tenant_id: &str,
) -> Result<Vec<ProductEntry>, BoxError> {
    let rows: Vec<ProductEntry> = sqlx::query_as(
        r#"
        SELECT p.id, p.source_id,
            CASE WHEN c.data IS NOT NULL
                THEN jsonb_set(p.data, '{category_name}', to_jsonb(c.data->>'name'))
                ELSE p.data
            END AS data,
            p.synced_at
        FROM cloud_products p
        LEFT JOIN cloud_categories c
            ON c.edge_server_id = p.edge_server_id
            AND c.source_id = (p.data->>'category_id')
        WHERE p.edge_server_id = $1 AND p.tenant_id = $2
        ORDER BY c.data->>'sort_order', p.data->>'sort_order', p.source_id
        "#,
    )
    .bind(edge_server_id)
    .bind(tenant_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Get cached order detail (from cloud_order_details, 30-day cache)
pub async fn get_order_detail(
    pool: &PgPool,
    edge_server_id: i64,
    tenant_id: &str,
    order_key: &str,
) -> Result<Option<serde_json::Value>, BoxError> {
    let row: Option<(serde_json::Value,)> = sqlx::query_as(
        r#"
        SELECT d.detail
        FROM cloud_order_details d
        JOIN cloud_archived_orders o ON o.id = d.archived_order_id
        WHERE o.edge_server_id = $1 AND o.tenant_id = $2 AND o.order_key = $3
        "#,
    )
    .bind(edge_server_id)
    .bind(tenant_id)
    .bind(order_key)
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|r| r.0))
}

/// Get order desglose from cloud_archived_orders.desglose JSONB column
pub async fn get_order_desglose(
    pool: &PgPool,
    edge_server_id: i64,
    tenant_id: &str,
    order_key: &str,
) -> Result<Vec<shared::cloud::TaxDesglose>, BoxError> {
    let row: Option<(serde_json::Value,)> = sqlx::query_as(
        r#"
        SELECT desglose
        FROM cloud_archived_orders
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

// ── Store overview statistics (computed from cloud_archived_orders) ──

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
}

#[derive(Debug, serde::Serialize, sqlx::FromRow)]
pub struct RevenueTrendPoint {
    pub hour: i32,
    pub revenue: f64,
    pub orders: i64,
}

#[derive(Debug, serde::Serialize)]
pub struct TaxBreakdownEntry {
    pub tax_rate: i32,
    pub base_amount: f64,
    pub tax_amount: f64,
}

#[derive(Debug, serde::Serialize)]
pub struct PaymentBreakdownEntry {
    pub method: String,
    pub amount: f64,
    pub count: i64,
}

#[derive(Debug, serde::Serialize)]
pub struct TopProductEntry {
    pub name: String,
    pub quantity: i64,
    pub revenue: f64,
}

#[derive(Debug, serde::Serialize)]
pub struct CategorySaleEntry {
    pub name: String,
    pub revenue: f64,
}

/// Compute store overview statistics for a time range (from..to as unix millis)
pub async fn get_store_overview(
    pool: &PgPool,
    edge_server_id: i64,
    tenant_id: &str,
    from: i64,
    to: i64,
) -> Result<StoreOverview, BoxError> {
    // 1. Basic aggregation from cloud_archived_orders
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
        FROM cloud_archived_orders
        WHERE edge_server_id = $1 AND tenant_id = $2
            AND end_time >= $3 AND end_time < $4
        "#,
    )
    .bind(edge_server_id)
    .bind(tenant_id)
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
        FROM cloud_archived_orders
        WHERE edge_server_id = $1 AND tenant_id = $2
            AND end_time >= $3 AND end_time < $4
            AND status = 'COMPLETED'
        GROUP BY hour
        ORDER BY hour
        "#,
    )
    .bind(edge_server_id)
    .bind(tenant_id)
    .bind(from)
    .bind(to)
    .fetch_all(pool)
    .await?;

    // 3. Tax breakdown from cloud_order_items (permanent, computed from line_total + tax_rate)
    let tax_rows: Vec<(i32, f64)> = sqlx::query_as(
        r#"
        SELECT
            i.tax_rate,
            COALESCE(SUM(i.line_total), 0)::DOUBLE PRECISION AS base_amount
        FROM cloud_order_items i
        JOIN cloud_archived_orders o ON o.id = i.archived_order_id
        WHERE o.edge_server_id = $1 AND o.tenant_id = $2
            AND o.end_time >= $3 AND o.end_time < $4
            AND o.status = 'COMPLETED'
        GROUP BY i.tax_rate
        ORDER BY i.tax_rate
        "#,
    )
    .bind(edge_server_id)
    .bind(tenant_id)
    .bind(from)
    .bind(to)
    .fetch_all(pool)
    .await?;

    let tax_breakdown: Vec<TaxBreakdownEntry> = tax_rows
        .into_iter()
        .map(|(tax_rate, base_amount)| {
            let tax_amount = base_amount * tax_rate as f64 / 100.0;
            TaxBreakdownEntry {
                tax_rate,
                base_amount,
                tax_amount,
            }
        })
        .collect();

    // 4. Payment breakdown from cloud_order_payments (permanent)
    let payment_rows: Vec<(String, f64, i64)> = sqlx::query_as(
        r#"
        SELECT
            p.method,
            COALESCE(SUM(p.amount), 0)::DOUBLE PRECISION AS amount,
            COUNT(*) AS count
        FROM cloud_order_payments p
        JOIN cloud_archived_orders o ON o.id = p.archived_order_id
        WHERE o.edge_server_id = $1 AND o.tenant_id = $2
            AND o.end_time >= $3 AND o.end_time < $4
            AND o.status = 'COMPLETED'
        GROUP BY p.method
        ORDER BY amount DESC
        "#,
    )
    .bind(edge_server_id)
    .bind(tenant_id)
    .bind(from)
    .bind(to)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    let payment_breakdown: Vec<PaymentBreakdownEntry> = payment_rows
        .into_iter()
        .map(|(method, amount, count)| PaymentBreakdownEntry {
            method,
            amount,
            count,
        })
        .collect();

    // 5. Top products from cloud_order_items (permanent)
    let product_rows: Vec<(String, i64, f64)> = sqlx::query_as(
        r#"
        SELECT
            i.name,
            COALESCE(SUM(i.quantity), 0)::BIGINT AS quantity,
            COALESCE(SUM(i.line_total), 0)::DOUBLE PRECISION AS revenue
        FROM cloud_order_items i
        JOIN cloud_archived_orders o ON o.id = i.archived_order_id
        WHERE o.edge_server_id = $1 AND o.tenant_id = $2
            AND o.end_time >= $3 AND o.end_time < $4
            AND o.status = 'COMPLETED'
        GROUP BY i.name
        ORDER BY quantity DESC
        LIMIT 10
        "#,
    )
    .bind(edge_server_id)
    .bind(tenant_id)
    .bind(from)
    .bind(to)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    let top_products: Vec<TopProductEntry> = product_rows
        .into_iter()
        .map(|(name, quantity, revenue)| TopProductEntry {
            name,
            quantity,
            revenue,
        })
        .collect();

    // 6. Category sales from cloud_order_items (permanent)
    let category_rows: Vec<(String, f64)> = sqlx::query_as(
        r#"
        SELECT
            COALESCE(i.category_name, 'Sin categoría') AS name,
            COALESCE(SUM(i.line_total), 0)::DOUBLE PRECISION AS revenue
        FROM cloud_order_items i
        JOIN cloud_archived_orders o ON o.id = i.archived_order_id
        WHERE o.edge_server_id = $1 AND o.tenant_id = $2
            AND o.end_time >= $3 AND o.end_time < $4
            AND o.status = 'COMPLETED'
        GROUP BY name
        ORDER BY revenue DESC
        LIMIT 10
        "#,
    )
    .bind(edge_server_id)
    .bind(tenant_id)
    .bind(from)
    .bind(to)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    let category_sales: Vec<CategorySaleEntry> = category_rows
        .into_iter()
        .map(|(name, revenue)| CategorySaleEntry { name, revenue })
        .collect();

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
    })
}

/// Compute tenant-wide overview statistics (all stores combined) for a time range
pub async fn get_tenant_overview(
    pool: &PgPool,
    tenant_id: &str,
    from: i64,
    to: i64,
) -> Result<StoreOverview, BoxError> {
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
        FROM cloud_archived_orders
        WHERE tenant_id = $1
            AND end_time >= $2 AND end_time < $3
        "#,
    )
    .bind(tenant_id)
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

    let revenue_trend: Vec<RevenueTrendPoint> = sqlx::query_as(
        r#"
        SELECT
            EXTRACT(HOUR FROM TO_TIMESTAMP(end_time / 1000.0))::INTEGER AS hour,
            COALESCE(SUM(total), 0)::DOUBLE PRECISION AS revenue,
            COUNT(*) AS orders
        FROM cloud_archived_orders
        WHERE tenant_id = $1
            AND end_time >= $2 AND end_time < $3
            AND status = 'COMPLETED'
        GROUP BY hour
        ORDER BY hour
        "#,
    )
    .bind(tenant_id)
    .bind(from)
    .bind(to)
    .fetch_all(pool)
    .await?;

    let tax_rows: Vec<(i32, f64)> = sqlx::query_as(
        r#"
        SELECT
            i.tax_rate,
            COALESCE(SUM(i.line_total), 0)::DOUBLE PRECISION AS base_amount
        FROM cloud_order_items i
        JOIN cloud_archived_orders o ON o.id = i.archived_order_id
        WHERE o.tenant_id = $1
            AND o.end_time >= $2 AND o.end_time < $3
            AND o.status = 'COMPLETED'
        GROUP BY i.tax_rate
        ORDER BY i.tax_rate
        "#,
    )
    .bind(tenant_id)
    .bind(from)
    .bind(to)
    .fetch_all(pool)
    .await?;

    let tax_breakdown: Vec<TaxBreakdownEntry> = tax_rows
        .into_iter()
        .map(|(tax_rate, base_amount)| {
            let tax_amount = base_amount * tax_rate as f64 / 100.0;
            TaxBreakdownEntry {
                tax_rate,
                base_amount,
                tax_amount,
            }
        })
        .collect();

    let payment_rows: Vec<(String, f64, i64)> = sqlx::query_as(
        r#"
        SELECT
            p.method,
            COALESCE(SUM(p.amount), 0)::DOUBLE PRECISION AS amount,
            COUNT(*) AS count
        FROM cloud_order_payments p
        JOIN cloud_archived_orders o ON o.id = p.archived_order_id
        WHERE o.tenant_id = $1
            AND o.end_time >= $2 AND o.end_time < $3
            AND o.status = 'COMPLETED'
        GROUP BY p.method
        ORDER BY amount DESC
        "#,
    )
    .bind(tenant_id)
    .bind(from)
    .bind(to)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    let payment_breakdown: Vec<PaymentBreakdownEntry> = payment_rows
        .into_iter()
        .map(|(method, amount, count)| PaymentBreakdownEntry {
            method,
            amount,
            count,
        })
        .collect();

    let product_rows: Vec<(String, i64, f64)> = sqlx::query_as(
        r#"
        SELECT
            i.name,
            COALESCE(SUM(i.quantity), 0)::BIGINT AS quantity,
            COALESCE(SUM(i.line_total), 0)::DOUBLE PRECISION AS revenue
        FROM cloud_order_items i
        JOIN cloud_archived_orders o ON o.id = i.archived_order_id
        WHERE o.tenant_id = $1
            AND o.end_time >= $2 AND o.end_time < $3
            AND o.status = 'COMPLETED'
        GROUP BY i.name
        ORDER BY quantity DESC
        LIMIT 10
        "#,
    )
    .bind(tenant_id)
    .bind(from)
    .bind(to)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    let top_products: Vec<TopProductEntry> = product_rows
        .into_iter()
        .map(|(name, quantity, revenue)| TopProductEntry {
            name,
            quantity,
            revenue,
        })
        .collect();

    let category_rows: Vec<(String, f64)> = sqlx::query_as(
        r#"
        SELECT
            COALESCE(i.category_name, 'Sin categoría') AS name,
            COALESCE(SUM(i.line_total), 0)::DOUBLE PRECISION AS revenue
        FROM cloud_order_items i
        JOIN cloud_archived_orders o ON o.id = i.archived_order_id
        WHERE o.tenant_id = $1
            AND o.end_time >= $2 AND o.end_time < $3
            AND o.status = 'COMPLETED'
        GROUP BY name
        ORDER BY revenue DESC
        LIMIT 10
        "#,
    )
    .bind(tenant_id)
    .bind(from)
    .bind(to)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    let category_sales: Vec<CategorySaleEntry> = category_rows
        .into_iter()
        .map(|(name, revenue)| CategorySaleEntry { name, revenue })
        .collect();

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
    })
}

/// Verify edge-server belongs to tenant, return edge_server_id
pub async fn verify_store_ownership(
    pool: &PgPool,
    store_id: i64,
    tenant_id: &str,
) -> Result<Option<i64>, BoxError> {
    let row: Option<(i64,)> =
        sqlx::query_as("SELECT id FROM cloud_edge_servers WHERE id = $1 AND tenant_id = $2")
            .bind(store_id)
            .bind(tenant_id)
            .fetch_optional(pool)
            .await?;
    Ok(row.map(|r| r.0))
}
