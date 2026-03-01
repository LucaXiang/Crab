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

/// Upsert credit note — header row + child items table.
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

    // Upsert header row (RETURNING id for child inserts).
    // fetch_optional: if version is older than existing, PG skips the update and RETURNING
    // returns no rows — we simply skip the sync item.
    let mut tx = pool.begin().await?;

    let row: Option<(i64,)> = sqlx::query_as(
        r#"
        INSERT INTO store_credit_notes (
            store_id, tenant_id, source_id, credit_note_number,
            original_order_id, original_receipt,
            subtotal_credit, tax_credit, total_credit,
            refund_method, reason, note,
            operator_name, authorizer_name,
            prev_hash, curr_hash, created_at,
            version, synced_at
        )
        VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19)
        ON CONFLICT (tenant_id, store_id, source_id)
        DO UPDATE SET credit_note_number = EXCLUDED.credit_note_number,
                      total_credit = EXCLUDED.total_credit,
                      prev_hash = EXCLUDED.prev_hash,
                      curr_hash = EXCLUDED.curr_hash,
                      version = EXCLUDED.version,
                      synced_at = EXCLUDED.synced_at
        WHERE store_credit_notes.version <= EXCLUDED.version
        RETURNING id
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
    .bind(version_to_i64(item.version))
    .bind(now)
    .fetch_optional(&mut *tx)
    .await?;

    let Some((cn_id,)) = row else {
        // Version already newer — skip this item
        tx.commit().await?;
        return Ok(());
    };

    // Replace child items (delete + re-insert) within same transaction
    sqlx::query("DELETE FROM store_credit_note_items WHERE credit_note_id = $1")
        .bind(cn_id)
        .execute(&mut *tx)
        .await?;

    if !cn.items.is_empty() {
        let cn_ids: Vec<i64> = cn.items.iter().map(|_| cn_id).collect();
        let instance_ids: Vec<&str> = cn
            .items
            .iter()
            .map(|i| i.original_instance_id.as_str())
            .collect();
        let names: Vec<&str> = cn.items.iter().map(|i| i.item_name.as_str()).collect();
        let quantities: Vec<i64> = cn.items.iter().map(|i| i.quantity).collect();
        let unit_prices: Vec<f64> = cn.items.iter().map(|i| i.unit_price).collect();
        let line_credits: Vec<f64> = cn.items.iter().map(|i| i.line_credit).collect();
        let tax_rates: Vec<i64> = cn.items.iter().map(|i| i.tax_rate).collect();
        let tax_credits: Vec<f64> = cn.items.iter().map(|i| i.tax_credit).collect();

        sqlx::query(
            r#"
            INSERT INTO store_credit_note_items (
                credit_note_id, original_instance_id, item_name, quantity, unit_price, line_credit, tax_rate, tax_credit
            )
            SELECT * FROM UNNEST($1::bigint[], $2::text[], $3::text[], $4::bigint[], $5::float8[], $6::float8[], $7::bigint[], $8::float8[])
            "#,
        )
        .bind(&cn_ids)
        .bind(&instance_ids)
        .bind(&names)
        .bind(&quantities)
        .bind(&unit_prices)
        .bind(&line_credits)
        .bind(&tax_rates)
        .bind(&tax_credits)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    Ok(())
}

