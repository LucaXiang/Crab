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
        "SELECT id, status, plan, max_edge_servers, max_clients, current_period_end, created_at FROM subscriptions WHERE tenant_id = $1 AND status = 'active'",
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
        SELECT id, source_id, data, synced_at
        FROM cloud_products
        WHERE edge_server_id = $1 AND tenant_id = $2
        ORDER BY source_id
        "#,
    )
    .bind(edge_server_id)
    .bind(tenant_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
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
