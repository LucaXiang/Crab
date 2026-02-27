//! Verifactu huella (fingerprint) computation per AEAT Registro de Alta spec.
//!
//! Implements the SHA-256 hash chain required by Spanish tax authorities
//! for invoice integrity verification.

use rust_decimal::Decimal;
use rust_decimal::prelude::FromPrimitive;
use sha2::{Digest, Sha256};

/// Format an f64 amount for Verifactu, removing trailing zeros.
///
/// Examples: 123.10 → "123.1", 100.00 → "100", 99.99 → "99.99", 0.0 → "0"
pub fn format_amount(value: f64) -> String {
    Decimal::from_f64(value)
        .expect("f64 must be convertible to Decimal")
        .normalize()
        .to_string()
}

/// Input fields for computing a Verifactu Registro de Alta huella.
pub struct HuellaAltaInput<'a> {
    pub nif: &'a str,
    pub invoice_number: &'a str,
    /// Date in DD-MM-YYYY format.
    pub fecha_expedicion: &'a str,
    /// Invoice type code (e.g. "F2", "R5").
    pub tipo_factura: &'a str,
    pub cuota_total: f64,
    pub importe_total: f64,
    /// Previous huella in the chain. `None` for the first record.
    pub prev_huella: Option<&'a str>,
    /// ISO 8601 timestamp with timezone.
    pub fecha_hora_registro: &'a str,
}

/// Compute the Verifactu Registro de Alta huella (fingerprint hash).
///
/// Concatenates invoice fields per AEAT spec and produces a SHA-256 hex digest.
/// For the first record in a chain, set `prev_huella = None` (empty Huella= value).
///
/// Returns a lowercase 64-character hex string.
pub fn compute_verifactu_huella_alta(input: &HuellaAltaInput<'_>) -> String {
    let cuota_str = format_amount(input.cuota_total);
    let importe_str = format_amount(input.importe_total);
    let huella_value = input.prev_huella.unwrap_or("");

    let concat = format!(
        "IDEmisorFactura={}&NumSerieFactura={}&FechaExpedicionFactura={}&TipoFactura={}&CuotaTotal={}&ImporteTotal={}&Huella={}&FechaHoraHusoGenRegistro={}",
        input.nif,
        input.invoice_number,
        input.fecha_expedicion,
        input.tipo_factura,
        cuota_str,
        importe_str,
        huella_value,
        input.fecha_hora_registro
    );

    let digest = Sha256::digest(concat.as_bytes());
    format!("{:x}", digest)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};

    fn make_input<'a>(
        cuota: f64,
        importe: f64,
        invoice: &'a str,
        prev: Option<&'a str>,
        ts: &'a str,
    ) -> HuellaAltaInput<'a> {
        HuellaAltaInput {
            nif: "B12345678",
            invoice_number: invoice,
            fecha_expedicion: "27-02-2026",
            tipo_factura: "F2",
            cuota_total: cuota,
            importe_total: importe,
            prev_huella: prev,
            fecha_hora_registro: ts,
        }
    }

    #[test]
    fn format_amount_removes_trailing_zeros() {
        assert_eq!(format_amount(123.10), "123.1");
        assert_eq!(format_amount(100.00), "100");
        assert_eq!(format_amount(99.99), "99.99");
        assert_eq!(format_amount(0.0), "0");
        assert_eq!(format_amount(5.50), "5.5");
    }

    #[test]
    fn huella_is_deterministic_and_64_chars() {
        let input = make_input(2.10, 12.10, "INV-001", None, "2026-02-27T10:00:00+01:00");
        let h1 = compute_verifactu_huella_alta(&input);
        let h2 = compute_verifactu_huella_alta(&input);
        assert_eq!(h1.len(), 64);
        assert_eq!(h1, h2);
    }

    #[test]
    fn first_record_uses_empty_huella() {
        // Manually compute expected hash for first record (empty Huella=)
        let raw = "IDEmisorFactura=B12345678&NumSerieFactura=INV-001&FechaExpedicionFactura=27-02-2026&TipoFactura=F2&CuotaTotal=2.1&ImporteTotal=12.1&Huella=&FechaHoraHusoGenRegistro=2026-02-27T10:00:00+01:00";
        let expected = format!("{:x}", Sha256::digest(raw.as_bytes()));

        let input = make_input(2.10, 12.10, "INV-001", None, "2026-02-27T10:00:00+01:00");
        let actual = compute_verifactu_huella_alta(&input);
        assert_eq!(actual, expected);
    }

    #[test]
    fn chained_record_includes_prev_huella() {
        let first_input = make_input(2.10, 12.10, "INV-001", None, "2026-02-27T10:00:00+01:00");
        let first = compute_verifactu_huella_alta(&first_input);

        // Manually compute expected hash for chained record
        let raw = format!(
            "IDEmisorFactura=B12345678&NumSerieFactura=INV-002&FechaExpedicionFactura=27-02-2026&TipoFactura=F2&CuotaTotal=5&ImporteTotal=25&Huella={}&FechaHoraHusoGenRegistro=2026-02-27T11:00:00+01:00",
            first
        );
        let expected = format!("{:x}", Sha256::digest(raw.as_bytes()));

        let chained_input = make_input(
            5.00,
            25.00,
            "INV-002",
            Some(&first),
            "2026-02-27T11:00:00+01:00",
        );
        let actual = compute_verifactu_huella_alta(&chained_input);
        assert_eq!(actual, expected);
        assert_ne!(actual, first);
    }

    #[test]
    fn different_amounts_produce_different_hashes() {
        let input1 = make_input(2.10, 12.10, "INV-001", None, "2026-02-27T10:00:00+01:00");
        let input2 = make_input(3.15, 15.15, "INV-001", None, "2026-02-27T10:00:00+01:00");
        let h1 = compute_verifactu_huella_alta(&input1);
        let h2 = compute_verifactu_huella_alta(&input2);
        assert_ne!(h1, h2);
    }
}