/// Upsert archived order — header row + child tables (items, options, payments, events, desglose).
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

    let d = &detail_sync.detail;

    // All writes (header + children) in a single transaction for atomicity.
    let mut tx = pool.begin().await?;

    // Upsert header row.
    // fetch_optional: if version is older than existing, PG skips the update and RETURNING
    // returns no rows — we simply skip this sync item.
    let row: Option<(i64,)> = sqlx::query_as(
        r#"
        INSERT INTO store_archived_orders (
            store_id, tenant_id, source_id, order_id,
            receipt_number, status, end_time, total, tax,
            prev_hash, curr_hash, last_event_hash,
            guest_count, discount_amount, void_type, loss_amount, start_time,
            zone_name, table_name, is_retail, original_total, subtotal,
            paid_amount, surcharge_amount, comp_total_amount,
            order_manual_discount_amount, order_manual_surcharge_amount,
            order_rule_discount_amount, order_rule_surcharge_amount,
            operator_name, loss_reason, void_note, member_name, service_type,
            operator_id, member_id, queue_number, shift_id, created_at,
            version, synced_at
        )
        VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20,$21,$22,$23,$24,$25,$26,$27,$28,$29,$30,$31,$32,$33,$34,$35,$36,$37,$38,$39,$40,$41)
        ON CONFLICT (tenant_id, store_id, order_id)
        DO UPDATE SET receipt_number = EXCLUDED.receipt_number,
                      status = EXCLUDED.status,
                      end_time = EXCLUDED.end_time,
                      total = EXCLUDED.total,
                      tax = EXCLUDED.tax,
                      prev_hash = EXCLUDED.prev_hash,
                      curr_hash = EXCLUDED.curr_hash,
                      last_event_hash = EXCLUDED.last_event_hash,
                      guest_count = EXCLUDED.guest_count,
                      discount_amount = EXCLUDED.discount_amount,
                      void_type = EXCLUDED.void_type,
                      loss_amount = EXCLUDED.loss_amount,
                      start_time = EXCLUDED.start_time,
                      zone_name = EXCLUDED.zone_name,
                      table_name = EXCLUDED.table_name,
                      is_retail = EXCLUDED.is_retail,
                      original_total = EXCLUDED.original_total,
                      subtotal = EXCLUDED.subtotal,
                      paid_amount = EXCLUDED.paid_amount,
                      surcharge_amount = EXCLUDED.surcharge_amount,
                      comp_total_amount = EXCLUDED.comp_total_amount,
                      order_manual_discount_amount = EXCLUDED.order_manual_discount_amount,
                      order_manual_surcharge_amount = EXCLUDED.order_manual_surcharge_amount,
                      order_rule_discount_amount = EXCLUDED.order_rule_discount_amount,
                      order_rule_surcharge_amount = EXCLUDED.order_rule_surcharge_amount,
                      operator_name = EXCLUDED.operator_name,
                      loss_reason = EXCLUDED.loss_reason,
                      void_note = EXCLUDED.void_note,
                      member_name = EXCLUDED.member_name,
                      service_type = EXCLUDED.service_type,
                      operator_id = EXCLUDED.operator_id,
                      member_id = EXCLUDED.member_id,
                      queue_number = EXCLUDED.queue_number,
                      shift_id = EXCLUDED.shift_id,
                      version = EXCLUDED.version,
                      synced_at = EXCLUDED.synced_at
        WHERE store_archived_orders.version <= EXCLUDED.version
        RETURNING id
        "#,
    )
    .bind(store_id)                          // $1
    .bind(tenant_id)                         // $2
    .bind(item.resource_id)                  // $3  source_id
    .bind(detail_sync.order_id)              // $4
    .bind(&detail_sync.receipt_number)       // $5
    .bind(&detail_sync.status)               // $6
    .bind(detail_sync.end_time)              // $7
    .bind(detail_sync.total_amount)          // $8
    .bind(detail_sync.tax)                   // $9
    .bind(&detail_sync.prev_hash)            // $10
    .bind(&detail_sync.curr_hash)            // $11
    .bind(&detail_sync.last_event_hash)      // $12
    .bind(d.guest_count)                     // $13
    .bind(d.discount_amount)                 // $14
    .bind(d.void_type.as_ref().map(|v| v.as_str())) // $15
    .bind(d.loss_amount)                     // $16
    .bind(d.start_time)                      // $17
    .bind(&d.zone_name)                      // $18
    .bind(&d.table_name)                     // $19
    .bind(d.is_retail)                       // $20
    .bind(d.original_total)                  // $21
    .bind(d.subtotal)                        // $22
    .bind(d.paid_amount)                     // $23
    .bind(d.surcharge_amount)                // $24
    .bind(d.comp_total_amount)               // $25
    .bind(d.order_manual_discount_amount)    // $26
    .bind(d.order_manual_surcharge_amount)   // $27
    .bind(d.order_rule_discount_amount)      // $28
    .bind(d.order_rule_surcharge_amount)     // $29
    .bind(&d.operator_name)                  // $30
    .bind(d.loss_reason.as_ref().map(|v| v.as_str())) // $31
    .bind(&d.void_note)                      // $32
    .bind(&d.member_name)                    // $33
    .bind(d.service_type.as_ref().map(|v| v.as_str())) // $34
    .bind(d.operator_id)                     // $35
    .bind(d.member_id)                       // $36
    .bind(&d.queue_number)                   // $37
    .bind(d.shift_id)                        // $38
    .bind(detail_sync.created_at)            // $39
    .bind(version_to_i64(item.version))      // $40
    .bind(now)                               // $41
    .fetch_optional(&mut *tx)
    .await?;

    let Some((order_pk,)) = row else {
        // Version already newer — skip this item
        tx.commit().await?;
        return Ok(());
    };

    // ── Replace all child tables (delete + batch re-insert) ──

    // Delete existing children (CASCADE would also work but explicit is clearer)
    sqlx::query("DELETE FROM store_order_items WHERE order_id = $1")
        .bind(order_pk)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM store_order_payments WHERE order_id = $1")
        .bind(order_pk)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM store_order_events WHERE order_id = $1")
        .bind(order_pk)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM store_order_desglose WHERE order_id = $1")
        .bind(order_pk)
        .execute(&mut *tx)
        .await?;

    // ── Items + Options ──
    if !d.items.is_empty() {
        let oids: Vec<i64> = d.items.iter().map(|_| order_pk).collect();
        let names: Vec<&str> = d.items.iter().map(|i| i.name.as_str()).collect();
        let spec_names: Vec<Option<&str>> =
            d.items.iter().map(|i| i.spec_name.as_deref()).collect();
        let cat_names: Vec<Option<&str>> =
            d.items.iter().map(|i| i.category_name.as_deref()).collect();
        let prod_ids: Vec<Option<i64>> = d.items.iter().map(|i| i.product_source_id).collect();
        let prices: Vec<f64> = d.items.iter().map(|i| i.price).collect();
        let quantities: Vec<i32> = d.items.iter().map(|i| i.quantity).collect();
        let unit_prices: Vec<f64> = d.items.iter().map(|i| i.unit_price).collect();
        let line_totals: Vec<f64> = d.items.iter().map(|i| i.line_total).collect();
        let discounts: Vec<f64> = d.items.iter().map(|i| i.discount_amount).collect();
        let surcharges: Vec<f64> = d.items.iter().map(|i| i.surcharge_amount).collect();
        let taxes: Vec<f64> = d.items.iter().map(|i| i.tax).collect();
        let tax_rates: Vec<i32> = d.items.iter().map(|i| i.tax_rate).collect();
        let comped: Vec<bool> = d.items.iter().map(|i| i.is_comped).collect();
        let notes: Vec<Option<&str>> = d.items.iter().map(|i| i.note.as_deref()).collect();

        let item_rows: Vec<(i64,)> = sqlx::query_as(
            r#"
            INSERT INTO store_order_items (
                order_id, name, spec_name, category_name, product_source_id,
                price, quantity, unit_price, line_total,
                discount_amount, surcharge_amount, tax, tax_rate, is_comped, note
            )
            SELECT * FROM UNNEST(
                $1::bigint[], $2::text[], $3::text[], $4::text[], $5::bigint[],
                $6::float8[], $7::int[], $8::float8[], $9::float8[],
                $10::float8[], $11::float8[], $12::float8[], $13::int[], $14::bool[], $15::text[]
            )
            RETURNING id
            "#,
        )
        .bind(&oids)
        .bind(&names)
        .bind(&spec_names)
        .bind(&cat_names)
        .bind(&prod_ids)
        .bind(&prices)
        .bind(&quantities)
        .bind(&unit_prices)
        .bind(&line_totals)
        .bind(&discounts)
        .bind(&surcharges)
        .bind(&taxes)
        .bind(&tax_rates)
        .bind(&comped)
        .bind(&notes)
        .fetch_all(&mut *tx)
        .await?;

        // Insert item options (batch across all items)
        let mut opt_item_ids: Vec<i64> = Vec::new();
        let mut opt_attr_names: Vec<String> = Vec::new();
        let mut opt_option_names: Vec<String> = Vec::new();
        let mut opt_prices: Vec<f64> = Vec::new();
        let mut opt_quantities: Vec<i32> = Vec::new();

        for (idx, sync_item) in d.items.iter().enumerate() {
            let item_pk = item_rows[idx].0;
            for opt in &sync_item.options {
                opt_item_ids.push(item_pk);
                opt_attr_names.push(opt.attribute_name.clone());
                opt_option_names.push(opt.option_name.clone());
                opt_prices.push(opt.price);
                opt_quantities.push(opt.quantity);
            }
        }

        if !opt_item_ids.is_empty() {
            sqlx::query(
                r#"
                INSERT INTO store_order_item_options (item_id, attribute_name, option_name, price, quantity)
                SELECT * FROM UNNEST($1::bigint[], $2::text[], $3::text[], $4::float8[], $5::int[])
                "#,
            )
            .bind(&opt_item_ids)
            .bind(&opt_attr_names)
            .bind(&opt_option_names)
            .bind(&opt_prices)
            .bind(&opt_quantities)
            .execute(&mut *tx)
            .await?;
        }
    }

    // ── Payments ──
    if !d.payments.is_empty() {
        let oids: Vec<i64> = d.payments.iter().map(|_| order_pk).collect();
        let seqs: Vec<i32> = d.payments.iter().map(|p| p.seq).collect();
        let methods: Vec<&str> = d.payments.iter().map(|p| p.method.as_str()).collect();
        let amounts: Vec<f64> = d.payments.iter().map(|p| p.amount).collect();
        let timestamps: Vec<i64> = d.payments.iter().map(|p| p.timestamp).collect();
        let cancelled: Vec<bool> = d.payments.iter().map(|p| p.cancelled).collect();
        let cancel_reasons: Vec<Option<&str>> = d
            .payments
            .iter()
            .map(|p| p.cancel_reason.as_deref())
            .collect();
        let tendereds: Vec<Option<f64>> = d.payments.iter().map(|p| p.tendered).collect();
        let change_amounts: Vec<Option<f64>> = d.payments.iter().map(|p| p.change_amount).collect();

        sqlx::query(
            r#"
            INSERT INTO store_order_payments (order_id, seq, method, amount, timestamp, cancelled, cancel_reason, tendered, change_amount)
            SELECT * FROM UNNEST($1::bigint[], $2::int[], $3::text[], $4::float8[], $5::bigint[], $6::bool[], $7::text[], $8::float8[], $9::float8[])
            "#,
        )
        .bind(&oids)
        .bind(&seqs)
        .bind(&methods)
        .bind(&amounts)
        .bind(&timestamps)
        .bind(&cancelled)
        .bind(&cancel_reasons)
        .bind(&tendereds)
        .bind(&change_amounts)
        .execute(&mut *tx)
        .await?;
    }

    // ── Events ──
    if !d.events.is_empty() {
        let oids: Vec<i64> = d.events.iter().map(|_| order_pk).collect();
        let seqs: Vec<i32> = d.events.iter().map(|e| e.seq).collect();
        let types: Vec<&str> = d.events.iter().map(|e| e.event_type.as_str()).collect();
        let timestamps: Vec<i64> = d.events.iter().map(|e| e.timestamp).collect();
        let op_ids: Vec<Option<i64>> = d.events.iter().map(|e| e.operator_id).collect();
        let op_names: Vec<Option<&str>> = d
            .events
            .iter()
            .map(|e| e.operator_name.as_deref())
            .collect();
        let data: Vec<Option<&str>> = d.events.iter().map(|e| e.data.as_deref()).collect();

        sqlx::query(
            r#"
            INSERT INTO store_order_events (order_id, seq, event_type, timestamp, operator_id, operator_name, data)
            SELECT * FROM UNNEST($1::bigint[], $2::int[], $3::text[], $4::bigint[], $5::bigint[], $6::text[], $7::text[])
            "#,
        )
        .bind(&oids)
        .bind(&seqs)
        .bind(&types)
        .bind(&timestamps)
        .bind(&op_ids)
        .bind(&op_names)
        .bind(&data)
        .execute(&mut *tx)
        .await?;
    }

    // ── Desglose ──
    if !detail_sync.desglose.is_empty() {
        let oids: Vec<i64> = detail_sync.desglose.iter().map(|_| order_pk).collect();
        let rates: Vec<i32> = detail_sync.desglose.iter().map(|d| d.tax_rate).collect();
        let bases: Vec<rust_decimal::Decimal> =
            detail_sync.desglose.iter().map(|d| d.base_amount).collect();
        let tax_amts: Vec<rust_decimal::Decimal> =
            detail_sync.desglose.iter().map(|d| d.tax_amount).collect();

        sqlx::query(
            r#"
            INSERT INTO store_order_desglose (order_id, tax_rate, base_amount, tax_amount)
            SELECT * FROM UNNEST($1::bigint[], $2::int[], $3::numeric[], $4::numeric[])
            ON CONFLICT (order_id, tax_rate) DO UPDATE SET
                base_amount = EXCLUDED.base_amount,
                tax_amount = EXCLUDED.tax_amount
            "#,
        )
        .bind(&oids)
        .bind(&rates)
        .bind(&bases)
        .bind(&tax_amts)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
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

    sqlx::query(
        r#"
        INSERT INTO store_anulaciones (
            store_id, tenant_id, source_id, anulacion_number, serie,
            original_invoice_id, original_invoice_number, original_order_id,
            huella, prev_huella, fecha_expedicion, fecha_hora_registro,
            nif, nombre_razon, reason, note, operator_id, operator_name,
            prev_hash, curr_hash,
            aeat_status, version, created_at, synced_at
        )
        VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20,$21,$22,$23,$24)
        ON CONFLICT (tenant_id, store_id, source_id)
        DO UPDATE SET anulacion_number = EXCLUDED.anulacion_number,
                      huella = EXCLUDED.huella,
                      prev_huella = EXCLUDED.prev_huella,
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
    .bind(anu.reason.as_str())
    .bind(&anu.note)
    .bind(anu.operator_id)
    .bind(&anu.operator_name)
    .bind(&anu.prev_hash)
    .bind(&anu.curr_hash)
    .bind("PENDING")
    .bind(version_to_i64(item.version))
    .bind(anu.created_at)
    .bind(now)
    .execute(pool)
    .await?;

    Ok(())
}

