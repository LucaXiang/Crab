//! Sync data storage operations
//!
//! Upsert operations for each resource type synced from edge-servers.
//! All data is stored as JSONB mirrors — crab-cloud is a data mirror,
//! not a business engine.

use shared::cloud::CloudSyncItem;
use sqlx::PgPool;

type BoxError = Box<dyn std::error::Error + Send + Sync>;

/// Ensure edge-server is registered, returning its database ID
pub async fn ensure_edge_server(
    pool: &PgPool,
    entity_id: &str,
    tenant_id: &str,
    device_id: &str,
    now: i64,
) -> Result<i64, BoxError> {
    let row: (i64,) = sqlx::query_as(
        r#"
        INSERT INTO cloud_edge_servers (entity_id, tenant_id, device_id, registered_at)
        VALUES ($1, $2, $3, $4)
        ON CONFLICT (entity_id, tenant_id) DO UPDATE SET device_id = EXCLUDED.device_id
        RETURNING id
        "#,
    )
    .bind(entity_id)
    .bind(tenant_id)
    .bind(device_id)
    .bind(now)
    .fetch_one(pool)
    .await?;

    Ok(row.0)
}

/// Update last_sync_at for an edge-server
pub async fn update_last_sync(
    pool: &PgPool,
    edge_server_id: i64,
    now: i64,
) -> Result<(), BoxError> {
    sqlx::query("UPDATE cloud_edge_servers SET last_sync_at = $1 WHERE id = $2")
        .bind(now)
        .bind(edge_server_id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Update sync cursor for a resource
pub async fn update_cursor(
    pool: &PgPool,
    edge_server_id: i64,
    resource: &str,
    version: i64,
    now: i64,
) -> Result<(), BoxError> {
    sqlx::query(
        r#"
        INSERT INTO cloud_sync_cursors (edge_server_id, resource, last_version, updated_at)
        VALUES ($1, $2, $3, $4)
        ON CONFLICT (edge_server_id, resource)
        DO UPDATE SET last_version = GREATEST(cloud_sync_cursors.last_version, EXCLUDED.last_version),
                      updated_at = EXCLUDED.updated_at
        "#,
    )
    .bind(edge_server_id)
    .bind(resource)
    .bind(version)
    .bind(now)
    .execute(pool)
    .await?;
    Ok(())
}

/// Upsert a resource based on its type
pub async fn upsert_resource(
    pool: &PgPool,
    edge_server_id: i64,
    tenant_id: &str,
    item: &CloudSyncItem,
    now: i64,
) -> Result<(), BoxError> {
    if item.action == "delete" {
        return delete_resource(pool, edge_server_id, &item.resource, &item.resource_id).await;
    }

    match item.resource.as_str() {
        "product" => {
            upsert_generic(pool, "cloud_products", edge_server_id, tenant_id, item, now).await
        }
        "category" => {
            upsert_generic(
                pool,
                "cloud_categories",
                edge_server_id,
                tenant_id,
                item,
                now,
            )
            .await
        }
        "archived_order" => upsert_archived_order(pool, edge_server_id, tenant_id, item, now).await,
        "daily_report" => {
            upsert_generic(
                pool,
                "cloud_daily_reports",
                edge_server_id,
                tenant_id,
                item,
                now,
            )
            .await
        }
        "store_info" => upsert_store_info(pool, edge_server_id, tenant_id, item, now).await,
        other => Err(format!("Unknown resource type: {other}").into()),
    }
}

/// Generic upsert for simple mirror tables (product, category, daily_report)
async fn upsert_generic(
    pool: &PgPool,
    table: &str,
    edge_server_id: i64,
    tenant_id: &str,
    item: &CloudSyncItem,
    now: i64,
) -> Result<(), BoxError> {
    let sql = format!(
        r#"
        INSERT INTO {table} (edge_server_id, tenant_id, source_id, data, version, synced_at)
        VALUES ($1, $2, $3, $4, $5, $6)
        ON CONFLICT (edge_server_id, source_id)
        DO UPDATE SET data = EXCLUDED.data, version = EXCLUDED.version, synced_at = EXCLUDED.synced_at
        WHERE {table}.version < EXCLUDED.version
        "#
    );

    sqlx::query(&sql)
        .bind(edge_server_id)
        .bind(tenant_id)
        .bind(&item.resource_id)
        .bind(&item.data)
        .bind(item.version as i64)
        .bind(now)
        .execute(pool)
        .await?;
    Ok(())
}

/// Upsert archived order with three-layer storage:
/// 1. cloud_archived_orders (永久摘要 + VeriFactu 字段)
/// 2. cloud_order_desglose (永久税率分拆)
/// 3. cloud_order_details (30 天滚动 JSONB)
async fn upsert_archived_order(
    pool: &PgPool,
    edge_server_id: i64,
    tenant_id: &str,
    item: &CloudSyncItem,
    now: i64,
) -> Result<(), BoxError> {
    use shared::cloud::OrderDetailSync;

    let detail_sync: OrderDetailSync = serde_json::from_value(item.data.clone())?;

    // 事务包裹三层写入：任何一步失败全部回滚
    let mut tx = pool.begin().await?;

    // 1. UPSERT cloud_archived_orders (永久摘要)
    let row: Option<(i64,)> = sqlx::query_as(
        r#"
        INSERT INTO cloud_archived_orders (
            edge_server_id, tenant_id, source_id, order_key,
            receipt_number, status, end_time, total, tax,
            prev_hash, curr_hash, data, version, synced_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
        ON CONFLICT (tenant_id, edge_server_id, order_key)
        DO UPDATE SET receipt_number = EXCLUDED.receipt_number,
                      status = EXCLUDED.status,
                      end_time = EXCLUDED.end_time,
                      total = EXCLUDED.total,
                      tax = EXCLUDED.tax,
                      prev_hash = EXCLUDED.prev_hash,
                      curr_hash = EXCLUDED.curr_hash,
                      data = EXCLUDED.data,
                      version = EXCLUDED.version,
                      synced_at = EXCLUDED.synced_at
        WHERE cloud_archived_orders.version < EXCLUDED.version
        RETURNING id
        "#,
    )
    .bind(edge_server_id)
    .bind(tenant_id)
    .bind(&item.resource_id) // source_id = order_key
    .bind(&detail_sync.order_key)
    .bind(&detail_sync.receipt_number)
    .bind(&detail_sync.status)
    .bind(detail_sync.end_time)
    .bind(detail_sync.total_amount)
    .bind(detail_sync.tax)
    .bind(&detail_sync.prev_hash)
    .bind(&detail_sync.curr_hash)
    .bind(&item.data)
    .bind(item.version as i64)
    .bind(now)
    .fetch_optional(&mut *tx)
    .await?;

    // If RETURNING id is None, version was not newer — skip desglose/detail update
    let Some((order_row_id,)) = row else {
        tx.commit().await?;
        return Ok(());
    };

    // 2. UPSERT cloud_order_desglose (永久税率分拆)
    for d in &detail_sync.desglose {
        sqlx::query(
            r#"
            INSERT INTO cloud_order_desglose (archived_order_id, tax_rate, base_amount, tax_amount)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (archived_order_id, tax_rate)
            DO UPDATE SET base_amount = EXCLUDED.base_amount,
                          tax_amount = EXCLUDED.tax_amount
            "#,
        )
        .bind(order_row_id)
        .bind(d.tax_rate)
        .bind(d.base_amount)
        .bind(d.tax_amount)
        .execute(&mut *tx)
        .await?;
    }

    // 3. UPSERT cloud_order_details (30 天滚动 JSONB)
    let detail_json = serde_json::to_value(&detail_sync.detail)?;
    sqlx::query(
        r#"
        INSERT INTO cloud_order_details (archived_order_id, detail, synced_at)
        VALUES ($1, $2, $3)
        ON CONFLICT (archived_order_id)
        DO UPDATE SET detail = EXCLUDED.detail,
                      synced_at = EXCLUDED.synced_at
        "#,
    )
    .bind(order_row_id)
    .bind(&detail_json)
    .bind(now)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(())
}

async fn upsert_store_info(
    pool: &PgPool,
    edge_server_id: i64,
    tenant_id: &str,
    item: &CloudSyncItem,
    now: i64,
) -> Result<(), BoxError> {
    sqlx::query(
        r#"
        INSERT INTO cloud_store_info (edge_server_id, tenant_id, data, version, synced_at)
        VALUES ($1, $2, $3, $4, $5)
        ON CONFLICT (edge_server_id, tenant_id)
        DO UPDATE SET data = EXCLUDED.data, version = EXCLUDED.version, synced_at = EXCLUDED.synced_at
        WHERE cloud_store_info.version < EXCLUDED.version
        "#,
    )
    .bind(edge_server_id)
    .bind(tenant_id)
    .bind(&item.data)
    .bind(item.version as i64)
    .bind(now)
    .execute(pool)
    .await?;
    Ok(())
}

async fn delete_resource(
    pool: &PgPool,
    edge_server_id: i64,
    resource: &str,
    resource_id: &str,
) -> Result<(), BoxError> {
    let table = match resource {
        "product" => "cloud_products",
        "category" => "cloud_categories",
        "daily_report" => "cloud_daily_reports",
        other => return Err(format!("Cannot delete resource type: {other}").into()),
    };

    let sql = format!("DELETE FROM {table} WHERE edge_server_id = $1 AND source_id = $2");
    sqlx::query(&sql)
        .bind(edge_server_id)
        .bind(resource_id)
        .execute(pool)
        .await?;
    Ok(())
}
