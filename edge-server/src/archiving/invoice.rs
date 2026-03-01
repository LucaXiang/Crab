//! Invoice Service (Verifactu)
//!
//! Creates F2 invoices for completed orders and R5 invoices for credit notes,
//! maintaining the Verifactu huella (fingerprint) hash chain required by AEAT.

use super::service::{ArchiveError, ArchiveResult};
use crate::db::repository::invoice as inv_repo;
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use shared::cloud::sync::TaxDesglose;
use shared::models::invoice::{AeatStatus, Invoice, InvoiceSourceType, TipoFactura};
use shared::order::verifactu::{HuellaAltaInput, compute_verifactu_huella_alta};

/// Service for creating Verifactu invoices (F2/R5) with huella chain.
#[derive(Clone)]
pub struct InvoiceService {
    tz: chrono_tz::Tz,
    serie: String,
    nif: String,
    nombre_razon: String,
}

/// Convert Decimal to f64 for SQLite storage.
fn decimal_to_f64(value: Decimal, field: &str) -> ArchiveResult<f64> {
    value.to_f64().ok_or_else(|| {
        ArchiveError::InvoiceConversion(format!("Decimal→f64 overflow {field}: {value}"))
    })
}

impl InvoiceService {
    pub fn new(tz: chrono_tz::Tz, store_number: u32, nif: String, nombre_razon: String) -> Self {
        let serie = store_number_to_serie(store_number);
        Self {
            tz,
            serie,
            nif,
            nombre_razon,
        }
    }

    pub fn tz(&self) -> chrono_tz::Tz {
        self.tz
    }

    pub fn serie(&self) -> &str {
        &self.serie
    }

    pub fn nif(&self) -> &str {
        &self.nif
    }

    pub fn nombre_razon(&self) -> &str {
        &self.nombre_razon
    }

    /// Create an F2 invoice for a completed order.
    ///
    /// Returns `Ok(None)` if total <= 0 (comped orders skip invoicing).
    /// The caller must hold `hash_chain_lock` to prevent TOCTOU races on `last_huella`.
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

        // Read prev huella inside the transaction to prevent stale reads.
        // Caller must hold hash_chain_lock to serialize concurrent chain updates.
        let prev_huella: Option<String> =
            sqlx::query_scalar("SELECT last_huella FROM system_state WHERE id = 1")
                .fetch_optional(&mut **tx)
                .await
                .map_err(|e| ArchiveError::Database(e.to_string()))?
                .flatten();

        // Allocate invoice number
        let invoice_number = inv_repo::next_invoice_number(tx, &self.serie, &date_str)
            .await
            .map_err(|e| ArchiveError::InvoiceNumber(e.to_string()))?;

