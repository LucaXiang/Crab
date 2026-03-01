//! Credit Note Service
//!
//! Creates credit notes (退款凭证) with hash chain integrity.
//! Shares the same hash chain lock as OrderArchiveService to prevent TOCTOU races.

use crate::db::repository::{credit_note as cn_repo, system_state};
use crate::orders::OrdersManager;
use shared::models::{
    CreateCreditNoteRequest, CreditNote, CreditNoteDetail, CreditNoteItem, RefundableInfo,
};
use shared::util::snowflake_id;
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
    /// OrdersManager for shared chain number generation
    orders_manager: Arc<OrdersManager>,
    /// Optional Verifactu invoice service (R5 invoices for credit notes)
    invoice_service: Option<super::invoice::InvoiceService>,
}

impl CreditNoteService {
    pub fn new(
        pool: SqlitePool,
        hash_chain_lock: Arc<Mutex<()>>,
        orders_manager: Arc<OrdersManager>,
        invoice_service: Option<super::invoice::InvoiceService>,
    ) -> Self {
        Self {
            pool,
            hash_chain_lock,
            orders_manager,
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

        // 3b. Per-item already refunded quantities
        let refunded_items = cn_repo::get_refunded_items(&self.pool, request.original_order_pk)
            .await
            .map_err(|e| ArchiveError::Database(e.to_string()))?;

        // 4. Build credit note items and compute amounts
        //
        // Tax calculation: Spain IVA prices are tax-inclusive.
        // line_credit = unit_price × quantity (含税总额 = 客户实际支付金额)
        // item_tax = line_credit × tax_rate / (10000 + tax_rate) (从含税价提取税)
        // item_subtotal = line_credit - item_tax (税前金额)
        // total_credit = Σ line_credit (= subtotal_credit + tax_credit)
        use rust_decimal::prelude::*;
        let mut cn_items: Vec<CreditNoteItem> = Vec::with_capacity(request.items.len());
        let mut dec_subtotal = rust_decimal::Decimal::ZERO;
        let mut dec_tax = rust_decimal::Decimal::ZERO;
        let hundred = rust_decimal::Decimal::ONE_HUNDRED;

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

            let already_refunded_qty = refunded_items
                .iter()
                .find(|ri| ri.instance_id == req_item.instance_id)
                .map(|ri| ri.refunded_quantity)
                .unwrap_or(0);
            let remaining_qty = original.quantity as i64 - already_refunded_qty;

            if req_item.quantity <= 0 || req_item.quantity > remaining_qty {
                return Err(ArchiveError::Validation(format!(
                    "Invalid quantity {} for item {} (original: {}, already refunded: {}, remaining: {})",
                    req_item.quantity,
                    req_item.instance_id,
                    original.quantity,
                    already_refunded_qty,
                    remaining_qty
                )));
            }

            let dec_unit_price = rust_decimal::Decimal::try_from(original.unit_price)
                .map_err(|e| ArchiveError::Validation(format!("unit_price f64→Decimal: {e}")))?;
            let dec_qty = rust_decimal::Decimal::from(req_item.quantity);
            let dec_line = dec_unit_price * dec_qty;

            // Extract tax from tax-inclusive price: tax = gross * rate / (100 + rate)
            // tax_rate is integer percentage (e.g., 10 = 10% IVA)
            let tax_rate_dec = rust_decimal::Decimal::from(original.tax_rate);
            let item_tax = if tax_rate_dec > rust_decimal::Decimal::ZERO {
                dec_line * tax_rate_dec / (hundred + tax_rate_dec)
            } else {
                rust_decimal::Decimal::ZERO
            };
            let item_subtotal = dec_line - item_tax;

            let line_credit_f64 = dec_line.to_f64().unwrap_or(0.0);
            let item_tax_f64 = item_tax.to_f64().unwrap_or(0.0);

            cn_items.push(CreditNoteItem {
                id: 0,             // will be assigned by DB
                credit_note_id: 0, // will be set after insert
                original_instance_id: req_item.instance_id.clone(),
                item_name: original.name.clone(),
                quantity: req_item.quantity,
                unit_price: original.unit_price,
                line_credit: line_credit_f64,
                tax_rate: original.tax_rate,
                tax_credit: item_tax_f64,
            });

            dec_subtotal += item_subtotal;
            dec_tax += item_tax;
        }

        let dec_total = dec_subtotal + dec_tax;

        // 5. Anti-over-refund: verify total (Decimal precision)
        let dec_order_total = rust_decimal::Decimal::try_from(order.total_amount)
            .map_err(|e| ArchiveError::Validation(format!("total_amount f64→Decimal: {e}")))?;
        let dec_already_refunded = rust_decimal::Decimal::try_from(already_refunded)
            .map_err(|e| ArchiveError::Validation(format!("already_refunded f64→Decimal: {e}")))?;
        let dec_remaining = dec_order_total - dec_already_refunded;
        if dec_total > dec_remaining {
            return Err(ArchiveError::Validation(format!(
                "Refund amount {:.2} exceeds remaining refundable {:.2} \
                 (original: {:.2}, already refunded: {:.2})",
                dec_total, dec_remaining, dec_order_total, dec_already_refunded
            )));
        }

        let subtotal_credit = dec_subtotal.to_f64().unwrap_or(0.0);
        let tax_credit = dec_tax.to_f64().unwrap_or(0.0);
        let total_credit = dec_total.to_f64().unwrap_or(0.0);

        // 6. Generate credit note number
        let cn_number = self.generate_credit_note_number()?;

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
            now,
            operator_name,
            &request.refund_method,
        );

