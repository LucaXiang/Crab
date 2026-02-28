//! Sync data storage operations
//!
//! Upsert operations for each resource type synced from edge-servers.
//! All resources are stored in normalized tables with typed columns.

use std::collections::HashMap;

use shared::cloud::{CloudSyncItem, SyncResource};
use shared::models::store_info::StoreInfo;
use sqlx::PgPool;

type BoxError = Box<dyn std::error::Error + Send + Sync>;

/// Side-effect produced by `upsert_resource` that callers should handle.
pub enum SyncEffect {
    None,
    /// StoreInfo was upserted — callers should broadcast to consoles.
    StoreInfoUpdated(Box<StoreInfo>),
}

/// Safely convert u64 version to i64 for PostgreSQL storage.
/// Clamps to i64::MAX on overflow (practically unreachable).
fn version_to_i64(version: u64) -> i64 {
    i64::try_from(version).unwrap_or(i64::MAX)
}

/// Ensure edge-server is registered, returning its database ID (snowflake)
pub async fn ensure_store(
    pool: &PgPool,
    entity_id: &str,
    tenant_id: i64,
    device_id: &str,
    now: i64,
) -> Result<i64, BoxError> {
    let id = shared::util::snowflake_id();
    let row: (i64,) = sqlx::query_as(
        r#"
        INSERT INTO stores (id, entity_id, tenant_id, device_id, registered_at, store_number, alias)
        VALUES ($1, $2, $3, $4, $5,
                (SELECT COALESCE(MAX(store_number), 0) + 1 FROM stores WHERE tenant_id = $3 AND status = 'active'),
                'Store' || LPAD((SELECT COALESCE(MAX(store_number), 0) + 1 FROM stores WHERE tenant_id = $3 AND status = 'active')::TEXT, 2, '0'))
        ON CONFLICT (entity_id, tenant_id) DO UPDATE SET device_id = EXCLUDED.device_id
        RETURNING id
        "#,
    )
    .bind(id)
    .bind(entity_id)
    .bind(tenant_id)
    .bind(device_id)
    .bind(now)
    .fetch_one(pool)
    .await?;

    Ok(row.0)
}

/// Get the store_number for an edge-server
pub async fn get_store_number(
    pool: &PgPool,
    entity_id: &str,
    tenant_id: i64,
) -> Result<u32, BoxError> {
    let row: (i32,) =
        sqlx::query_as("SELECT store_number FROM stores WHERE entity_id = $1 AND tenant_id = $2")
            .bind(entity_id)
            .bind(tenant_id)
            .fetch_one(pool)
            .await?;

    Ok(row.0 as u32)
}