        // Compute huella (propagates HuellaError → ArchiveError::InvoiceConversion)
        let huella = compute_verifactu_huella_alta(&HuellaAltaInput {
            nif: &self.nif,
            invoice_number: &invoice_number,
            fecha_expedicion: &fecha_expedicion,
            tipo_factura: TipoFactura::F2.as_str(),
            cuota_total: tax,
            importe_total: total,
            prev_huella: prev_huella.as_deref(),
            fecha_hora_registro: &fecha_hora_registro,
        })?;

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
            fecha_hora_registro,
            nif: self.nif.clone(),
            nombre_razon: self.nombre_razon.clone(),
            factura_rectificada_id: None,
            factura_rectificada_num: None,
            factura_sustituida_id: None,
            factura_sustituida_num: None,
            customer_nif: None,
            customer_nombre: None,
            customer_address: None,
            customer_email: None,
            customer_phone: None,
            cloud_synced: false,
            aeat_status: AeatStatus::Pending,
            created_at: now,
        };

        let invoice_id = inv_repo::insert(tx, &invoice).await?;

        // Insert desglose lines (validate amounts are finite)
        for d in desglose {
            let base = decimal_to_f64(d.base_amount, "desglose.base_amount")?;
            let tax_amt = decimal_to_f64(d.tax_amount, "desglose.tax_amount")?;
            inv_repo::insert_desglose(tx, invoice_id, i64::from(d.tax_rate), base, tax_amt).await?;
        }

        // Update system_state.last_huella within the same transaction
        sqlx::query("UPDATE system_state SET last_huella = ?1, updated_at = ?2 WHERE id = 1")
            .bind(&huella)
            .bind(now)
            .execute(&mut **tx)
            .await
            .map_err(|e| ArchiveError::Database(e.to_string()))?;

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
    /// The caller must hold `hash_chain_lock` to prevent TOCTOU races on `last_huella`.
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
            .map_err(|e| ArchiveError::Database(e.to_string()))?;

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

        // Read prev huella inside the transaction
        let prev_huella: Option<String> =
            sqlx::query_scalar("SELECT last_huella FROM system_state WHERE id = 1")
                .fetch_optional(&mut **tx)
                .await
                .map_err(|e| ArchiveError::Database(e.to_string()))?
                .flatten();

        // Allocate invoice number
        let invoice_number = inv_repo::next_invoice_number(tx, &self.serie, &date_str)
            .await
            .map_err(|e| ArchiveError::InvoiceNumber(e.to_string()))?;

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
        })?;

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
            fecha_hora_registro,
            nif: self.nif.clone(),
            nombre_razon: self.nombre_razon.clone(),
            factura_rectificada_id: rectificada_id,
            factura_rectificada_num: rectificada_num,
            factura_sustituida_id: None,
            factura_sustituida_num: None,
            customer_nif: None,
            customer_nombre: None,
            customer_address: None,
            customer_email: None,
            customer_phone: None,
            cloud_synced: false,
            aeat_status: AeatStatus::Pending,
            created_at: now,
        };

        let invoice_id = inv_repo::insert(tx, &invoice).await?;

        // Insert desglose lines (validate amounts are finite)
        for d in desglose {
            let base = decimal_to_f64(d.base_amount, "desglose.base_amount")?;
            let tax_amt = decimal_to_f64(d.tax_amount, "desglose.tax_amount")?;
            inv_repo::insert_desglose(tx, invoice_id, i64::from(d.tax_rate), base, tax_amt).await?;
        }

        // Update system_state.last_huella within the same transaction
        sqlx::query("UPDATE system_state SET last_huella = ?1, updated_at = ?2 WHERE id = 1")
            .bind(&huella)
            .bind(now)
            .execute(&mut **tx)
            .await
            .map_err(|e| ArchiveError::Database(e.to_string()))?;

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
    use rust_decimal::prelude::FromPrimitive;

    // ── store_number_to_serie ───────────────────────────────────────

    #[test]
    fn store_number_to_serie_letters() {
        assert_eq!(store_number_to_serie(1), "A");
        assert_eq!(store_number_to_serie(2), "B");
        assert_eq!(store_number_to_serie(26), "Z");
    }

    #[test]
    fn store_number_to_serie_all_26_letters() {
        for n in 1..=26u32 {
            let s = store_number_to_serie(n);
            assert_eq!(s.len(), 1);
            let ch = s.chars().next().unwrap();
            assert!(ch.is_ascii_uppercase());
            assert_eq!(ch as u32 - 'A' as u32, n - 1);
        }
    }

    #[test]
    fn store_number_to_serie_fallback() {
        assert_eq!(store_number_to_serie(0), "S0");
        assert_eq!(store_number_to_serie(27), "S27");
        assert_eq!(store_number_to_serie(100), "S100");
    }

    #[test]
    fn store_number_to_serie_u32_max() {
        let s = store_number_to_serie(u32::MAX);
        assert!(s.starts_with('S'));
    }

    // ── decimal_to_f64 ─────────────────────────────────────────────

    #[test]
    fn decimal_to_f64_normal_values() {
        assert_eq!(
            decimal_to_f64(Decimal::from_f64(12.50).unwrap(), "test").unwrap(),
            12.5
        );
        assert_eq!(
            decimal_to_f64(Decimal::from_f64(0.0).unwrap(), "test").unwrap(),
            0.0
        );
        assert_eq!(
            decimal_to_f64(Decimal::from_f64(-99.99).unwrap(), "test").unwrap(),
            -99.99
        );
        assert_eq!(
            decimal_to_f64(Decimal::from_f64(0.01).unwrap(), "test").unwrap(),
            0.01
        );
    }

    #[test]
    fn decimal_to_f64_large_value() {
        // Large but representable in f64
        let big = Decimal::from_f64(1e15).unwrap();
        assert!(decimal_to_f64(big, "big").is_ok());
    }

    #[test]
    fn decimal_to_f64_max_decimal_still_converts() {
        // Decimal::MAX.to_f64() returns Some — it's within f64 range (just imprecise)
        let result = decimal_to_f64(Decimal::MAX, "max");
        assert!(result.is_ok());
    }

    #[test]
    fn decimal_to_f64_error_preserves_field_name() {
        // Directly test the error message format
        let err = ArchiveError::InvoiceConversion(
            "Decimal→f64 overflow desglose.base_amount: 999".to_string(),
        );
        let msg = format!("{err}");
        assert!(msg.contains("desglose.base_amount"));
    }

    #[test]
    fn decimal_to_f64_negative_zero() {
        // Decimal doesn't have negative zero, but -0.0 in f64 == 0.0
        let result = decimal_to_f64(Decimal::from_f64(0.0).unwrap(), "z").unwrap();
        assert_eq!(result, 0.0);
    }

    // ── InvoiceService::new ────────────────────────────────────────

    #[test]
    fn invoice_service_new_stores_serie_and_nif() {
        let svc = InvoiceService::new(
            chrono_tz::Europe::Madrid,
            3,
            "B12345678".to_string(),
            "Test SL".to_string(),
        );
        assert_eq!(svc.serie, "C");
        assert_eq!(svc.nif, "B12345678");
        assert_eq!(svc.nombre_razon, "Test SL");
    }

    #[test]
    fn invoice_service_new_fallback_serie() {
        let svc = InvoiceService::new(
            chrono_tz::Europe::Madrid,
            0,
            "X00000000".to_string(),
            "Fallback".to_string(),
        );
        assert_eq!(svc.serie, "S0");
    }
}
