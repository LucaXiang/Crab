//! Sync data storage operations
//!
//! Upsert operations for each resource type synced from edge-servers.
//! Catalog resources (product, category, tag, attribute, price_rule) are stored
//! in normalized tables. Other resources use JSONB mirror tables.

use std::collections::HashMap;

use shared::cloud::CloudSyncItem;
use sqlx::PgPool;

type BoxError = Box<dyn std::error::Error + Send + Sync>;

/// Safely convert u64 version to i64 for PostgreSQL storage.
/// Clamps to i64::MAX on overflow (practically unreachable).
fn version_to_i64(version: u64) -> i64 {
    i64::try_from(version).unwrap_or(i64::MAX)
}

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

/// Get all sync cursors for an edge-server (resource → last_version)
pub async fn get_cursors(
    pool: &PgPool,
    edge_server_id: i64,
) -> Result<HashMap<String, u64>, BoxError> {
    let rows: Vec<(String, i64)> = sqlx::query_as(
        "SELECT resource, last_version FROM cloud_sync_cursors WHERE edge_server_id = $1",
    )
    .bind(edge_server_id)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|(resource, version)| (resource, version.max(0) as u64))
        .collect())
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
            let source_id: i64 = item.resource_id.parse()?;
            super::catalog::upsert_product_from_sync(
                pool,
                edge_server_id,
                source_id,
                &item.data,
                version_to_i64(item.version),
                now,
            )
            .await
        }
        "category" => {
            let source_id: i64 = item.resource_id.parse()?;
            super::catalog::upsert_category_from_sync(
                pool,
                edge_server_id,
                source_id,
                &item.data,
                version_to_i64(item.version),
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
        "shift" => upsert_generic(pool, "cloud_shifts", edge_server_id, tenant_id, item, now).await,
        "employee" => {
            upsert_generic(
                pool,
                "cloud_employees",
                edge_server_id,
                tenant_id,
                item,
                now,
            )
            .await
        }
        "tag" => {
            let source_id: i64 = item.resource_id.parse()?;
            super::catalog::upsert_tag_from_sync(pool, edge_server_id, source_id, &item.data, now)
                .await
        }
        "attribute" => {
            let source_id: i64 = item.resource_id.parse()?;
            super::catalog::upsert_attribute_from_sync(
                pool,
                edge_server_id,
                source_id,
                &item.data,
                now,
            )
            .await
        }
        "attribute_binding" => {
            let source_id: i64 = item.resource_id.parse()?;
            super::catalog::upsert_binding_from_sync(
                pool,
                edge_server_id,
                source_id,
                &item.data,
                now,
            )
            .await
        }
        "price_rule" => {
            let source_id: i64 = item.resource_id.parse()?;
            super::catalog::upsert_price_rule_from_sync(
                pool,
                edge_server_id,
                source_id,
                &item.data,
                now,
            )
            .await
        }
        "zone" => upsert_generic(pool, "cloud_zones", edge_server_id, tenant_id, item, now).await,
        "dining_table" => {
            upsert_generic(
                pool,
                "cloud_dining_tables",
                edge_server_id,
                tenant_id,
                item,
                now,
            )
            .await
        }
        other => Err(format!("Unknown resource type: {other}").into()),
    }
}

/// Generic upsert for simple mirror tables (daily_report, shift, employee)
///
/// `table` must be a `&'static str` from the match in `upsert_resource` — never user input.
async fn upsert_generic(
    pool: &PgPool,
    table: &'static str,
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
        .bind(version_to_i64(item.version))
        .bind(now)
        .execute(pool)
        .await?;
    Ok(())
}