/// Update last_sync_at for an edge-server
pub async fn update_last_sync(pool: &PgPool, store_id: i64, now: i64) -> Result<(), BoxError> {
    sqlx::query("UPDATE stores SET last_sync_at = $1 WHERE id = $2")
        .bind(now)
        .bind(store_id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Update sync cursor for a resource
pub async fn update_cursor(
    pool: &PgPool,
    store_id: i64,
    resource: SyncResource,
    version: i64,
    now: i64,
) -> Result<(), BoxError> {
    sqlx::query(
        r#"
        INSERT INTO store_sync_cursors (store_id, resource, last_version, updated_at)
        VALUES ($1, $2, $3, $4)
        ON CONFLICT (store_id, resource)
        DO UPDATE SET last_version = GREATEST(store_sync_cursors.last_version, EXCLUDED.last_version),
                      updated_at = EXCLUDED.updated_at
        "#,
    )
    .bind(store_id)
    .bind(resource.as_str())
    .bind(version)
    .bind(now)
    .execute(pool)
    .await?;
    Ok(())
}

/// Batch update sync cursors for multiple resources at once
pub async fn update_cursors_batch(
    pool: &PgPool,
    store_id: i64,
    cursors: &[(&str, i64)],
    now: i64,
) -> Result<(), BoxError> {
    if cursors.is_empty() {
        return Ok(());
    }
    let eids: Vec<i64> = cursors.iter().map(|_| store_id).collect();
    let resources: Vec<&str> = cursors.iter().map(|(r, _)| *r).collect();
    let versions: Vec<i64> = cursors.iter().map(|(_, v)| *v).collect();
    let nows: Vec<i64> = cursors.iter().map(|_| now).collect();
    sqlx::query(
        r#"
        INSERT INTO store_sync_cursors (store_id, resource, last_version, updated_at)
        SELECT * FROM UNNEST($1::bigint[], $2::text[], $3::bigint[], $4::bigint[])
        ON CONFLICT (store_id, resource)
        DO UPDATE SET last_version = GREATEST(store_sync_cursors.last_version, EXCLUDED.last_version),
                      updated_at = EXCLUDED.updated_at
        "#,
    )
    .bind(&eids)
    .bind(&resources)
    .bind(&versions)
    .bind(&nows)
    .execute(pool)
    .await?;
    Ok(())
}

/// Get all sync cursors for an edge-server (resource → last_version)
pub async fn get_cursors(pool: &PgPool, store_id: i64) -> Result<HashMap<String, u64>, BoxError> {
    let rows: Vec<(String, i64)> =
        sqlx::query_as("SELECT resource, last_version FROM store_sync_cursors WHERE store_id = $1")
            .bind(store_id)
            .fetch_all(pool)
            .await?;

    Ok(rows
        .into_iter()
        .map(|(resource, version)| (resource, version.max(0) as u64))
        .collect())
}

/// Upsert a resource based on its type.
///
/// Returns `SyncEffect` so callers can handle side-effects (e.g. broadcasting StoreInfo changes).
pub async fn upsert_resource(
    pool: &PgPool,
    store_id: i64,
    tenant_id: i64,
    item: &CloudSyncItem,
    now: i64,
) -> Result<SyncEffect, BoxError> {
    if item.action == shared::cloud::SyncAction::Delete {
        delete_resource(pool, store_id, item.resource, item.resource_id).await?;
        return Ok(SyncEffect::None);
    }

    match item.resource {
        SyncResource::Product => {
            let source_id = item.resource_id;
            super::store::upsert_product_from_sync(
                pool,
                store_id,
                source_id,
                &item.data,
                version_to_i64(item.version),
                now,
            )
            .await?;
        }
        SyncResource::Category => {
            let source_id = item.resource_id;
            super::store::upsert_category_from_sync(
                pool,
                store_id,
                source_id,
                &item.data,
                version_to_i64(item.version),
                now,
            )
            .await?;
        }
        SyncResource::ArchivedOrder => {
            upsert_archived_order(pool, store_id, tenant_id, item, now).await?;
        }
        SyncResource::DailyReport => {
            let source_id = item.resource_id;
            super::store::upsert_daily_report_from_sync(
                pool, store_id, tenant_id, source_id, &item.data, now,
            )
            .await?;
        }
        SyncResource::StoreInfo => {
            let info =
                super::store::upsert_store_info_from_sync(pool, store_id, &item.data, now).await?;
            return Ok(SyncEffect::StoreInfoUpdated(Box::new(info)));
        }
        SyncResource::Shift => {
            let source_id = item.resource_id;
            super::store::upsert_shift_from_sync(
                pool, store_id, tenant_id, source_id, &item.data, now,
            )
            .await?;
        }
        SyncResource::Employee => {
            let source_id = item.resource_id;
            super::store::upsert_employee_from_sync(pool, store_id, source_id, &item.data, now)
                .await?;
        }
        SyncResource::Tag => {
            let source_id = item.resource_id;
            super::store::upsert_tag_from_sync(pool, store_id, source_id, &item.data, now).await?;
        }
        SyncResource::Attribute => {
            let source_id = item.resource_id;
            super::store::upsert_attribute_from_sync(pool, store_id, source_id, &item.data, now)
                .await?;
        }
        SyncResource::AttributeBinding => {
            let source_id = item.resource_id;
            super::store::upsert_binding_from_sync(pool, store_id, source_id, &item.data, now)
                .await?;
        }
        SyncResource::PriceRule => {
            let source_id = item.resource_id;
            super::store::upsert_price_rule_from_sync(pool, store_id, source_id, &item.data, now)
                .await?;
        }
        SyncResource::Zone => {
            let source_id = item.resource_id;
            super::store::upsert_zone_from_sync(pool, store_id, source_id, &item.data, now).await?;
        }
        SyncResource::DiningTable => {
            let source_id = item.resource_id;
            super::store::upsert_dining_table_from_sync(pool, store_id, source_id, &item.data, now)
                .await?;
        }
        SyncResource::LabelTemplate => {
            let source_id = item.resource_id;
            super::store::upsert_label_template_from_sync(
                pool, store_id, tenant_id, source_id, &item.data, now,
            )
            .await?;
        }
        SyncResource::CreditNote => {
            upsert_credit_note(pool, store_id, tenant_id, item, now).await?;
        }
        SyncResource::Invoice => {
            upsert_invoice(pool, store_id, tenant_id, item, now).await?;
        }
        SyncResource::Anulacion => {
            upsert_anulacion(pool, store_id, tenant_id, item, now).await?;
        }
        other => return Err(format!("Unhandled resource type: {other}").into()),
    }

    Ok(SyncEffect::None)
}

/// Upsert credit note — summary columns + detail JSONB.
async fn upsert_credit_note(
    pool: &PgPool,
    store_id: i64,
    tenant_id: i64,
    item: &CloudSyncItem,
    now: i64,
) -> Result<(), BoxError> {
    use shared::cloud::CreditNoteSync;

    let cn: CreditNoteSync = serde_json::from_value(item.data.clone())?;
    let source_id = item.resource_id;

    // Verify hash chain integrity after deserialization
    if let Some(recomputed) = cn.verify_hash() {
        tracing::warn!(
            credit_note_number = %cn.credit_note_number,
            source_id,
            stored = %cn.curr_hash,
            recomputed,
            "Credit note hash verification failed after deserialization"
        );
    }

    let detail_json = serde_json::to_value(&cn)?;

    sqlx::query(
        r#"
        INSERT INTO store_credit_notes (
            store_id, tenant_id, source_id, credit_note_number,
            original_order_id, original_receipt,
            subtotal_credit, tax_credit, total_credit,
            refund_method, reason, note,
            operator_name, authorizer_name,
            prev_hash, curr_hash, created_at,
            detail, version, synced_at
        )
        VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20)
        ON CONFLICT (tenant_id, store_id, source_id)
        DO UPDATE SET credit_note_number = EXCLUDED.credit_note_number,
                      total_credit = EXCLUDED.total_credit,
                      prev_hash = EXCLUDED.prev_hash,
                      curr_hash = EXCLUDED.curr_hash,
                      detail = EXCLUDED.detail,
                      version = EXCLUDED.version,
                      synced_at = EXCLUDED.synced_at
        WHERE store_credit_notes.version <= EXCLUDED.version
        "#,
    )
    .bind(store_id)
    .bind(tenant_id)
    .bind(source_id)
    .bind(&cn.credit_note_number)
    .bind(cn.original_order_id)
    .bind(&cn.original_receipt)
    .bind(cn.subtotal_credit)
    .bind(cn.tax_credit)
    .bind(cn.total_credit)
    .bind(&cn.refund_method)
    .bind(&cn.reason)
    .bind(&cn.note)
    .bind(&cn.operator_name)
    .bind(&cn.authorizer_name)
    .bind(&cn.prev_hash)
    .bind(&cn.curr_hash)
    .bind(cn.created_at)
    .bind(&detail_json)
    .bind(version_to_i64(item.version))
    .bind(now)
    .execute(pool)
    .await?;

    Ok(())
}

/// Upsert archived order — single table with summary columns + detail JSONB.
async fn upsert_archived_order(
    pool: &PgPool,
    store_id: i64,
    tenant_id: i64,
    item: &CloudSyncItem,
    now: i64,
) -> Result<(), BoxError> {
    use shared::cloud::OrderDetailSync;

    let detail_sync: OrderDetailSync = serde_json::from_value(item.data.clone())?;

    // Verify hash chain integrity after deserialization
    // If last_event_hash is None (pre-upgrade data), skip silently
    if let Some(recomputed) = detail_sync.verify_hash()
        && detail_sync.last_event_hash.is_some()
    {
        tracing::warn!(
            order_id = %detail_sync.order_id,
            receipt = %detail_sync.receipt_number,
            stored = %detail_sync.curr_hash,
            recomputed,
            "Order hash verification failed after deserialization"
        );
    }

    let desglose_json = serde_json::to_value(&detail_sync.desglose)?;
    let detail_json = serde_json::to_value(&detail_sync.detail)?;

    sqlx::query(
        r#"
        INSERT INTO store_archived_orders (
            store_id, tenant_id, source_id, order_id,
            receipt_number, status, end_time, total, tax,
            prev_hash, curr_hash, last_event_hash, desglose,
            guest_count, discount_amount, void_type, loss_amount, start_time,
            detail, version, synced_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21)
        ON CONFLICT (tenant_id, store_id, order_id)
        DO UPDATE SET receipt_number = EXCLUDED.receipt_number,
                      status = EXCLUDED.status,
                      end_time = EXCLUDED.end_time,
                      total = EXCLUDED.total,
                      tax = EXCLUDED.tax,
                      prev_hash = EXCLUDED.prev_hash,
                      curr_hash = EXCLUDED.curr_hash,
                      last_event_hash = EXCLUDED.last_event_hash,
                      desglose = EXCLUDED.desglose,
                      guest_count = EXCLUDED.guest_count,
                      discount_amount = EXCLUDED.discount_amount,
                      void_type = EXCLUDED.void_type,
                      loss_amount = EXCLUDED.loss_amount,
                      start_time = EXCLUDED.start_time,
                      detail = EXCLUDED.detail,
                      version = EXCLUDED.version,
                      synced_at = EXCLUDED.synced_at
        WHERE store_archived_orders.version <= EXCLUDED.version
        "#,
    )
    .bind(store_id)
    .bind(tenant_id)
    .bind(item.resource_id)
    .bind(detail_sync.order_id)
    .bind(&detail_sync.receipt_number)
    .bind(&detail_sync.status)
    .bind(detail_sync.end_time)
    .bind(detail_sync.total_amount)
    .bind(detail_sync.tax)
    .bind(&detail_sync.prev_hash)
    .bind(&detail_sync.curr_hash)
    .bind(&detail_sync.last_event_hash)
    .bind(&desglose_json)
    .bind(detail_sync.detail.guest_count)
    .bind(detail_sync.detail.discount_amount)
    .bind(&detail_sync.detail.void_type)
    .bind(detail_sync.detail.loss_amount)
    .bind(detail_sync.detail.start_time)
    .bind(&detail_json)
    .bind(version_to_i64(item.version))
    .bind(now)
    .execute(pool)
    .await?;

    Ok(())
}

/// Map deletable SyncResource to its PostgreSQL table name.
fn deletable_table(resource: SyncResource) -> Option<&'static str> {
    match resource {
        SyncResource::Product => Some("store_products"),
        SyncResource::Category => Some("store_categories"),
        SyncResource::DailyReport => Some("store_daily_reports"),
        SyncResource::Shift => Some("store_shifts"),
        SyncResource::Employee => Some("store_employees"),
        SyncResource::Tag => Some("store_tags"),
        SyncResource::Attribute => Some("store_attributes"),
        SyncResource::AttributeBinding => Some("store_attribute_bindings"),
        SyncResource::PriceRule => Some("store_price_rules"),
        SyncResource::Zone => Some("store_zones"),
        SyncResource::DiningTable => Some("store_dining_tables"),
        SyncResource::LabelTemplate => Some("store_label_templates"),
        _ => None,
    }
}