        // 9. Begin transaction — all writes atomic
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| ArchiveError::Database(e.to_string()))?;

        // 9a. Insert credit_note
        let cn_pk = snowflake_id();
        sqlx::query(
            "INSERT INTO credit_note \
             (id, credit_note_number, original_order_pk, original_receipt, \
              subtotal_credit, tax_credit, total_credit, refund_method, \
              reason, note, operator_id, operator_name, \
              authorizer_id, authorizer_name, shift_id, cloud_synced, created_at) \
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,0,?16)",
        )
        .bind(cn_pk)
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
        .execute(&mut *tx)
        .await
        .map_err(|e| ArchiveError::Database(e.to_string()))?;

        // 9b. Insert credit_note_items
        for item in &cn_items {
            let cn_item_id = snowflake_id();
            sqlx::query(
                "INSERT INTO credit_note_item \
                 (id, credit_note_id, original_instance_id, item_name, quantity, \
                  unit_price, line_credit, tax_rate, tax_credit) \
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
            )
            .bind(cn_item_id)
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
        let chain_entry_id = snowflake_id();
        sqlx::query(
            "INSERT INTO chain_entry (id, entry_type, entry_pk, prev_hash, curr_hash, created_at) \
             VALUES (?1, 'CREDIT_NOTE', ?2, ?3, ?4, ?5)",
        )
        .bind(chain_entry_id)
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
                        // base_amount = line_credit - tax_credit (net/tax-exclusive)
                        entry.0 += item.line_credit - item.tax_credit;
                        entry.1 += item.tax_credit;
                        map
                    },
                )
                .into_iter()
                .map(|(rate, (base, tax))| {
                    Ok(shared::cloud::sync::TaxDesglose {
                        tax_rate: rate as i32,
                        base_amount: rust_decimal::Decimal::try_from(base).map_err(|e| {
                            ArchiveError::InvoiceConversion(format!(
                                "cn desglose base f64→Decimal: {e}"
                            ))
                        })?,
                        tax_amount: rust_decimal::Decimal::try_from(tax).map_err(|e| {
                            ArchiveError::InvoiceConversion(format!(
                                "cn desglose tax f64→Decimal: {e}"
                            ))
                        })?,
                    })
                })
                .collect::<ArchiveResult<Vec<_>>>()?;

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

        let refunded_items = cn_repo::get_refunded_items(&self.pool, order_pk)
            .await
            .map_err(|e| ArchiveError::Database(e.to_string()))?;

        Ok(RefundableInfo {
            original_order_pk: order_pk,
            original_receipt: order.receipt_number,
            original_total: order.total_amount,
            already_refunded,
            remaining_refundable: order.total_amount - already_refunded,
            refunded_items,
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

    /// Generate credit note number using the shared chain counter (same as receipt_number).
    ///
    /// Returns the same format as receipt_number: `{store_number:02}-{YYYYMMDD}-{daily_seq:04}`
    fn generate_credit_note_number(&self) -> ArchiveResult<String> {
        Ok(self.orders_manager.next_chain_number())
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

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use rust_decimal::prelude::*;

    /// Helper: extract tax from tax-inclusive price using Spain IVA formula.
    /// Mirrors the logic in `create_credit_note()` lines 141-148.
    fn extract_tax(gross: rust_decimal::Decimal, tax_rate: i64) -> rust_decimal::Decimal {
        let hundred = rust_decimal::Decimal::ONE_HUNDRED;
        let rate = rust_decimal::Decimal::from(tax_rate);
        if rate > rust_decimal::Decimal::ZERO {
            gross * rate / (hundred + rate)
        } else {
            rust_decimal::Decimal::ZERO
        }
    }

    // -----------------------------------------------------------------------
    // Task #47: Tax extraction formula tests
    // -----------------------------------------------------------------------

    #[test]
    fn tax_extraction_10_percent() {
        // 110€ gross at 10% IVA → tax = 110 * 10 / 110 = 10€
        let gross = rust_decimal::Decimal::from(110);
        let tax = extract_tax(gross, 10);
        assert_eq!(tax, rust_decimal::Decimal::from(10));
    }

    #[test]
    fn tax_extraction_21_percent() {
        // 121€ gross at 21% IVA → tax = 121 * 21 / 121 = 21€
        let gross = rust_decimal::Decimal::from(121);
        let tax = extract_tax(gross, 21);
        assert_eq!(tax, rust_decimal::Decimal::from(21));
    }

    #[test]
    fn tax_extraction_0_percent() {
        let gross = rust_decimal::Decimal::from(100);
        let tax = extract_tax(gross, 0);
        assert_eq!(tax, rust_decimal::Decimal::ZERO);
    }

    #[test]
    fn tax_extraction_subtotal_plus_tax_equals_gross() {
        // For any gross amount, subtotal + tax must equal gross (conservation)
        let gross = rust_decimal::Decimal::new(1595, 2); // 15.95€
        let tax = extract_tax(gross, 10);
        let subtotal = gross - tax;
        assert_eq!(subtotal + tax, gross);
    }

    #[test]
    fn tax_extraction_multiple_items_precision() {
        // 3 items × 3.30€ at 10% IVA
        let unit_price = rust_decimal::Decimal::new(330, 2); // 3.30€
        let qty = rust_decimal::Decimal::from(3);
        let gross = unit_price * qty; // 9.90€
        let tax = extract_tax(gross, 10);
        let subtotal = gross - tax;

        // tax = 9.90 * 10 / 110 = 0.9 exactly
        assert_eq!(tax, rust_decimal::Decimal::new(9, 1));
        assert_eq!(subtotal, rust_decimal::Decimal::new(9, 0));
        assert_eq!(subtotal + tax, gross);
    }

    #[test]
    fn tax_extraction_non_round_result() {
        // 10€ at 21% → tax = 10 * 21 / 121 = 210/121 ≈ 1.735537...
        // Decimal division should give exact rational result (not truncated)
        let gross = rust_decimal::Decimal::from(10);
        let tax = extract_tax(gross, 21);
        let subtotal = gross - tax;

        // Key invariant: subtotal + tax == gross
        assert_eq!(subtotal + tax, gross);

        // tax should be approximately 1.7355...
        let lower = rust_decimal::Decimal::new(173, 2); // 1.73
        let upper = rust_decimal::Decimal::new(174, 2); // 1.74
        assert!(tax > lower && tax < upper, "tax={tax} not in (1.73, 1.74)");
    }

    #[test]
    fn tax_extraction_round_trip_f64() {
        // Simulate the actual code path: Decimal tax → f64 → reconstruct
        let unit_price_f64: f64 = 12.50;
        let qty: i64 = 2;
        let tax_rate: i64 = 10;

        let dec_unit = rust_decimal::Decimal::try_from(unit_price_f64).unwrap();
        let dec_qty = rust_decimal::Decimal::from(qty);
        let gross = dec_unit * dec_qty; // 25.00

        let tax = extract_tax(gross, tax_rate);
        let subtotal = gross - tax;

        // Convert to f64 as the code does
        let tax_f64 = tax.to_f64().unwrap();
        let subtotal_f64 = subtotal.to_f64().unwrap();
        let total_f64 = gross.to_f64().unwrap();

        // f64 round-trip: subtotal + tax ≈ total (within floating point)
        let reconstructed = subtotal_f64 + tax_f64;
        assert!(
            (reconstructed - total_f64).abs() < 1e-10,
            "f64 round-trip failed: {subtotal_f64} + {tax_f64} = {reconstructed} != {total_f64}"
        );
    }

    // -----------------------------------------------------------------------
    // Task #48: Decimal anti-over-refund tests
    // -----------------------------------------------------------------------

    /// Helper: simulate the anti-over-refund check from create_credit_note().
    /// Returns Ok(()) if refund is within limits, Err(msg) if over-refund.
    fn check_over_refund(
        order_total: f64,
        already_refunded: f64,
        new_refund: rust_decimal::Decimal,
    ) -> Result<(), String> {
        let dec_order_total = rust_decimal::Decimal::try_from(order_total)
            .map_err(|e| format!("order_total: {e}"))?;
        let dec_already = rust_decimal::Decimal::try_from(already_refunded)
            .map_err(|e| format!("already_refunded: {e}"))?;
        let dec_remaining = dec_order_total - dec_already;
        if new_refund > dec_remaining {
            Err(format!("over-refund: {new_refund} > {dec_remaining}"))
        } else {
            Ok(())
        }
    }

    #[test]
    fn anti_overrefund_exact_match_passes() {
        // Refund exactly the remaining amount — should pass
        let order_total = 100.0;
        let already_refunded = 0.0;
        let new_refund = rust_decimal::Decimal::from(100);
        assert!(check_over_refund(order_total, already_refunded, new_refund).is_ok());
    }

    #[test]
    fn anti_overrefund_one_cent_over_rejected() {
        // Refund 1 cent more than remaining — must reject
        let order_total = 100.0;
        let already_refunded = 0.0;
        let new_refund = rust_decimal::Decimal::new(10001, 2); // 100.01
        assert!(check_over_refund(order_total, already_refunded, new_refund).is_err());
    }

    #[test]
    fn anti_overrefund_cumulative_refunds() {
        // First refund: 60€ of 100€ → OK
        let order_total = 100.0;
        assert!(check_over_refund(order_total, 0.0, rust_decimal::Decimal::from(60)).is_ok());

        // Second refund: 40€ of remaining 40€ → OK (exact)
        assert!(check_over_refund(order_total, 60.0, rust_decimal::Decimal::from(40)).is_ok());

        // Third refund: anything > 0 → rejected
        assert!(check_over_refund(order_total, 100.0, rust_decimal::Decimal::new(1, 2)).is_err());
    }

    #[test]
    fn anti_overrefund_f64_precision_edge_case() {
        // Classic f64 problem: 0.1 + 0.2 != 0.3
        // Order total = 10.30, first refund = 7.20, second refund = 3.10
        // With f64 tolerance (old code), this might erroneously pass/fail
        let order_total = 10.30;
        let already_refunded = 7.20;
        let new_refund = rust_decimal::Decimal::new(310, 2); // 3.10

        // This should pass because 7.20 + 3.10 = 10.30
        assert!(
            check_over_refund(order_total, already_refunded, new_refund).is_ok(),
            "f64 precision edge case: 7.20 + 3.10 should equal 10.30"
        );
    }

    #[test]
    fn anti_overrefund_partial_refund_leaves_remainder() {
        let order_total = 25.50;
        let already_refunded = 0.0;
        let new_refund = rust_decimal::Decimal::new(1275, 2); // 12.75 (half)
        assert!(check_over_refund(order_total, already_refunded, new_refund).is_ok());

        // Second refund of the other half
        assert!(check_over_refund(order_total, 12.75, rust_decimal::Decimal::new(1275, 2)).is_ok());

        // One cent over the remaining
        assert!(
            check_over_refund(order_total, 12.75, rust_decimal::Decimal::new(1276, 2)).is_err()
        );
    }
}
