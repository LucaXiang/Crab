//! Sync data storage operations
//!
//! Upsert operations for each resource type synced from edge-servers.
//! All resources are stored in normalized tables with typed columns.

use std::collections::HashMap;

use shared::cloud::{CloudSyncItem, SyncResource};
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
        INSERT INTO edge_servers (entity_id, tenant_id, device_id, registered_at)
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
    sqlx::query("UPDATE edge_servers SET last_sync_at = $1 WHERE id = $2")
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
    resource: SyncResource,
    version: i64,
    now: i64,
) -> Result<(), BoxError> {
    sqlx::query(
        r#"
        INSERT INTO store_sync_cursors (edge_server_id, resource, last_version, updated_at)
        VALUES ($1, $2, $3, $4)
        ON CONFLICT (edge_server_id, resource)
        DO UPDATE SET last_version = GREATEST(store_sync_cursors.last_version, EXCLUDED.last_version),
                      updated_at = EXCLUDED.updated_at
        "#,
    )
    .bind(edge_server_id)
    .bind(resource.as_str())
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
        "SELECT resource, last_version FROM store_sync_cursors WHERE edge_server_id = $1",
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
    if item.action == shared::cloud::SyncAction::Delete {
        return delete_resource(pool, edge_server_id, item.resource, &item.resource_id).await;
    }

    match item.resource {
        SyncResource::Product => {
            let source_id: i64 = item.resource_id.parse()?;
            super::store::upsert_product_from_sync(
                pool,
                edge_server_id,
                source_id,
                &item.data,
                version_to_i64(item.version),
                now,
            )
            .await
        }
        SyncResource::Category => {
            let source_id: i64 = item.resource_id.parse()?;
            super::store::upsert_category_from_sync(
                pool,
                edge_server_id,
                source_id,
                &item.data,
                version_to_i64(item.version),
                now,
            )
            .await
        }
        SyncResource::ArchivedOrder => {
            upsert_archived_order(pool, edge_server_id, tenant_id, item, now).await
        }
        SyncResource::DailyReport => {
            let source_id: i64 = item.resource_id.parse()?;
            super::store::upsert_daily_report_from_sync(
                pool,
                edge_server_id,
                source_id,
                &item.data,
                now,
            )
            .await
        }
        SyncResource::StoreInfo => {
            super::store::upsert_store_info_from_sync(pool, edge_server_id, &item.data, now).await
        }
        SyncResource::Shift => {
            let source_id: i64 = item.resource_id.parse()?;
            super::store::upsert_shift_from_sync(pool, edge_server_id, source_id, &item.data, now)
                .await
        }
        SyncResource::Employee => {
            let source_id: i64 = item.resource_id.parse()?;
            super::store::upsert_employee_from_sync(
                pool,
                edge_server_id,
                source_id,
                &item.data,
                now,
            )
            .await
        }
        SyncResource::Tag => {
            let source_id: i64 = item.resource_id.parse()?;
            super::store::upsert_tag_from_sync(pool, edge_server_id, source_id, &item.data, now)
                .await
        }
        SyncResource::Attribute => {
            let source_id: i64 = item.resource_id.parse()?;
            super::store::upsert_attribute_from_sync(
                pool,
                edge_server_id,
                source_id,
                &item.data,
                now,
            )
            .await
        }
        SyncResource::AttributeBinding => {
            let source_id: i64 = item.resource_id.parse()?;
            super::store::upsert_binding_from_sync(pool, edge_server_id, source_id, &item.data, now)
                .await
        }
        SyncResource::PriceRule => {
            let source_id: i64 = item.resource_id.parse()?;
            super::store::upsert_price_rule_from_sync(
                pool,
                edge_server_id,
                source_id,
                &item.data,
                now,
            )
            .await
        }
        SyncResource::Zone => {
            let source_id: i64 = item.resource_id.parse()?;
            super::store::upsert_zone_from_sync(pool, edge_server_id, source_id, &item.data, now)
                .await
        }
        SyncResource::DiningTable => {
            let source_id: i64 = item.resource_id.parse()?;
            super::store::upsert_dining_table_from_sync(
                pool,
                edge_server_id,
                source_id,
                &item.data,
                now,
            )
            .await
        }
        SyncResource::LabelTemplate => {
            let source_id: i64 = item.resource_id.parse()?;
            super::store::upsert_label_template_from_sync(
                pool,
                edge_server_id,
                tenant_id,
                source_id,
                &item.data,
                now,
            )
            .await
        }
        other => Err(format!("Unhandled resource type: {other}").into()),
    }
}

