//! Invoice Anulación Service (Verifactu RegistroFacturaBaja)
//!
//! Creates invoice anulaciones (legal invoice revocation) with hash chain + huella chain integrity.
//! Shares the same hash_chain_lock as OrderArchiveService and CreditNoteService.

use crate::db::repository::{anulacion as anulacion_repo, invoice as inv_repo};
use shared::models::invoice::{AeatStatus, AnulacionReason, InvoiceAnulacion};
use shared::order::verifactu::{HuellaBajaInput, compute_verifactu_huella_baja};
use shared::util::snowflake_id;
use sqlx::SqlitePool;
use std::sync::Arc;
use tokio::sync::Mutex;

use super::invoice::InvoiceService;
use super::service::{ArchiveError, ArchiveResult};

/// Request to create an invoice anulación
#[derive(Debug, Clone, serde::Deserialize)]
pub struct CreateAnulacionRequest {
    pub original_order_pk: i64,
    pub reason: AnulacionReason,
    #[serde(default)]
    pub note: Option<String>,
}

/// Service for creating invoice anulaciones
#[derive(Clone)]
pub struct AnulacionService {
    pool: SqlitePool,
    /// Shared with OrderArchiveService & CreditNoteService — serializes all chain writes
    hash_chain_lock: Arc<Mutex<()>>,
    /// Invoice service for reading invoice data and getting serie/nif
    invoice_service: Option<InvoiceService>,
}

impl AnulacionService {
    pub fn new(
        pool: SqlitePool,
        hash_chain_lock: Arc<Mutex<()>>,
        invoice_service: Option<InvoiceService>,
    ) -> Self {
        Self {
            pool,
            hash_chain_lock,
            invoice_service,
        }
    }

    /// Create an invoice anulación (RegistroFacturaBaja).
    ///
    /// Preconditions:
    /// 1. Order must be archived and COMPLETED (not already voided/merged)
    /// 2. Order must have NO credit notes (anulación and R5 are mutually exclusive)
    /// 3. Order must have an F2 invoice
    /// 4. Order must not already have an anulación
    /// 5. InvoiceService must be configured (Verifactu enabled)
    ///
    /// This creates:
    /// - invoice_anulacion record
    /// - chain_entry with type ANULACION
    /// - Updates system_state.last_chain_hash and last_huella
    /// - Marks archived_order.is_anulada = 1
    pub async fn create_anulacion(
        &self,
        request: &CreateAnulacionRequest,
        operator_id: i64,
        operator_name: &str,
    ) -> ArchiveResult<InvoiceAnulacion> {
        let inv_svc = self.invoice_service.as_ref().ok_or_else(|| {
            ArchiveError::Validation("Verifactu not configured — cannot create anulación".into())
        })?;

        // Acquire hash chain lock (shared with archive + credit note services)
        let _hash_lock = self.hash_chain_lock.lock().await;

        let now = shared::util::now_millis();

        // 1. Validate order exists and is COMPLETED
        let order = sqlx::query_as::<_, OrderStatusRef>(
            "SELECT id, status, is_anulada FROM archived_order WHERE id = ?",
        )
        .bind(request.original_order_pk)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ArchiveError::Database(e.to_string()))?
        .ok_or_else(|| {
            ArchiveError::Validation(format!("Order not found: {}", request.original_order_pk))
        })?;

        if order.status != "COMPLETED" {
            return Err(ArchiveError::Validation(format!(
                "Order status is '{}', only COMPLETED orders can be anulled",
                order.status
            )));
        }

        if order.is_anulada != 0 {
            return Err(ArchiveError::Validation(
                "Order already has an anulación".into(),
            ));
        }

        // 2. Check no credit notes exist (mutually exclusive with R5)
        let cn_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM credit_note WHERE original_order_pk = ?")
                .bind(request.original_order_pk)
                .fetch_one(&self.pool)
                .await
                .map_err(|e| ArchiveError::Database(e.to_string()))?;

        if cn_count > 0 {
            return Err(ArchiveError::Validation(
                "Cannot create anulación: order has credit notes (use R5 refund instead)".into(),
            ));
        }

