//! Invoice Service (Verifactu)
//!
//! Creates F2 invoices for completed orders and R5 invoices for credit notes,
//! maintaining the Verifactu huella (fingerprint) hash chain required by AEAT.

use crate::db::repository::{invoice as inv_repo, system_state};
use shared::cloud::sync::TaxDesglose;
use shared::models::invoice::{AeatStatus, Invoice, InvoiceSourceType, TipoFactura};
use shared::order::verifactu::{HuellaAltaInput, compute_verifactu_huella_alta};
use sqlx::SqlitePool;

use super::service::ArchiveResult;

/// Service for creating Verifactu invoices (F2/R5) with huella chain.
#[derive(Clone)]
pub struct InvoiceService {
    pool: SqlitePool,
    tz: chrono_tz::Tz,
    serie: String,
    nif: String,
    nombre_razon: String,
}

impl InvoiceService {
    pub fn new(
        pool: SqlitePool,
        tz: chrono_tz::Tz,
        store_number: u32,
        nif: String,
        nombre_razon: String,
    ) -> Self {
        let serie = store_number_to_serie(store_number);
        Self {
            pool,
            tz,
            serie,
            nif,
            nombre_razon,
        }
    }

    /// Create an F2 invoice for a completed order.
    ///
    /// Returns `Ok(None)` if total <= 0 (comped orders skip invoicing).
    /// The `tx` is the caller's open transaction; huella prev is read from pool
    /// (system_state was already read before the tx started in the archive flow).
    pub async fn create_order_invoice(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        order_pk: i64,
        subtotal: f64,
        tax: f64,
        total: f64,
        desglose: &[TaxDesglose],
    ) -> ArchiveResult<Option<i64>> {
        if total <= 0.0 {
            tracing::debug!(order_pk, "Skipping F2 invoice for zero/negative total");
            return Ok(None);
        }

        let now_dt = chrono::Utc::now().with_timezone(&self.tz);
        let fecha_expedicion = now_dt.format("%d-%m-%Y").to_string();
        let fecha_hora_registro = now_dt.to_rfc3339();
        let date_str = now_dt.format("%Y%m%d").to_string();

        // Read prev huella from pool (not tx) — system_state was read before tx started
        let system_state = system_state::get_or_create(&self.pool)
            .await
            .map_err(|e| super::service::ArchiveError::Database(e.to_string()))?;
        let prev_huella = system_state.last_huella;

        // Allocate invoice number
        let invoice_number = inv_repo::next_invoice_number(tx, &self.serie, &date_str)
            .await
            .map_err(|e| super::service::ArchiveError::Database(e.to_string()))?;

        // Compute huella
        let huella = compute_verifactu_huella_alta(&HuellaAltaInput {
            nif: &self.nif,
            invoice_number: &invoice_number,
            fecha_expedicion: &fecha_expedicion,
            tipo_factura: TipoFactura::F2.as_str(),
            cuota_total: tax,
            importe_total: total,
            prev_huella: prev_huella.as_deref(),
            fecha_hora_registro: &fecha_hora_registro,
        });

        let now = shared::util::now_millis();

        let invoice = Invoice {
            id: 0, // will be assigned by repo
            invoice_number,
            serie: self.serie.clone(),
            tipo_factura: TipoFactura::F2,
            source_type: InvoiceSourceType::Order,
            source_pk: order_pk,
            subtotal,
            tax,
            total,
            huella: huella.clone(),
            prev_huella,
            fecha_expedicion,
            nif: self.nif.clone(),
            nombre_razon: self.nombre_razon.clone(),
            factura_rectificada_id: None,
            factura_rectificada_num: None,
            cloud_synced: false,
            aeat_status: AeatStatus::Pending,
            created_at: now,
        };

        let invoice_id = inv_repo::insert(tx, &invoice)
            .await
            .map_err(|e| super::service::ArchiveError::Database(e.to_string()))?;

        // Insert desglose lines
        for d in desglose {
            inv_repo::insert_desglose(
                tx,
                invoice_id,
                d.tax_rate as i64,
                d.base_amount.try_into().unwrap_or(0.0),
                d.tax_amount.try_into().unwrap_or(0.0),
            )
            .await
            .map_err(|e| super::service::ArchiveError::Database(e.to_string()))?;
        }

        // Update system_state.last_huella
        sqlx::query("UPDATE system_state SET last_huella = ?1, updated_at = ?2 WHERE id = 1")
            .bind(&huella)
            .bind(now)
            .execute(&mut **tx)
            .await
            .map_err(|e| super::service::ArchiveError::Database(e.to_string()))?;

        tracing::info!(
            invoice_id,
            invoice_number = %invoice.invoice_number,
            order_pk,
            "F2 invoice created"
        );

        Ok(Some(invoice_id))
    }