/// Upsert Verifactu invoice — header row + desglose child table.
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

    let mut tx = pool.begin().await?;

    let row: Option<(i64,)> = sqlx::query_as(
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
            version, synced_at
        )
        VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20,$21,$22,$23,$24,$25,$26,$27,$28,$29)
        ON CONFLICT (tenant_id, store_id, source_id)
        DO UPDATE SET invoice_number = EXCLUDED.invoice_number,
                      total = EXCLUDED.total,
                      huella = EXCLUDED.huella,
                      prev_huella = EXCLUDED.prev_huella,
                      version = EXCLUDED.version,
                      synced_at = EXCLUDED.synced_at
        WHERE store_invoices.version <= EXCLUDED.version
        RETURNING id
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
    .bind(version_to_i64(item.version))
    .bind(now)
    .fetch_optional(&mut *tx)
    .await?;

    let Some((invoice_id,)) = row else {
        tx.commit().await?;
        return Ok(());
    };

    // Replace desglose child rows within same transaction
    sqlx::query("DELETE FROM store_invoice_desglose WHERE invoice_id = $1")
        .bind(invoice_id)
        .execute(&mut *tx)
        .await?;

    if !inv.desglose.is_empty() {
        let inv_ids: Vec<i64> = inv.desglose.iter().map(|_| invoice_id).collect();
        let rates: Vec<i32> = inv.desglose.iter().map(|d| d.tax_rate).collect();
        let bases: Vec<rust_decimal::Decimal> =
            inv.desglose.iter().map(|d| d.base_amount).collect();
        let tax_amts: Vec<rust_decimal::Decimal> =
            inv.desglose.iter().map(|d| d.tax_amount).collect();

        sqlx::query(
            r#"
            INSERT INTO store_invoice_desglose (invoice_id, tax_rate, base_amount, tax_amount)
            SELECT * FROM UNNEST($1::bigint[], $2::int[], $3::numeric[], $4::numeric[])
            "#,
        )
        .bind(&inv_ids)
        .bind(&rates)
        .bind(&bases)
        .bind(&tax_amts)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
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
