//! Credit Note Repository
//!
//! CRUD operations for credit_note and credit_note_item in SQLite.

use super::{RepoError, RepoResult};
use shared::models::{CreditNote, CreditNoteDetail, CreditNoteItem};
use sqlx::SqlitePool;

/// Insert a credit note (header only). Returns the inserted row.
pub async fn create(pool: &SqlitePool, cn: &CreditNote) -> RepoResult<CreditNote> {
    let id = sqlx::query_scalar::<_, i64>(
        "INSERT INTO credit_note \
         (credit_note_number, original_order_pk, original_receipt, \
          subtotal_credit, tax_credit, total_credit, refund_method, \
          reason, note, operator_id, operator_name, \
          authorizer_id, authorizer_name, shift_id, cloud_synced, created_at) \
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16) \
         RETURNING id",
    )
    .bind(&cn.credit_note_number)
    .bind(cn.original_order_pk)
    .bind(&cn.original_receipt)
    .bind(cn.subtotal_credit)
    .bind(cn.tax_credit)
    .bind(cn.total_credit)
    .bind(&cn.refund_method)
    .bind(&cn.reason)
    .bind(&cn.note)
    .bind(cn.operator_id)
    .bind(&cn.operator_name)
    .bind(cn.authorizer_id)
    .bind(&cn.authorizer_name)
    .bind(cn.shift_id)
    .bind(cn.cloud_synced)
    .bind(cn.created_at)
    .fetch_one(pool)
    .await?;

    get_by_id(pool, id)
        .await?
        .ok_or_else(|| RepoError::Database("Failed to read credit note after insert".into()))
}

/// Insert a credit note item row.
pub async fn create_item(pool: &SqlitePool, item: &CreditNoteItem) -> RepoResult<()> {
    sqlx::query(
        "INSERT INTO credit_note_item \
         (credit_note_id, original_instance_id, item_name, quantity, \
          unit_price, line_credit, tax_rate, tax_credit) \
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
    )
    .bind(item.credit_note_id)
    .bind(&item.original_instance_id)
    .bind(&item.item_name)
    .bind(item.quantity)
    .bind(item.unit_price)
    .bind(item.line_credit)
    .bind(item.tax_rate)
    .bind(item.tax_credit)
    .execute(pool)
    .await?;
    Ok(())
}

/// Get a credit note by id.
pub async fn get_by_id(pool: &SqlitePool, id: i64) -> RepoResult<Option<CreditNote>> {
    let cn = sqlx::query_as::<_, CreditNote>(
        "SELECT id, credit_note_number, original_order_pk, original_receipt, \
         subtotal_credit, tax_credit, total_credit, refund_method, \
         reason, note, operator_id, operator_name, \
         authorizer_id, authorizer_name, shift_id, cloud_synced, created_at \
         FROM credit_note WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    Ok(cn)
}

/// Get credit note detail (with items) by id.
pub async fn get_detail(pool: &SqlitePool, id: i64) -> RepoResult<Option<CreditNoteDetail>> {
    let cn = match get_by_id(pool, id).await? {
        Some(cn) => cn,
        None => return Ok(None),
    };

    let items = sqlx::query_as::<_, CreditNoteItem>(
        "SELECT id, credit_note_id, original_instance_id, item_name, \
         quantity, unit_price, line_credit, tax_rate, tax_credit \
         FROM credit_note_item WHERE credit_note_id = ? ORDER BY id",
    )
    .bind(id)
    .fetch_all(pool)
    .await?;

    Ok(Some(CreditNoteDetail {
        credit_note: cn,
        items,
    }))
}

/// List all credit notes for a given archived order.
pub async fn list_by_order(pool: &SqlitePool, order_pk: i64) -> RepoResult<Vec<CreditNote>> {
    let rows = sqlx::query_as::<_, CreditNote>(
        "SELECT id, credit_note_number, original_order_pk, original_receipt, \
         subtotal_credit, tax_credit, total_credit, refund_method, \
         reason, note, operator_id, operator_name, \
         authorizer_id, authorizer_name, shift_id, cloud_synced, created_at \
         FROM credit_note WHERE original_order_pk = ? ORDER BY created_at DESC",
    )
    .bind(order_pk)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Get the total already refunded for an order (防超退).
pub async fn get_total_refunded(pool: &SqlitePool, order_pk: i64) -> RepoResult<f64> {
    let total: f64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(total_credit), 0.0) FROM credit_note WHERE original_order_pk = ?",
    )
    .bind(order_pk)
    .fetch_one(pool)
    .await?;
    Ok(total)
}

