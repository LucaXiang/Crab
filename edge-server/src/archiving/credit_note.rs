//! Credit Note Service
//!
//! Creates credit notes (退款凭证) with hash chain integrity.
//! Shares the same hash chain lock as OrderArchiveService to prevent TOCTOU races.

use crate::db::repository::{credit_note as cn_repo, system_state};
use shared::models::{
    CreateCreditNoteRequest, CreditNote, CreditNoteDetail, CreditNoteItem, RefundableInfo,
};
use sqlx::SqlitePool;
use std::sync::Arc;
use tokio::sync::Mutex;

use super::service::{ArchiveError, ArchiveResult};

/// Service for creating and querying credit notes
#[derive(Clone)]
pub struct CreditNoteService {
    pool: SqlitePool,
    /// Shared with OrderArchiveService — serializes all chain_entry writes
    hash_chain_lock: Arc<Mutex<()>>,
    /// 业务时区 (用于退款凭证编号日期)
    tz: chrono_tz::Tz,
    /// Optional Verifactu invoice service (R5 invoices for credit notes)
    invoice_service: Option<super::invoice::InvoiceService>,
}

impl CreditNoteService {
    pub fn new(
        pool: SqlitePool,
        tz: chrono_tz::Tz,
        hash_chain_lock: Arc<Mutex<()>>,
        invoice_service: Option<super::invoice::InvoiceService>,
    ) -> Self {
        Self {
            pool,
            hash_chain_lock,
            tz,
            invoice_service,
        }
    }

    /// Create a credit note with hash chain integrity.
    ///
    /// 1. Validate: order exists, not over-refund, items match
    /// 2. Compute amounts from original order items
    /// 3. Insert credit_note + credit_note_item + chain_entry in one tx
    /// 4. Update system_state.last_chain_hash
    pub async fn create_credit_note(
        &self,
        request: &CreateCreditNoteRequest,
        operator_id: i64,
        operator_name: &str,
        shift_id: Option<i64>,
    ) -> ArchiveResult<CreditNoteDetail> {
        // Acquire hash chain lock (shared with OrderArchiveService)
        let _hash_lock = self.hash_chain_lock.lock().await;

        let now = shared::util::now_millis();

        // 1. Validate original order exists
        let order = sqlx::query_as::<_, ArchivedOrderRef>(
            "SELECT receipt_number, total_amount FROM archived_order WHERE id = ?",
        )
        .bind(request.original_order_pk)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ArchiveError::Database(e.to_string()))?
        .ok_or_else(|| {
            ArchiveError::Database(format!(
                "Original order not found: {}",
                request.original_order_pk
            ))
        })?;