    /// Create an R5 (rectificativa) invoice for a credit note.
    ///
    /// Looks up the original F2 invoice for the order to set factura_rectificada fields.
    /// If no F2 exists (order was comped), logs a warning and sets rectificada fields to None.
    #[allow(clippy::too_many_arguments)]
    pub async fn create_credit_note_invoice(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        cn_pk: i64,
        original_order_pk: i64,
        subtotal: f64,
        tax: f64,
        total: f64,
        desglose: &[TaxDesglose],
    ) -> ArchiveResult<Option<i64>> {
        if total <= 0.0 {
            tracing::debug!(cn_pk, "Skipping R5 invoice for zero/negative total");
            return Ok(None);
        }

        let now_dt = chrono::Utc::now().with_timezone(&self.tz);
        let fecha_expedicion = now_dt.format("%d-%m-%Y").to_string();
        let fecha_hora_registro = now_dt.to_rfc3339();
        let date_str = now_dt.format("%Y%m%d").to_string();

        // Look up original F2 invoice
        let original_f2 = inv_repo::find_order_invoice(tx, original_order_pk)
            .await
            .map_err(|e| super::service::ArchiveError::Database(e.to_string()))?;

        let (rectificada_id, rectificada_num) = match original_f2 {
            Some(ref f2) => (Some(f2.id), Some(f2.invoice_number.clone())),
            None => {
                tracing::warn!(
                    cn_pk,
                    original_order_pk,
                    "No F2 invoice found for credit note (order was likely comped)"
                );
                (None, None)
            }
        };

        // Read prev huella from pool
        let system_state = system_state::get_or_create(&self.pool)
            .await
            .map_err(|e| super::service::ArchiveError::Database(e.to_string()))?;
        let prev_huella = system_state.last_huella;

        // Allocate invoice number
        let invoice_number = inv_repo::next_invoice_number(tx, &self.serie, &date_str)
            .await
            .map_err(|e| super::service::ArchiveError::Database(e.to_string()))?;

        // Compute huella
        let huella = compute_verifactu_huella_alta(&HuellaAltaInput {
            nif: &self.nif,
            invoice_number: &invoice_number,
            fecha_expedicion: &fecha_expedicion,
            tipo_factura: TipoFactura::R5.as_str(),
            cuota_total: tax,
            importe_total: total,
            prev_huella: prev_huella.as_deref(),
            fecha_hora_registro: &fecha_hora_registro,
        });

        let now = shared::util::now_millis();

        let invoice = Invoice {
            id: 0,
            invoice_number,
            serie: self.serie.clone(),
            tipo_factura: TipoFactura::R5,
            source_type: InvoiceSourceType::CreditNote,
            source_pk: cn_pk,
            subtotal,
            tax,
            total,
            huella: huella.clone(),
            prev_huella,
            fecha_expedicion,
            nif: self.nif.clone(),
            nombre_razon: self.nombre_razon.clone(),
            factura_rectificada_id: rectificada_id,
            factura_rectificada_num: rectificada_num,
            cloud_synced: false,
            aeat_status: AeatStatus::Pending,
            created_at: now,
        };

        let invoice_id = inv_repo::insert(tx, &invoice)
            .await
            .map_err(|e| super::service::ArchiveError::Database(e.to_string()))?;

        // Insert desglose lines
        for d in desglose {
            inv_repo::insert_desglose(
                tx,
                invoice_id,
                d.tax_rate as i64,
                d.base_amount.try_into().unwrap_or(0.0),
                d.tax_amount.try_into().unwrap_or(0.0),
            )
            .await
            .map_err(|e| super::service::ArchiveError::Database(e.to_string()))?;
        }

        // Update system_state.last_huella
        sqlx::query("UPDATE system_state SET last_huella = ?1, updated_at = ?2 WHERE id = 1")
            .bind(&huella)
            .bind(now)
            .execute(&mut **tx)
            .await
            .map_err(|e| super::service::ArchiveError::Database(e.to_string()))?;

        tracing::info!(
            invoice_id,
            invoice_number = %invoice.invoice_number,
            cn_pk,
            original_order_pk,
            "R5 invoice created"
        );

        Ok(Some(invoice_id))
    }
}

/// Convert store number to invoice serie letter.
///
/// 1→"A", 2→"B", ..., 26→"Z", 0 or >26 → "S{n}"
pub fn store_number_to_serie(store_number: u32) -> String {
    if (1..=26).contains(&store_number) {
        let letter = (b'A' + (store_number - 1) as u8) as char;
        letter.to_string()
    } else {
        format!("S{store_number}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn store_number_to_serie_letters() {
        assert_eq!(store_number_to_serie(1), "A");
        assert_eq!(store_number_to_serie(2), "B");
        assert_eq!(store_number_to_serie(26), "Z");
    }

    #[test]
    fn store_number_to_serie_fallback() {
        assert_eq!(store_number_to_serie(0), "S0");
        assert_eq!(store_number_to_serie(27), "S27");
        assert_eq!(store_number_to_serie(100), "S100");
    }
}
