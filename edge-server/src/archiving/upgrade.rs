//! Upgrade Service — Order-layer upgrade (mark order as upgraded)
//!
//! Marks an archived order as upgraded + chain_entry + chain_hash.
//! Invoice layer (Verifactu F2→F3) is a downstream consumer that scans chain entries separately.
//!
//! Shares the same hash_chain_lock as OrderArchiveService and CreditNoteService.

use shared::util::snowflake_id;
use sqlx::SqlitePool;
use std::sync::Arc;
use tokio::sync::Mutex;

use shared::error::ErrorCode;

use super::service::{ArchiveError, ArchiveResult};

/// Request to create an order upgrade
#[derive(Debug, Clone, serde::Deserialize)]
pub struct CreateUpgradeRequest {
    pub order_pk: i64,
    pub customer_nif: String,
    pub customer_nombre: String,
    #[serde(default)]
    pub customer_address: Option<String>,
    #[serde(default)]
    pub customer_email: Option<String>,
    #[serde(default)]
    pub customer_phone: Option<String>,
}

/// Response from creating an upgrade
#[derive(Debug, Clone, serde::Serialize)]
pub struct UpgradeResponse {
    pub order_pk: i64,
    pub chain_entry_id: i64,
    pub receipt_number: String,
}

/// Service for upgrading orders
#[derive(Clone)]
pub struct UpgradeService {
    pool: SqlitePool,
    /// Shared with OrderArchiveService, CreditNoteService, AnulacionService
    hash_chain_lock: Arc<Mutex<()>>,
}

impl UpgradeService {
    pub fn new(pool: SqlitePool, hash_chain_lock: Arc<Mutex<()>>) -> Self {
        Self {
            pool,
            hash_chain_lock,
        }
    }

    /// Create an order upgrade (order-layer).
    ///
    /// - Validates order is COMPLETED, not anulada, not already upgraded
    /// - Creates chain_entry (entry_pk = order_pk, type = UPGRADE)
    /// - Updates system_state.last_chain_hash
    /// - Marks archived_order.is_upgraded = 1
    pub async fn create_upgrade(
        &self,
        request: &CreateUpgradeRequest,
        operator_id: i64,
        operator_name: &str,
    ) -> ArchiveResult<UpgradeResponse> {
        // Acquire hash chain lock
        let _hash_lock = self.hash_chain_lock.lock().await;

        let now = shared::util::now_millis();

        // 1. Validate order
        let order = sqlx::query_as::<_, OrderUpgradeRef>(
            "SELECT id, status, is_voided, is_upgraded, receipt_number, total_amount, tax \
             FROM archived_order WHERE id = ?",
        )
        .bind(request.order_pk)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ArchiveError::Database(e.to_string()))?
        .ok_or_else(|| {
            ArchiveError::BusinessRule(
                ErrorCode::OrderNotFound,
                format!("Order not found: {}", request.order_pk),
            )
        })?;

        if order.status != "COMPLETED" {
            return Err(ArchiveError::BusinessRule(
                ErrorCode::OrderNotCompleted,
                format!(
                    "Order status is '{}', only COMPLETED orders can be upgraded",
                    order.status
                ),
            ));
        }

        if order.is_voided != 0 {
            return Err(ArchiveError::BusinessRule(
                ErrorCode::OrderVoidedNoCreditNote,
                "Order is voided — cannot upgrade".into(),
            ));
        }

        if order.is_upgraded != 0 {
            return Err(ArchiveError::BusinessRule(
                ErrorCode::OrderAlreadyUpgraded,
                "Order already has an upgrade".into(),
            ));
        }