        // 2. Fetch original order items for validation and price lookup
        let original_items: Vec<ArchivedItemRef> = sqlx::query_as::<_, ArchivedItemRef>(
            "SELECT instance_id, name, unit_price, quantity, tax_rate \
             FROM archived_order_item WHERE order_pk = ?",
        )
        .bind(request.original_order_pk)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ArchiveError::Database(e.to_string()))?;

        // 3. Anti-over-refund: check total already refunded
        let already_refunded = cn_repo::get_total_refunded(&self.pool, request.original_order_pk)
            .await
            .map_err(|e| ArchiveError::Database(e.to_string()))?;

        // 4. Build credit note items and compute amounts
        let mut cn_items: Vec<CreditNoteItem> = Vec::with_capacity(request.items.len());
        let mut subtotal_credit = 0.0_f64;
        let mut tax_credit = 0.0_f64;

        for req_item in &request.items {
            let original = original_items
                .iter()
                .find(|i| i.instance_id == req_item.instance_id)
                .ok_or_else(|| {
                    ArchiveError::Validation(format!(
                        "Item not found in original order: {}",
                        req_item.instance_id
                    ))
                })?;

            if req_item.quantity <= 0 || req_item.quantity > original.quantity as i64 {
                return Err(ArchiveError::Validation(format!(
                    "Invalid quantity {} for item {} (original: {})",
                    req_item.quantity, req_item.instance_id, original.quantity
                )));
            }

            let line_credit = original.unit_price * req_item.quantity as f64;
            let item_tax = line_credit * original.tax_rate as f64 / 10000.0;

            cn_items.push(CreditNoteItem {
                id: 0,             // will be assigned by DB
                credit_note_id: 0, // will be set after insert
                original_instance_id: req_item.instance_id.clone(),
                item_name: original.name.clone(),
                quantity: req_item.quantity,
                unit_price: original.unit_price,
                line_credit,
                tax_rate: original.tax_rate,
                tax_credit: item_tax,
            });

            subtotal_credit += line_credit;
            tax_credit += item_tax;
        }

        let total_credit = subtotal_credit + tax_credit;

        // 5. Anti-over-refund: verify total
        let remaining_refundable = order.total_amount - already_refunded;
        if total_credit > remaining_refundable + 0.001 {
            return Err(ArchiveError::Validation(format!(
                "Refund amount {:.2} exceeds remaining refundable {:.2} \
                 (original: {:.2}, already refunded: {:.2})",
                total_credit, remaining_refundable, order.total_amount, already_refunded
            )));
        }

        // 6. Generate credit note number
        let cn_number = self.generate_credit_note_number().await?;

        // 7. Get last chain hash
        let system_state = system_state::get_or_create(&self.pool)
            .await
            .map_err(|e| ArchiveError::Database(e.to_string()))?;

        let prev_hash = system_state
            .last_chain_hash
            .unwrap_or_else(|| "genesis".to_string());

        // 8. Compute chain hash
        let cn_hash = shared::order::compute_credit_note_chain_hash(
            &prev_hash,
            &cn_number,
            &order.receipt_number,
            total_credit,
            tax_credit,
        );

        // 9. Begin transaction — all writes atomic
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| ArchiveError::Database(e.to_string()))?;

        // 9a. Insert credit_note
        let cn_pk: i64 = sqlx::query_scalar::<_, i64>(
            "INSERT INTO credit_note \
             (credit_note_number, original_order_pk, original_receipt, \
              subtotal_credit, tax_credit, total_credit, refund_method, \
              reason, note, operator_id, operator_name, \
              authorizer_id, authorizer_name, shift_id, cloud_synced, created_at) \
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,0,?15) \
             RETURNING id",
        )
        .bind(&cn_number)
        .bind(request.original_order_pk)
        .bind(&order.receipt_number)
        .bind(subtotal_credit)
        .bind(tax_credit)
        .bind(total_credit)
        .bind(&request.refund_method)
        .bind(&request.reason)
        .bind(&request.note)
        .bind(operator_id)
        .bind(operator_name)
        .bind(request.authorizer_id)
        .bind(&request.authorizer_name)
        .bind(shift_id)
        .bind(now)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| ArchiveError::Database(e.to_string()))?;

        // 9b. Insert credit_note_items
        for item in &cn_items {
            sqlx::query(
                "INSERT INTO credit_note_item \
                 (credit_note_id, original_instance_id, item_name, quantity, \
                  unit_price, line_credit, tax_rate, tax_credit) \
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
            )
            .bind(cn_pk)
            .bind(&item.original_instance_id)
            .bind(&item.item_name)
            .bind(item.quantity)
            .bind(item.unit_price)
            .bind(item.line_credit)
            .bind(item.tax_rate)
            .bind(item.tax_credit)
            .execute(&mut *tx)
            .await
            .map_err(|e| ArchiveError::Database(e.to_string()))?;
        }

        // 9c. Insert chain_entry
        sqlx::query(
            "INSERT INTO chain_entry (entry_type, entry_pk, prev_hash, curr_hash, created_at) \
             VALUES ('CREDIT_NOTE', ?1, ?2, ?3, ?4)",
        )
        .bind(cn_pk)
        .bind(&prev_hash)
        .bind(&cn_hash)
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(|e| ArchiveError::Database(e.to_string()))?;

        // 9d. Update system_state.last_chain_hash
        sqlx::query("UPDATE system_state SET last_chain_hash = ?1, updated_at = ?2 WHERE id = ?3")
            .bind(&cn_hash)
            .bind(now)
            .bind(1_i64)
            .execute(&mut *tx)
            .await
            .map_err(|e| ArchiveError::Database(e.to_string()))?;

        // 9e. Create Verifactu R5 invoice
        if let Some(ref inv_svc) = self.invoice_service {
            // Build desglose from cn_items (aggregate by tax_rate using BTreeMap)
            let desglose: Vec<_> = cn_items
                .iter()
                .fold(
                    std::collections::BTreeMap::<i64, (f64, f64)>::new(),
                    |mut map, item| {
                        let entry = map.entry(item.tax_rate).or_insert((0.0, 0.0));
                        entry.0 += item.line_credit;
                        entry.1 += item.tax_credit;
                        map
                    },
                )
                .into_iter()
                .map(|(rate, (base, tax))| shared::cloud::sync::TaxDesglose {
                    tax_rate: rate as i32,
                    base_amount: rust_decimal::Decimal::try_from(base).unwrap_or_default(),
                    tax_amount: rust_decimal::Decimal::try_from(tax).unwrap_or_default(),
                })
                .collect();

            inv_svc
                .create_credit_note_invoice(
                    &mut tx,
                    cn_pk,
                    request.original_order_pk,
                    subtotal_credit,
                    tax_credit,
                    total_credit,
                    &desglose,
                )
                .await?;
        }

        tx.commit()
            .await
            .map_err(|e| ArchiveError::Database(e.to_string()))?;

        // 10. Cash refund: deduct from open shift's expected_cash
        if request.refund_method == "CASH"
            && let Err(e) =
                crate::db::repository::shift::add_cash_payment(&self.pool, -total_credit).await
        {
            tracing::warn!(
                credit_note_number = %cn_number,
                error = %e,
                "Failed to deduct cash refund from shift"
            );
        }

        tracing::info!(
            credit_note_number = %cn_number,
            original_receipt = %order.receipt_number,
            total_credit = total_credit,
            "Credit note created"
        );

        // 10. Read back the full detail
        cn_repo::get_detail(&self.pool, cn_pk)
            .await
            .map_err(|e| ArchiveError::Database(e.to_string()))?
            .ok_or_else(|| ArchiveError::Database("Failed to read credit note after insert".into()))
    }

    /// Get refundable info for an order (防超退查询)
    pub async fn get_refundable_info(&self, order_pk: i64) -> ArchiveResult<RefundableInfo> {
        let order = sqlx::query_as::<_, ArchivedOrderRef>(
            "SELECT receipt_number, total_amount FROM archived_order WHERE id = ?",
        )
        .bind(order_pk)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ArchiveError::Database(e.to_string()))?
        .ok_or_else(|| ArchiveError::Database(format!("Order not found: {}", order_pk)))?;

        let already_refunded = cn_repo::get_total_refunded(&self.pool, order_pk)
            .await
            .map_err(|e| ArchiveError::Database(e.to_string()))?;

        Ok(RefundableInfo {
            original_order_pk: order_pk,
            original_receipt: order.receipt_number,
            original_total: order.total_amount,
            already_refunded,
            remaining_refundable: order.total_amount - already_refunded,
        })
    }

    /// Get credit note detail by id
    pub async fn get_detail(&self, id: i64) -> ArchiveResult<Option<CreditNoteDetail>> {
        cn_repo::get_detail(&self.pool, id)
            .await
            .map_err(|e| ArchiveError::Database(e.to_string()))
    }

    /// List credit notes for an order
    pub async fn list_by_order(&self, order_pk: i64) -> ArchiveResult<Vec<CreditNote>> {
        cn_repo::list_by_order(&self.pool, order_pk)
            .await
            .map_err(|e| ArchiveError::Database(e.to_string()))
    }

    /// Generate credit note number: CN-YYYYMMDD-NNNN
    async fn generate_credit_note_number(&self) -> ArchiveResult<String> {
        let now = chrono::Utc::now().with_timezone(&self.tz);
        let date_str = now.format("%Y%m%d").to_string();

        // Count existing credit notes for today
        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM credit_note WHERE credit_note_number LIKE ?")
                .bind(format!("CN-{}-%%", date_str))
                .fetch_one(&self.pool)
                .await
                .map_err(|e| ArchiveError::Database(e.to_string()))?;

        Ok(format!("CN-{}-{:04}", date_str, count + 1))
    }
}

// ============================================================================
// Internal query helper types
// ============================================================================

#[derive(Debug, sqlx::FromRow)]
struct ArchivedOrderRef {
    receipt_number: String,
    total_amount: f64,
}

#[derive(Debug, sqlx::FromRow)]
struct ArchivedItemRef {
    instance_id: String,
    name: String,
    unit_price: f64,
    quantity: i32,
    tax_rate: i64,
}