/// Upsert archived order with four-layer storage:
/// 1. cloud_archived_orders (永久摘要 + desglose JSONB)
/// 2. cloud_order_items (永久轻量级菜品统计，含 tax_rate)
/// 3. cloud_order_payments (永久轻量级支付统计)
/// 4. cloud_order_details (30 天缓存，完整详情 JSONB)
async fn upsert_archived_order(
    pool: &PgPool,
    edge_server_id: i64,
    tenant_id: &str,
    item: &CloudSyncItem,
    now: i64,
) -> Result<(), BoxError> {
    use shared::cloud::OrderDetailSync;

    let detail_sync: OrderDetailSync = serde_json::from_value(item.data.clone())?;

    // 事务包裹所有写入：任何一步失败全部回滚
    let mut tx = pool.begin().await?;

    // 1. UPSERT cloud_archived_orders (永久摘要 + desglose + 统计字段)
    let desglose_json = serde_json::to_value(&detail_sync.desglose)?;
    let row: Option<(i64,)> = sqlx::query_as(
        r#"
        INSERT INTO cloud_archived_orders (
            edge_server_id, tenant_id, source_id, order_key,
            receipt_number, status, end_time, total, tax,
            prev_hash, curr_hash, desglose,
            guest_count, discount_amount, void_type, loss_amount, start_time,
            version, synced_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19)
        ON CONFLICT (tenant_id, edge_server_id, order_key)
        DO UPDATE SET receipt_number = EXCLUDED.receipt_number,
                      status = EXCLUDED.status,
                      end_time = EXCLUDED.end_time,
                      total = EXCLUDED.total,
                      tax = EXCLUDED.tax,
                      prev_hash = EXCLUDED.prev_hash,
                      curr_hash = EXCLUDED.curr_hash,
                      desglose = EXCLUDED.desglose,
                      guest_count = EXCLUDED.guest_count,
                      discount_amount = EXCLUDED.discount_amount,
                      void_type = EXCLUDED.void_type,
                      loss_amount = EXCLUDED.loss_amount,
                      start_time = EXCLUDED.start_time,
                      version = EXCLUDED.version,
                      synced_at = EXCLUDED.synced_at
        WHERE cloud_archived_orders.version <= EXCLUDED.version
        RETURNING id
        "#,
    )
    .bind(edge_server_id)
    .bind(tenant_id)
    .bind(&item.resource_id)
    .bind(&detail_sync.order_key)
    .bind(&detail_sync.receipt_number)
    .bind(&detail_sync.status)
    .bind(detail_sync.end_time)
    .bind(detail_sync.total_amount)
    .bind(detail_sync.tax)
    .bind(&detail_sync.prev_hash)
    .bind(&detail_sync.curr_hash)
    .bind(&desglose_json)
    .bind(detail_sync.detail.guest_count)
    .bind(detail_sync.detail.discount_amount)
    .bind(&detail_sync.detail.void_type)
    .bind(detail_sync.detail.loss_amount)
    .bind(detail_sync.detail.start_time)
    .bind(version_to_i64(item.version))
    .bind(now)
    .fetch_optional(&mut *tx)
    .await?;

    // If RETURNING id is None, version was not newer — skip sub-table updates
    let Some((order_row_id,)) = row else {
        tx.commit().await?;
        return Ok(());
    };

    // 2. Replace cloud_order_items (永久轻量级菜品，用于统计)
    sqlx::query("DELETE FROM cloud_order_items WHERE archived_order_id = $1")
        .bind(order_row_id)
        .execute(&mut *tx)
        .await?;

    for item_sync in &detail_sync.detail.items {
        sqlx::query(
            r#"
            INSERT INTO cloud_order_items (archived_order_id, name, category_name, quantity, line_total, tax_rate, product_source_id)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
        )
        .bind(order_row_id)
        .bind(&item_sync.name)
        .bind(&item_sync.category_name)
        .bind(item_sync.quantity)
        .bind(item_sync.line_total)
        .bind(item_sync.tax_rate)
        .bind(item_sync.product_source_id)
        .execute(&mut *tx)
        .await?;
    }

    // 4. Replace cloud_order_payments (永久轻量级支付，用于统计)
    sqlx::query("DELETE FROM cloud_order_payments WHERE archived_order_id = $1")
        .bind(order_row_id)
        .execute(&mut *tx)
        .await?;

    for pay in &detail_sync.detail.payments {
        if pay.cancelled {
            continue; // 跳过已取消的支付
        }
        sqlx::query(
            r#"
            INSERT INTO cloud_order_payments (archived_order_id, method, amount)
            VALUES ($1, $2, $3)
            "#,
        )
        .bind(order_row_id)
        .bind(&pay.method)
        .bind(pay.amount)
        .execute(&mut *tx)
        .await?;
    }

    // 5. Replace cloud_order_events (永久，用于 Red Flags 监控)
    sqlx::query("DELETE FROM cloud_order_events WHERE archived_order_id = $1")
        .bind(order_row_id)
        .execute(&mut *tx)
        .await?;

    for ev in &detail_sync.detail.events {
        sqlx::query(
            r#"
            INSERT INTO cloud_order_events (archived_order_id, seq, event_type, timestamp, operator_id, operator_name)
            VALUES ($1, $2, $3, $4, $5, $6)
            "#,
        )
        .bind(order_row_id)
        .bind(ev.seq)
        .bind(&ev.event_type)
        .bind(ev.timestamp)
        .bind(ev.operator_id)
        .bind(&ev.operator_name)
        .execute(&mut *tx)
        .await?;
    }

    // 6. UPSERT cloud_order_details (30 天缓存，完整详情)
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
    .bind(version_to_i64(item.version))
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
    let source_id_i64: i64 = resource_id.parse().unwrap_or(0);

    // Catalog resources use dedicated functions (CASCADE deletes children)
    match resource {
        "product" => {
            return super::catalog::delete_product(pool, edge_server_id, source_id_i64).await;
        }
        "category" => {
            return super::catalog::delete_category(pool, edge_server_id, source_id_i64).await;
        }
        _ => {}
    }

    let query = match resource {
        "daily_report" => sqlx::query(
            "DELETE FROM cloud_daily_reports WHERE edge_server_id = $1 AND source_id = $2",
        ),
        "shift" => {
            sqlx::query("DELETE FROM cloud_shifts WHERE edge_server_id = $1 AND source_id = $2")
        }
        "employee" => {
            sqlx::query("DELETE FROM cloud_employees WHERE edge_server_id = $1 AND source_id = $2")
        }
        "tag" => {
            sqlx::query("DELETE FROM catalog_tags WHERE edge_server_id = $1 AND source_id = $2")
        }
        "attribute" => sqlx::query(
            "DELETE FROM catalog_attributes WHERE edge_server_id = $1 AND source_id = $2",
        ),
        "attribute_binding" => sqlx::query(
            "DELETE FROM catalog_attribute_bindings WHERE edge_server_id = $1 AND source_id = $2",
        ),
        "price_rule" => sqlx::query(
            "DELETE FROM catalog_price_rules WHERE edge_server_id = $1 AND source_id = $2",
        ),
        "zone" => {
            sqlx::query("DELETE FROM cloud_zones WHERE edge_server_id = $1 AND source_id = $2")
        }
        "dining_table" => sqlx::query(
            "DELETE FROM cloud_dining_tables WHERE edge_server_id = $1 AND source_id = $2",
        ),
        other => return Err(format!("Cannot delete resource type: {other}").into()),
    };

    query
        .bind(edge_server_id)
        .bind(resource_id)
        .execute(pool)
        .await?;
    Ok(())
}
