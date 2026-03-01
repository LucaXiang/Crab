//! Invoice Upgrade Service (F2 → F3 Sustitutiva)
//!
//! Creates F3 invoices that substitute the original F2 simplified invoice,
//! adding customer information (NIF, company name, etc.) while keeping
//! the same amounts as the original F2.

use crate::db::repository::invoice as inv_repo;
use shared::models::invoice::{AeatStatus, Invoice, InvoiceSourceType, TipoFactura};
use shared::order::verifactu::{HuellaAltaInput, compute_verifactu_huella_alta};
use shared::util::snowflake_id;
use sqlx::SqlitePool;
use std::sync::Arc;
use tokio::sync::Mutex;

use super::invoice::InvoiceService;
use super::service::{ArchiveError, ArchiveResult};

/// Request to create an invoice upgrade (F2 → F3)
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

/// Service for upgrading F2 invoices to F3 (sustitutiva)
#[derive(Clone)]
pub struct UpgradeService {
    pool: SqlitePool,
    /// Shared with OrderArchiveService, CreditNoteService, AnulacionService
    hash_chain_lock: Arc<Mutex<()>>,
    invoice_service: Option<InvoiceService>,
}

impl UpgradeService {
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

    /// Create an F3 sustitutiva invoice.
    ///
    /// Preconditions:
    /// 1. Order must be archived and COMPLETED
    /// 2. Order must not be voided (is_anulada = 0)
    /// 3. Order must not be already upgraded (is_upgraded = 0)
    /// 4. Order must have an F2 invoice
    /// 5. InvoiceService must be configured (Verifactu enabled)
    ///
    /// The F3 invoice copies amounts from the original F2, adds customer info,
    /// and sets factura_sustituida reference.
    pub async fn create_upgrade(
        &self,
        request: &CreateUpgradeRequest,
        operator_id: i64,
        operator_name: &str,
    ) -> ArchiveResult<Invoice> {
        let inv_svc = self.invoice_service.as_ref().ok_or_else(|| {
            ArchiveError::Validation("Verifactu not configured — cannot create upgrade".into())
        })?;

        // Acquire hash chain lock
        let _hash_lock = self.hash_chain_lock.lock().await;

        let now = shared::util::now_millis();

        // 1. Validate order
        let order = sqlx::query_as::<_, OrderUpgradeRef>(
            "SELECT id, status, is_anulada, is_upgraded FROM archived_order WHERE id = ?",
        )
        .bind(request.order_pk)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ArchiveError::Database(e.to_string()))?
        .ok_or_else(|| {
            ArchiveError::Validation(format!("Order not found: {}", request.order_pk))
        })?;

        if order.status != "COMPLETED" {
            return Err(ArchiveError::Validation(format!(
                "Order status is '{}', only COMPLETED orders can be upgraded",
                order.status
            )));
        }

        if order.is_anulada != 0 {
            return Err(ArchiveError::Validation(
                "Order is voided — cannot upgrade".into(),
            ));
        }

        if order.is_upgraded != 0 {
            return Err(ArchiveError::Validation(
                "Order already has an F3 upgrade".into(),
            ));
        }