async fn delete_resource(
    pool: &PgPool,
    store_id: i64,
    resource: SyncResource,
    resource_id: i64,
) -> Result<(), BoxError> {
    let table = deletable_table(resource)
        .ok_or_else(|| format!("Cannot delete resource type: {resource}"))?;

    // All deletable resources use the same (store_id, source_id) key pattern.
    // FK CASCADE handles child rows (e.g. product specs, category tags).
    let sql = format!("DELETE FROM {table} WHERE store_id = $1 AND source_id = $2");
    sqlx::query(&sql)
        .bind(store_id)
        .bind(resource_id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Upsert Verifactu anulación (RegistroFacturaBaja) — invoice voiding record.
async fn upsert_anulacion(
    pool: &PgPool,
    store_id: i64,
    tenant_id: i64,
    item: &CloudSyncItem,
    now: i64,
) -> Result<(), BoxError> {
    use shared::cloud::sync::AnulacionSync;

    let anu: AnulacionSync = serde_json::from_value(item.data.clone())?;
    let source_id = item.resource_id;

    // Verify chain hash integrity (warn only, consistent with order/credit_note)
    if let Some(recomputed) = anu.verify_hash() {
        tracing::warn!(
            anulacion_number = %anu.anulacion_number,
            stored = %anu.curr_hash,
            recomputed = %recomputed,
            "Chain hash verification failed on anulacion sync"
        );
    }

    // Verify huella integrity before storing
    if let Some(mismatch) = anu.verify_huella() {
        tracing::warn!(
            anulacion_number = %anu.anulacion_number,
            "Huella verification failed on anulacion sync: {mismatch}"
        );
        return Err(format!(
            "huella verification failed for anulacion {}: {mismatch}",
            anu.anulacion_number
        )
        .into());
    }

    let detail_json = serde_json::to_value(&anu)?;

    sqlx::query(
        r#"
        INSERT INTO store_anulaciones (
            store_id, tenant_id, source_id, anulacion_number, serie,
            original_invoice_id, original_invoice_number, original_order_id,
            huella, prev_huella, fecha_expedicion, fecha_hora_registro,
            nif, nombre_razon, reason, note, operator_id, operator_name,
            prev_hash, curr_hash,
            detail, aeat_status, version, created_at, synced_at
        )
        VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20,$21,$22,$23,$24,$25)
        ON CONFLICT (tenant_id, store_id, source_id)
        DO UPDATE SET anulacion_number = EXCLUDED.anulacion_number,
                      huella = EXCLUDED.huella,
                      prev_huella = EXCLUDED.prev_huella,
                      detail = EXCLUDED.detail,
                      version = EXCLUDED.version,
                      synced_at = EXCLUDED.synced_at
        WHERE store_anulaciones.version <= EXCLUDED.version
        "#,
    )
    .bind(store_id)
    .bind(tenant_id)
    .bind(source_id)
    .bind(&anu.anulacion_number)
    .bind(&anu.serie)
    .bind(anu.original_invoice_id)
    .bind(&anu.original_invoice_number)
    .bind(anu.original_order_pk)
    .bind(&anu.huella)
    .bind(&anu.prev_huella)
    .bind(&anu.fecha_expedicion)
    .bind(&anu.fecha_hora_registro)
    .bind(&anu.nif)
    .bind(&anu.nombre_razon)
    .bind(&anu.reason)
    .bind(&anu.note)
    .bind(anu.operator_id)
    .bind(&anu.operator_name)
    .bind(&anu.prev_hash)
    .bind(&anu.curr_hash)
    .bind(&detail_json)
    .bind("PENDING")
    .bind(version_to_i64(item.version))
    .bind(anu.created_at)
    .bind(now)
    .execute(pool)
    .await?;

    Ok(())
}

/// Upsert Verifactu invoice — summary columns + detail JSONB.
/// Cloud never modifies invoice content (edge is authoritative for creation).
/// Cloud only updates aeat_status/aeat_csv after AEAT submission (separate path).
async fn upsert_invoice(
    pool: &PgPool,
    store_id: i64,
    tenant_id: i64,
    item: &CloudSyncItem,
    now: i64,
) -> Result<(), BoxError> {
    use shared::cloud::sync::InvoiceSync;

    let inv: InvoiceSync = serde_json::from_value(item.data.clone())?;
    let source_id = item.resource_id;

    // Verify huella integrity before storing
    if let Some(mismatch) = inv.verify_huella() {
        tracing::warn!(
            invoice_number = %inv.invoice_number,
            "Huella verification failed on invoice sync: {mismatch}"
        );
        return Err(format!(
            "huella verification failed for {}: {mismatch}",
            inv.invoice_number
        )
        .into());
    }

    let detail_json = serde_json::to_value(&inv)?;

    sqlx::query(
        r#"
        INSERT INTO store_invoices (
            store_id, tenant_id, source_id, invoice_number, serie,
            tipo_factura, source_type, source_pk,
            subtotal, tax, total,
            huella, prev_huella, fecha_expedicion,
            nif, nombre_razon,
            factura_rectificada_id, factura_rectificada_num,
            factura_sustituida_id, factura_sustituida_num,
            customer_nif, customer_nombre, customer_address, customer_email, customer_phone,
            aeat_status, created_at,
            detail, version, synced_at
        )
        VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20,$21,$22,$23,$24,$25,$26,$27,$28,$29,$30)
        ON CONFLICT (tenant_id, store_id, source_id)
        DO UPDATE SET invoice_number = EXCLUDED.invoice_number,
                      total = EXCLUDED.total,
                      huella = EXCLUDED.huella,
                      prev_huella = EXCLUDED.prev_huella,
                      detail = EXCLUDED.detail,
                      version = EXCLUDED.version,
                      synced_at = EXCLUDED.synced_at
        WHERE store_invoices.version <= EXCLUDED.version
        "#,
    )
    .bind(store_id)
    .bind(tenant_id)
    .bind(source_id)
    .bind(&inv.invoice_number)
    .bind(&inv.serie)
    .bind(inv.tipo_factura.as_str())
    .bind(inv.source_type.as_str())
    .bind(inv.source_pk)
    .bind(inv.subtotal)
    .bind(inv.tax)
    .bind(inv.total)
    .bind(&inv.huella)
    .bind(&inv.prev_huella)
    .bind(&inv.fecha_expedicion)
    .bind(&inv.nif)
    .bind(&inv.nombre_razon)
    .bind(inv.factura_rectificada_id)
    .bind(&inv.factura_rectificada_num)
    .bind(inv.factura_sustituida_id)
    .bind(&inv.factura_sustituida_num)
    .bind(&inv.customer_nif)
    .bind(&inv.customer_nombre)
    .bind(&inv.customer_address)
    .bind(&inv.customer_email)
    .bind(&inv.customer_phone)
    .bind("PENDING")
    .bind(inv.created_at)
    .bind(&detail_json)
    .bind(version_to_i64(item.version))
    .bind(now)
    .execute(pool)
    .await?;

    Ok(())
}