/// Upsert archived order with four-layer storage:
/// 1. store_archived_orders (永久摘要 + desglose JSONB)
/// 2. store_order_items (永久轻量级菜品统计，含 tax_rate)
/// 3. store_order_payments (永久轻量级支付统计)
/// 4. store_order_details (30 天缓存，完整详情 JSONB)
async fn upsert_archived_order(
    pool: &PgPool,
    edge_server_id: i64,
    tenant_id: &str,
    item: &CloudSyncItem,
    now: i64,
) -> Result<(), BoxError> {
    use shared::cloud::OrderDetailSync;

    let detail_sync: OrderDetailSync = serde_json::from_value(item.data.clone())?;

    let mut tx = pool.begin().await?;

    // 1. UPSERT store_archived_orders
    let desglose_json = serde_json::to_value(&detail_sync.desglose)?;
    let row: Option<(i64,)> = sqlx::query_as(
        r#"
        INSERT INTO store_archived_orders (
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
        WHERE store_archived_orders.version <= EXCLUDED.version
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

    let Some((order_row_id,)) = row else {
        tx.commit().await?;
        return Ok(());
    };

    // 2. Replace store_order_items
    sqlx::query("DELETE FROM store_order_items WHERE archived_order_id = $1")
        .bind(order_row_id)
        .execute(&mut *tx)
        .await?;

    for item_sync in &detail_sync.detail.items {
        sqlx::query(
            r#"
            INSERT INTO store_order_items (archived_order_id, name, category_name, quantity, line_total, tax_rate, product_source_id)
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

    // 3. Replace store_order_payments
    sqlx::query("DELETE FROM store_order_payments WHERE archived_order_id = $1")
        .bind(order_row_id)
        .execute(&mut *tx)
        .await?;

    for pay in &detail_sync.detail.payments {
        if pay.cancelled {
            continue;
        }
        sqlx::query(
            r#"
            INSERT INTO store_order_payments (archived_order_id, method, amount)
            VALUES ($1, $2, $3)
            "#,
        )
        .bind(order_row_id)
        .bind(&pay.method)
        .bind(pay.amount)
        .execute(&mut *tx)
        .await?;
    }

    // 4. Replace store_order_events
    sqlx::query("DELETE FROM store_order_events WHERE archived_order_id = $1")
        .bind(order_row_id)
        .execute(&mut *tx)
        .await?;

    for ev in &detail_sync.detail.events {
        sqlx::query(
            r#"
            INSERT INTO store_order_events (archived_order_id, seq, event_type, timestamp, operator_id, operator_name)
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

    // 5. UPSERT store_order_details (30 天缓存)
    let detail_json = serde_json::to_value(&detail_sync.detail)?;
    sqlx::query(
        r#"
        INSERT INTO store_order_details (archived_order_id, detail, synced_at)
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

async fn delete_resource(
    pool: &PgPool,
    edge_server_id: i64,
    resource: SyncResource,
    resource_id: &str,
) -> Result<(), BoxError> {
    let source_id: i64 = resource_id.parse()?;

    match resource {
        SyncResource::Product => {
            super::store::delete_product(pool, edge_server_id, source_id).await
        }
        SyncResource::Category => {
            super::store::delete_category(pool, edge_server_id, source_id).await
        }
        SyncResource::DailyReport => {
            sqlx::query(
                "DELETE FROM store_daily_reports WHERE edge_server_id = $1 AND source_id = $2",
            )
            .bind(edge_server_id)
            .bind(source_id)
            .execute(pool)
            .await?;
            Ok(())
        }
        SyncResource::Shift => {
            sqlx::query("DELETE FROM store_shifts WHERE edge_server_id = $1 AND source_id = $2")
                .bind(edge_server_id)
                .bind(source_id)
                .execute(pool)
                .await?;
            Ok(())
        }
        SyncResource::Employee => {
            sqlx::query("DELETE FROM store_employees WHERE edge_server_id = $1 AND source_id = $2")
                .bind(edge_server_id)
                .bind(source_id)
                .execute(pool)
                .await?;
            Ok(())
        }
        SyncResource::Tag => {
            sqlx::query("DELETE FROM store_tags WHERE edge_server_id = $1 AND source_id = $2")
                .bind(edge_server_id)
                .bind(source_id)
                .execute(pool)
                .await?;
            Ok(())
        }
        SyncResource::Attribute => {
            sqlx::query(
                "DELETE FROM store_attributes WHERE edge_server_id = $1 AND source_id = $2",
            )
            .bind(edge_server_id)
            .bind(source_id)
            .execute(pool)
            .await?;
            Ok(())
        }
        SyncResource::AttributeBinding => {
            sqlx::query(
                "DELETE FROM store_attribute_bindings WHERE edge_server_id = $1 AND source_id = $2",
            )
            .bind(edge_server_id)
            .bind(source_id)
            .execute(pool)
            .await?;
            Ok(())
        }
        SyncResource::PriceRule => {
            sqlx::query(
                "DELETE FROM store_price_rules WHERE edge_server_id = $1 AND source_id = $2",
            )
            .bind(edge_server_id)
            .bind(source_id)
            .execute(pool)
            .await?;
            Ok(())
        }
        SyncResource::Zone => {
            sqlx::query("DELETE FROM store_zones WHERE edge_server_id = $1 AND source_id = $2")
                .bind(edge_server_id)
                .bind(source_id)
                .execute(pool)
                .await?;
            Ok(())
        }
        SyncResource::DiningTable => {
            sqlx::query(
                "DELETE FROM store_dining_tables WHERE edge_server_id = $1 AND source_id = $2",
            )
            .bind(edge_server_id)
            .bind(source_id)
            .execute(pool)
            .await?;
            Ok(())
        }
        SyncResource::LabelTemplate => {
            sqlx::query(
                "DELETE FROM store_label_templates WHERE edge_server_id = $1 AND source_id = $2",
            )
            .bind(edge_server_id)
            .bind(source_id)
            .execute(pool)
            .await?;
            Ok(())
        }
        other => Err(format!("Cannot delete resource type: {other}").into()),
    }
}