/// Mark a credit note as synced to cloud.
pub async fn mark_synced(pool: &SqlitePool, id: i64) -> RepoResult<()> {
    sqlx::query("UPDATE credit_note SET cloud_synced = 1 WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Mark multiple credit notes as synced to cloud.
pub async fn mark_synced_batch(pool: &SqlitePool, ids: &[i64]) -> RepoResult<()> {
    if ids.is_empty() {
        return Ok(());
    }
    let placeholders = ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    let sql = format!("UPDATE credit_note SET cloud_synced = 1 WHERE id IN ({placeholders})");
    let mut query = sqlx::query(&sql);
    for id in ids {
        query = query.bind(id);
    }
    query.execute(pool).await?;
    Ok(())
}

/// List credit note IDs not yet synced to cloud (ordered by id for chain consistency).
pub async fn list_unsynced_ids(pool: &SqlitePool, limit: i64) -> RepoResult<Vec<i64>> {
    let rows = sqlx::query_scalar::<_, i64>(
        "SELECT id FROM credit_note WHERE cloud_synced = 0 ORDER BY id LIMIT ?",
    )
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Build CreditNoteSync payload for cloud sync.
pub async fn build_sync(
    pool: &SqlitePool,
    cn_id: i64,
) -> RepoResult<shared::cloud::CreditNoteSync> {
    use shared::cloud::{CreditNoteItemSync, CreditNoteSync};

    // Join credit_note with archived_order (for order_key) and chain_entry (for hashes)
    #[derive(sqlx::FromRow)]
    struct CnRow {
        credit_note_number: String,
        original_receipt: String,
        subtotal_credit: f64,
        tax_credit: f64,
        total_credit: f64,
        refund_method: String,
        reason: String,
        note: Option<String>,
        operator_name: String,
        authorizer_name: Option<String>,
        created_at: i64,
        order_key: String,
        prev_hash: String,
        curr_hash: String,
    }

    let row = sqlx::query_as::<_, CnRow>(
        "SELECT cn.credit_note_number, cn.original_receipt, \
         cn.subtotal_credit, cn.tax_credit, cn.total_credit, \
         cn.refund_method, cn.reason, cn.note, \
         cn.operator_name, cn.authorizer_name, cn.created_at, \
         ao.order_key, \
         ce.prev_hash, ce.curr_hash \
         FROM credit_note cn \
         JOIN archived_order ao ON ao.id = cn.original_order_pk \
         JOIN chain_entry ce ON ce.entry_type = 'CREDIT_NOTE' AND ce.entry_pk = cn.id \
         WHERE cn.id = ?",
    )
    .bind(cn_id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| RepoError::Database(format!("Credit note {cn_id} not found for sync")))?;

    // Fetch items
    let items = sqlx::query_as::<_, CreditNoteItem>(
        "SELECT id, credit_note_id, original_instance_id, item_name, \
         quantity, unit_price, line_credit, tax_rate, tax_credit \
         FROM credit_note_item WHERE credit_note_id = ? ORDER BY id",
    )
    .bind(cn_id)
    .fetch_all(pool)
    .await?;

    Ok(CreditNoteSync {
        credit_note_number: row.credit_note_number,
        original_order_key: row.order_key,
        original_receipt: row.original_receipt,
        subtotal_credit: row.subtotal_credit,
        tax_credit: row.tax_credit,
        total_credit: row.total_credit,
        refund_method: row.refund_method,
        reason: row.reason,
        note: row.note,
        operator_name: row.operator_name,
        authorizer_name: row.authorizer_name,
        prev_hash: row.prev_hash,
        curr_hash: row.curr_hash,
        created_at: row.created_at,
        items: items
            .into_iter()
            .map(|i| CreditNoteItemSync {
                item_name: i.item_name,
                quantity: i.quantity,
                unit_price: i.unit_price,
                line_credit: i.line_credit,
                tax_rate: i.tax_rate,
                tax_credit: i.tax_credit,
            })
            .collect(),
    })
}
