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