        // 2. Begin transaction
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| ArchiveError::Database(e.to_string()))?;

        // 3. Read last_chain_hash
        let prev_hash: String = sqlx::query_scalar(
            "SELECT COALESCE(last_chain_hash, 'genesis') FROM system_state WHERE id = 1",
        )
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| ArchiveError::Database(e.to_string()))?;

        // 4. Compute chain hash
        let receipt = order.receipt_number.as_deref().unwrap_or("unknown");
        let chain_hash = shared::order::compute_upgrade_chain_hash(
            &prev_hash,
            &format!("UPG-{receipt}"),
            receipt,
            request.order_pk,
            order.total_amount,
            order.tax,
            now,
            operator_name,
        );

        // 5. Insert chain_entry (entry_pk = order_pk)
        let chain_entry_id = snowflake_id();
        sqlx::query(
            "INSERT INTO chain_entry (id, entry_type, entry_pk, prev_hash, curr_hash, created_at) \
             VALUES (?1, 'UPGRADE', ?2, ?3, ?4, ?5)",
        )
        .bind(chain_entry_id)
        .bind(request.order_pk)
        .bind(&prev_hash)
        .bind(&chain_hash)
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(|e| ArchiveError::Database(e.to_string()))?;

        // 6. Update system_state.last_chain_hash
        sqlx::query("UPDATE system_state SET last_chain_hash = ?1, updated_at = ?2 WHERE id = 1")
            .bind(&chain_hash)
            .bind(now)
            .execute(&mut *tx)
            .await
            .map_err(|e| ArchiveError::Database(e.to_string()))?;

        // 7. Mark archived_order as upgraded + store customer info + reset cloud_synced for re-sync
        sqlx::query(
            "UPDATE archived_order SET is_upgraded = 1, cloud_synced = 0, \
             customer_nif = ?1, customer_nombre = ?2, customer_address = ?3, \
             customer_email = ?4, customer_phone = ?5 \
             WHERE id = ?6",
        )
        .bind(&request.customer_nif)
        .bind(&request.customer_nombre)
        .bind(&request.customer_address)
        .bind(&request.customer_email)
        .bind(&request.customer_phone)
        .bind(request.order_pk)
        .execute(&mut *tx)
        .await
        .map_err(|e| ArchiveError::Database(e.to_string()))?;

        tx.commit()
            .await
            .map_err(|e| ArchiveError::Database(e.to_string()))?;

        tracing::info!(
            order_pk = request.order_pk,
            %operator_id,
            operator_name,
            customer_nif = %request.customer_nif,
            "Order upgrade created"
        );

        Ok(UpgradeResponse {
            order_pk: request.order_pk,
            chain_entry_id,
            receipt_number: receipt.to_string(),
        })
    }

    /// Check if an order is eligible for upgrade.
    pub async fn check_upgrade_eligibility(&self, order_pk: i64) -> ArchiveResult<()> {
        let order = sqlx::query_as::<_, OrderUpgradeRef>(
            "SELECT id, status, is_voided, is_upgraded, receipt_number, total_amount, tax \
             FROM archived_order WHERE id = ?",
        )
        .bind(order_pk)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ArchiveError::Database(e.to_string()))?
        .ok_or_else(|| {
            ArchiveError::BusinessRule(
                ErrorCode::OrderNotFound,
                format!("Order not found: {order_pk}"),
            )
        })?;

        if order.status != "COMPLETED" {
            return Err(ArchiveError::BusinessRule(
                ErrorCode::OrderNotCompleted,
                format!(
                    "Order status is '{}', only COMPLETED orders can be upgraded",
                    order.status
                ),
            ));
        }

        if order.is_voided != 0 {
            return Err(ArchiveError::BusinessRule(
                ErrorCode::OrderVoidedNoCreditNote,
                "Order is voided — cannot upgrade".into(),
            ));
        }

        if order.is_upgraded != 0 {
            return Err(ArchiveError::BusinessRule(
                ErrorCode::OrderAlreadyUpgraded,
                "Order already has an upgrade".into(),
            ));
        }

        Ok(())
    }
}

// ============================================================================
// Internal query helper types
// ============================================================================

#[derive(Debug, sqlx::FromRow)]
struct OrderUpgradeRef {
    #[allow(dead_code)]
    id: i64,
    status: String,
    is_voided: i64,
    is_upgraded: i64,
    receipt_number: Option<String>,
    total_amount: f64,
    tax: f64,
}
