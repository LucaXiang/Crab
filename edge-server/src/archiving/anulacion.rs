//! Anulación Service — Order-layer annulment
//!
//! Marks an archived order as anulada + chain_entry + chain_hash.
//! Invoice layer (Verifactu) is a downstream consumer that scans chain entries separately.
//!
//! Shares the same hash_chain_lock as OrderArchiveService and CreditNoteService.

use shared::models::invoice::AnulacionReason;
use shared::util::snowflake_id;
use sqlx::SqlitePool;
use std::sync::Arc;
use tokio::sync::Mutex;

use shared::error::ErrorCode;

use super::service::{ArchiveError, ArchiveResult};

/// Request to create an anulación
#[derive(Debug, Clone, serde::Deserialize)]
pub struct CreateAnulacionRequest {
    pub original_order_pk: i64,
    pub reason: AnulacionReason,
    #[serde(default)]
    pub note: Option<String>,
}

/// Response from creating an anulación
#[derive(Debug, Clone, serde::Serialize)]
pub struct AnulacionResponse {
    pub order_pk: i64,
    pub chain_entry_id: i64,
    pub receipt_number: String,
}

/// Service for creating anulaciones
#[derive(Clone)]
pub struct AnulacionService {
    pool: SqlitePool,
    /// Shared with OrderArchiveService & CreditNoteService — serializes all chain writes
    hash_chain_lock: Arc<Mutex<()>>,
}

impl AnulacionService {
    pub fn new(pool: SqlitePool, hash_chain_lock: Arc<Mutex<()>>) -> Self {
        Self {
            pool,
            hash_chain_lock,
        }
    }

    /// Create an anulación (order-layer).
    ///
    /// - Validates order is COMPLETED, not already anulada
    /// - Creates chain_entry (entry_pk = order_pk, type = ANULACION)
    /// - Updates system_state.last_chain_hash
    /// - Marks archived_order.is_voided = 1
    pub async fn create_anulacion(
        &self,
        request: &CreateAnulacionRequest,
        operator_id: i64,
        operator_name: &str,
    ) -> ArchiveResult<AnulacionResponse> {
        // Acquire hash chain lock (shared with archive + credit note services)
        let _hash_lock = self.hash_chain_lock.lock().await;

        let now = shared::util::now_millis();

        // 1. Validate order exists and is COMPLETED
        let order = sqlx::query_as::<_, OrderAnulRef>(
            "SELECT id, status, is_voided, receipt_number FROM archived_order WHERE id = ?",
        )
        .bind(request.original_order_pk)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ArchiveError::Database(e.to_string()))?
        .ok_or_else(|| {
            ArchiveError::BusinessRule(
                ErrorCode::OrderNotFound,
                format!("Order not found: {}", request.original_order_pk),
            )
        })?;

        if order.status != "COMPLETED" {
            return Err(ArchiveError::BusinessRule(
                ErrorCode::OrderNotCompleted,
                format!(
                    "Order status is '{}', only COMPLETED orders can be anulled",
                    order.status
                ),
            ));
        }

        if order.is_voided != 0 {
            return Err(ArchiveError::BusinessRule(
                ErrorCode::OrderAlreadyVoided,
                "Order already has an anulación".into(),
            ));
        }

        // 2. Begin transaction (credit notes are allowed — anulación voids the order regardless)
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
        let chain_hash = shared::order::compute_anulacion_chain_hash(
            &prev_hash,
            &format!("ANUL-{receipt}"),
            receipt,
            request.original_order_pk,
            request.reason.as_str(),
            now,
            operator_name,
        );

        // 5. Insert chain_entry (entry_pk = order_pk)
        let chain_entry_id = snowflake_id();
        sqlx::query(
            "INSERT INTO chain_entry (id, entry_type, entry_pk, prev_hash, curr_hash, created_at) \
             VALUES (?1, 'ANULACION', ?2, ?3, ?4, ?5)",
        )
        .bind(chain_entry_id)
        .bind(request.original_order_pk)
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

        // 7. Mark archived_order as anulada + reset cloud_synced for re-sync
        sqlx::query("UPDATE archived_order SET is_voided = 1, cloud_synced = 0 WHERE id = ?1")
            .bind(request.original_order_pk)
            .execute(&mut *tx)
            .await
            .map_err(|e| ArchiveError::Database(e.to_string()))?;

        tx.commit()
            .await
            .map_err(|e| ArchiveError::Database(e.to_string()))?;

        tracing::info!(
            order_pk = request.original_order_pk,
            %operator_id,
            operator_name,
            reason = %request.reason,
            "Anulación created"
        );

        Ok(AnulacionResponse {
            order_pk: request.original_order_pk,
            chain_entry_id,
            receipt_number: receipt.to_string(),
        })
    }

    /// Check if an order can be anulled (order-layer eligibility).
    pub async fn check_anulacion_eligibility(&self, order_pk: i64) -> ArchiveResult<()> {
        let order = sqlx::query_as::<_, OrderAnulRef>(
            "SELECT id, status, is_voided, receipt_number FROM archived_order WHERE id = ?",
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
                    "Order status is '{}', only COMPLETED orders can be anulled",
                    order.status
                ),
            ));
        }

        if order.is_voided != 0 {
            return Err(ArchiveError::BusinessRule(
                ErrorCode::OrderAlreadyVoided,
                "Order already has an anulación".into(),
            ));
        }

        Ok(())
    }
}

// ============================================================================
// Internal query helper types
// ============================================================================

#[derive(Debug, sqlx::FromRow)]
struct OrderAnulRef {
    #[allow(dead_code)]
    id: i64,
    status: String,
    is_voided: i64,
    receipt_number: Option<String>,
}