/// 将已有门店绑定到新的 entity_id/device_id
pub async fn rebind_store(
    pool: &PgPool,
    store_id: i64,
    entity_id: &str,
    device_id: &str,
) -> Result<(), BoxError> {
    sqlx::query(
        "UPDATE stores SET entity_id = $1, device_id = $2 WHERE id = $3 AND status = 'active'",
    )
    .bind(entity_id)
    .bind(device_id)
    .bind(store_id)
    .execute(pool)
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_to_i64_normal() {
        assert_eq!(version_to_i64(0), 0);
        assert_eq!(version_to_i64(1), 1);
        assert_eq!(version_to_i64(i64::MAX as u64), i64::MAX);
    }

    #[test]
    fn version_to_i64_overflow_clamps() {
        assert_eq!(version_to_i64(u64::MAX), i64::MAX);
        assert_eq!(version_to_i64(i64::MAX as u64 + 1), i64::MAX);
    }

    #[test]
    fn deletable_table_covers_all_simple_resources() {
        let expected = [
            (SyncResource::Product, "store_products"),
            (SyncResource::Category, "store_categories"),
            (SyncResource::DailyReport, "store_daily_reports"),
            (SyncResource::Shift, "store_shifts"),
            (SyncResource::Employee, "store_employees"),
            (SyncResource::Tag, "store_tags"),
            (SyncResource::Attribute, "store_attributes"),
            (SyncResource::AttributeBinding, "store_attribute_bindings"),
            (SyncResource::PriceRule, "store_price_rules"),
            (SyncResource::Zone, "store_zones"),
            (SyncResource::DiningTable, "store_dining_tables"),
            (SyncResource::LabelTemplate, "store_label_templates"),
        ];
        for (resource, table) in expected {
            assert_eq!(
                deletable_table(resource),
                Some(table),
                "Missing mapping for {resource}"
            );
        }
    }

    #[test]
    fn deletable_table_returns_none_for_non_deletable() {
        // Chain resources and complex types should NOT be deletable
        assert!(deletable_table(SyncResource::ArchivedOrder).is_none());
        assert!(deletable_table(SyncResource::CreditNote).is_none());
        assert!(deletable_table(SyncResource::Invoice).is_none());
        assert!(deletable_table(SyncResource::Anulacion).is_none());
        assert!(deletable_table(SyncResource::StoreInfo).is_none());
    }
}