        // 3. Find the F2 invoice for this order
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| ArchiveError::Database(e.to_string()))?;

        let f2_invoice = inv_repo::find_order_invoice(&mut tx, request.original_order_pk)
            .await
            .map_err(|e| ArchiveError::Database(e.to_string()))?
            .ok_or_else(|| {
                ArchiveError::Validation(format!(
                    "No F2 invoice found for order {}",
                    request.original_order_pk
                ))
            })?;

        // 4. Compute huella (Baja formula — shares chain with Alta)
        let now_dt = chrono::Utc::now().with_timezone(&inv_svc.tz());
        let fecha_expedicion = now_dt.format("%d-%m-%Y").to_string();
        let fecha_hora_registro = now_dt.to_rfc3339();
        let date_str = now_dt.format("%Y%m%d").to_string();

        // 4a. Read both last_huella and last_chain_hash from tx (single row)
        let (prev_huella, prev_hash) = {
            let row: Option<(Option<String>, Option<String>)> = sqlx::query_as(
                "SELECT last_huella, last_chain_hash FROM system_state WHERE id = 1",
            )
            .fetch_optional(&mut *tx)
            .await
            .map_err(|e| ArchiveError::Database(e.to_string()))?;

            match row {
                Some((huella, hash)) => (huella, hash.unwrap_or_else(|| "genesis".to_string())),
                None => (None, "genesis".to_string()),
            }
        };

        let anulacion_number =
            anulacion_repo::next_anulacion_number(&mut tx, inv_svc.serie(), &date_str)
                .await
                .map_err(|e| ArchiveError::InvoiceNumber(e.to_string()))?;

        let huella = compute_verifactu_huella_baja(&HuellaBajaInput {
            nif: inv_svc.nif(),
            invoice_number: &f2_invoice.invoice_number,
            fecha_expedicion: &fecha_expedicion,
            prev_huella: prev_huella.as_deref(),
            fecha_hora_registro: &fecha_hora_registro,
        });

        // 6. Compute chain hash
        let chain_hash = shared::order::compute_anulacion_chain_hash(
            &prev_hash,
            &anulacion_number,
            &f2_invoice.invoice_number,
            request.original_order_pk,
            request.reason.as_str(),
            now,
            operator_name,
        );

        // 7. Build anulación record
        let anulacion = InvoiceAnulacion {
            id: 0, // will be assigned by repo
            anulacion_number: anulacion_number.clone(),
            serie: inv_svc.serie().to_string(),
            original_invoice_id: f2_invoice.id,
            original_invoice_number: f2_invoice.invoice_number.clone(),
            huella: huella.clone(),
            prev_huella,
            fecha_expedicion,
            fecha_hora_registro,
            nif: inv_svc.nif().to_string(),
            nombre_razon: inv_svc.nombre_razon().to_string(),
            original_order_pk: request.original_order_pk,
            reason: request.reason.clone(),
            note: request.note.clone(),
            operator_id,
            operator_name: operator_name.to_string(),
            cloud_synced: false,
            aeat_status: AeatStatus::Pending,
            created_at: now,
        };

        // 8. Insert anulación
        let anulacion_pk = anulacion_repo::insert(&mut tx, &anulacion)
            .await
            .map_err(|e| ArchiveError::Database(e.to_string()))?;

        // 9. Insert chain_entry
        let chain_entry_id = snowflake_id();
        sqlx::query(
            "INSERT INTO chain_entry (id, entry_type, entry_pk, prev_hash, curr_hash, created_at) \
             VALUES (?1, 'ANULACION', ?2, ?3, ?4, ?5)",
        )
        .bind(chain_entry_id)
        .bind(anulacion_pk)
        .bind(&prev_hash)
        .bind(&chain_hash)
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(|e| ArchiveError::Database(e.to_string()))?;

        // 10. Update system_state.last_chain_hash + last_huella
        sqlx::query(
            "UPDATE system_state SET last_chain_hash = ?1, last_huella = ?2, updated_at = ?3 WHERE id = 1",
        )
        .bind(&chain_hash)
        .bind(&huella)
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(|e| ArchiveError::Database(e.to_string()))?;

        // 11. Mark archived_order as anulada
        sqlx::query("UPDATE archived_order SET is_anulada = 1 WHERE id = ?")
            .bind(request.original_order_pk)
            .execute(&mut *tx)
            .await
            .map_err(|e| ArchiveError::Database(e.to_string()))?;

        tx.commit()
            .await
            .map_err(|e| ArchiveError::Database(e.to_string()))?;

        tracing::info!(
            anulacion_number = %anulacion_number,
            original_invoice = %f2_invoice.invoice_number,
            order_pk = request.original_order_pk,
            reason = %request.reason,
            "Invoice anulación created"
        );

        // Read back
        anulacion_repo::get_by_id(&self.pool, anulacion_pk)
            .await
            .map_err(|e| ArchiveError::Database(e.to_string()))?
            .ok_or_else(|| ArchiveError::Database("Failed to read anulación after insert".into()))
    }

    /// Check if an order can be anulled.
    ///
    /// Returns Ok(()) if eligible, Err with reason if not.
    pub async fn check_anulacion_eligibility(&self, order_pk: i64) -> ArchiveResult<()> {
        if self.invoice_service.is_none() {
            return Err(ArchiveError::Validation("Verifactu not configured".into()));
        }

        let order = sqlx::query_as::<_, OrderStatusRef>(
            "SELECT id, status, is_anulada FROM archived_order WHERE id = ?",
        )
        .bind(order_pk)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ArchiveError::Database(e.to_string()))?
        .ok_or_else(|| ArchiveError::Validation(format!("Order not found: {order_pk}")))?;

        if order.status != "COMPLETED" {
            return Err(ArchiveError::Validation(format!(
                "Order status is '{}', only COMPLETED orders can be anulled",
                order.status
            )));
        }

        if order.is_anulada != 0 {
            return Err(ArchiveError::Validation(
                "Order already has an anulación".into(),
            ));
        }

        let cn_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM credit_note WHERE original_order_pk = ?")
                .bind(order_pk)
                .fetch_one(&self.pool)
                .await
                .map_err(|e| ArchiveError::Database(e.to_string()))?;

        if cn_count > 0 {
            return Err(ArchiveError::Validation(
                "Order has credit notes — cannot create anulación".into(),
            ));
        }

        // Check F2 invoice exists
        let invoice_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM invoice WHERE source_type = 'ORDER' AND source_pk = ? AND tipo_factura = 'F2'",
        )
        .bind(order_pk)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| ArchiveError::Database(e.to_string()))?;

        if invoice_count == 0 {
            return Err(ArchiveError::Validation(
                "No F2 invoice found for order".into(),
            ));
        }

        Ok(())
    }
}

// ============================================================================
// Internal query helper types
// ============================================================================

#[derive(Debug, sqlx::FromRow)]
struct OrderStatusRef {
    #[allow(dead_code)]
    id: i64,
    status: String,
    is_anulada: i64,
}