        // 2. Begin transaction and find original F2 invoice
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| ArchiveError::Database(e.to_string()))?;

        let f2_invoice = inv_repo::find_order_invoice(&mut tx, request.order_pk)
            .await
            .map_err(|e| ArchiveError::Database(e.to_string()))?
            .ok_or_else(|| {
                ArchiveError::Validation(format!(
                    "No F2 invoice found for order {}",
                    request.order_pk
                ))
            })?;

        // 3. Copy desglose from F2
        let f2_desglose = inv_repo::get_desglose_tx(&mut tx, f2_invoice.id)
            .await
            .map_err(|e| ArchiveError::Database(e.to_string()))?;

        // 4. Compute huella (F3 uses huella_alta same as F2/R5)
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

        let invoice_number = inv_repo::next_invoice_number(&mut tx, inv_svc.serie(), &date_str)
            .await
            .map_err(|e| ArchiveError::InvoiceNumber(e.to_string()))?;

        let huella = compute_verifactu_huella_alta(&HuellaAltaInput {
            nif: inv_svc.nif(),
            invoice_number: &invoice_number,
            fecha_expedicion: &fecha_expedicion,
            tipo_factura: TipoFactura::F3.as_str(),
            cuota_total: f2_invoice.tax,
            importe_total: f2_invoice.total,
            prev_huella: prev_huella.as_deref(),
            fecha_hora_registro: &fecha_hora_registro,
        })?;

        // 5. Build F3 invoice (amounts copied from F2)
        let invoice = Invoice {
            id: 0,
            invoice_number: invoice_number.clone(),
            serie: inv_svc.serie().to_string(),
            tipo_factura: TipoFactura::F3,
            source_type: InvoiceSourceType::Upgrade,
            source_pk: request.order_pk,
            subtotal: f2_invoice.subtotal,
            tax: f2_invoice.tax,
            total: f2_invoice.total,
            huella: huella.clone(),
            prev_huella,
            fecha_expedicion,
            fecha_hora_registro,
            nif: inv_svc.nif().to_string(),
            nombre_razon: inv_svc.nombre_razon().to_string(),
            factura_rectificada_id: None,
            factura_rectificada_num: None,
            factura_sustituida_id: Some(f2_invoice.id),
            factura_sustituida_num: Some(f2_invoice.invoice_number.clone()),
            customer_nif: Some(request.customer_nif.clone()),
            customer_nombre: Some(request.customer_nombre.clone()),
            customer_address: request.customer_address.clone(),
            customer_email: request.customer_email.clone(),
            customer_phone: request.customer_phone.clone(),
            cloud_synced: false,
            aeat_status: AeatStatus::Pending,
            created_at: now,
        };

        // 6. Insert F3 invoice
        let f3_invoice_id = inv_repo::insert(&mut tx, &invoice).await?;

        // 7. Copy desglose lines from F2
        for d in &f2_desglose {
            inv_repo::insert_desglose(
                &mut tx,
                f3_invoice_id,
                d.tax_rate,
                d.base_amount,
                d.tax_amount,
            )
            .await?;
        }

        // 8. Compute chain hash (prev_hash already read from tx in step 4a)
        let chain_hash = shared::order::compute_upgrade_chain_hash(
            &prev_hash,
            &invoice_number,
            &f2_invoice.invoice_number,
            request.order_pk,
            f2_invoice.total,
            f2_invoice.tax,
            now,
            operator_name,
        );

        // 9. Insert chain_entry
        let chain_entry_id = snowflake_id();
        sqlx::query(
            "INSERT INTO chain_entry (id, entry_type, entry_pk, prev_hash, curr_hash, created_at) \
             VALUES (?1, 'UPGRADE', ?2, ?3, ?4, ?5)",
        )
        .bind(chain_entry_id)
        .bind(f3_invoice_id)
        .bind(&prev_hash)
        .bind(&chain_hash)
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(|e| ArchiveError::Database(e.to_string()))?;

        // 10. Update system_state
        sqlx::query(
            "UPDATE system_state SET last_chain_hash = ?1, last_huella = ?2, updated_at = ?3 WHERE id = 1",
        )
        .bind(&chain_hash)
        .bind(&huella)
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(|e| ArchiveError::Database(e.to_string()))?;

        // 11. Mark archived_order as upgraded
        sqlx::query("UPDATE archived_order SET is_upgraded = 1 WHERE id = ?")
            .bind(request.order_pk)
            .execute(&mut *tx)
            .await
            .map_err(|e| ArchiveError::Database(e.to_string()))?;

        tx.commit()
            .await
            .map_err(|e| ArchiveError::Database(e.to_string()))?;

        tracing::info!(
            f3_invoice_id,
            invoice_number = %invoice_number,
            original_f2 = %f2_invoice.invoice_number,
            order_pk = request.order_pk,
            customer_nif = %request.customer_nif,
            operator_id,
            "F3 sustitutiva invoice created"
        );

        // Read back
        inv_repo::get_by_id(&self.pool, f3_invoice_id)
            .await
            .map_err(|e| ArchiveError::Database(e.to_string()))?
            .ok_or_else(|| ArchiveError::Database("Failed to read F3 invoice after insert".into()))
    }

    /// Check if an order is eligible for F3 upgrade.
    pub async fn check_upgrade_eligibility(&self, order_pk: i64) -> ArchiveResult<()> {
        if self.invoice_service.is_none() {
            return Err(ArchiveError::Validation("Verifactu not configured".into()));
        }

        let order = sqlx::query_as::<_, OrderUpgradeRef>(
            "SELECT id, status, is_anulada, is_upgraded FROM archived_order WHERE id = ?",
        )
        .bind(order_pk)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ArchiveError::Database(e.to_string()))?
        .ok_or_else(|| ArchiveError::Validation(format!("Order not found: {order_pk}")))?;

        if order.status != "COMPLETED" {
            return Err(ArchiveError::Validation(format!(
                "Order status is '{}', only COMPLETED orders can be upgraded",
                order.status
            )));
        }

        if order.is_anulada != 0 {
            return Err(ArchiveError::Validation(
                "Order is voided — cannot upgrade".into(),
            ));
        }

        if order.is_upgraded != 0 {
            return Err(ArchiveError::Validation(
                "Order already has an F3 upgrade".into(),
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
struct OrderUpgradeRef {
    #[allow(dead_code)]
    id: i64,
    status: String,
    is_anulada: i64,
    is_upgraded: i64,
}
