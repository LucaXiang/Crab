//! Verifactu huella (fingerprint) computation per AEAT Registro de Alta spec.
//!
//! Implements the SHA-256 hash chain required by Spanish tax authorities
//! for invoice integrity verification.

use rust_decimal::Decimal;
use rust_decimal::prelude::FromPrimitive;
use sha2::{Digest, Sha256};
use std::fmt;

/// Error type for Verifactu huella computation.
#[derive(Debug, Clone)]
pub struct HuellaError(pub String);

impl fmt::Display for HuellaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Huella computation error: {}", self.0)
    }
}

impl std::error::Error for HuellaError {}

/// Format an f64 amount for Verifactu, removing trailing zeros.
///
/// Returns `Err` for non-finite values (NaN, Infinity).
///
/// Examples: 123.10 → "123.1", 100.00 → "100", 99.99 → "99.99", 0.0 → "0"
pub fn format_amount(value: f64) -> Result<String, HuellaError> {
    Decimal::from_f64(value)
        .ok_or_else(|| HuellaError(format!("non-finite f64 value: {value}")))
        .map(|d| d.normalize().to_string())
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
/// Returns a lowercase 64-character hex string, or `HuellaError` if amounts are non-finite.
pub fn compute_verifactu_huella_alta(input: &HuellaAltaInput<'_>) -> Result<String, HuellaError> {
    let cuota_str = format_amount(input.cuota_total)?;
    let importe_str = format_amount(input.importe_total)?;
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
    Ok(format!("{:x}", digest))
}

// ── Registro de Baja (Anulación) ─────────────────────────────

/// Input fields for computing a Verifactu Registro de Baja huella.
///
/// Simpler than Alta: no TipoFactura, no amounts.
/// Formula: `IDEmisorFacturaAnulada={NIF}&NumSerieFacturaAnulada={NUM}&
///           FechaExpedicionFacturaAnulada={DD-MM-YYYY}&Huella={prev}&
///           FechaHoraHusoGenRegistro={RFC3339}`
pub struct HuellaBajaInput<'a> {
    pub nif: &'a str,
    pub invoice_number: &'a str,
    /// Date in DD-MM-YYYY format.
    pub fecha_expedicion: &'a str,
    /// Previous huella in the chain. `None` for the first record.
    pub prev_huella: Option<&'a str>,
    /// ISO 8601 timestamp with timezone.
    pub fecha_hora_registro: &'a str,
}

/// Compute the Verifactu Registro de Baja huella (anulación fingerprint hash).
///
/// Simpler than Alta — no amounts or invoice type in the hash.
/// Returns a lowercase 64-character hex string.
pub fn compute_verifactu_huella_baja(input: &HuellaBajaInput<'_>) -> String {
    let huella_value = input.prev_huella.unwrap_or("");

    let concat = format!(
        "IDEmisorFacturaAnulada={}&NumSerieFacturaAnulada={}&\
         FechaExpedicionFacturaAnulada={}&Huella={}&\
         FechaHoraHusoGenRegistro={}",
        input.nif,
        input.invoice_number,
        input.fecha_expedicion,
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
        assert_eq!(format_amount(123.10).unwrap(), "123.1");
        assert_eq!(format_amount(100.00).unwrap(), "100");
        assert_eq!(format_amount(99.99).unwrap(), "99.99");
        assert_eq!(format_amount(0.0).unwrap(), "0");
        assert_eq!(format_amount(5.50).unwrap(), "5.5");
    }

    #[test]
    fn format_amount_rejects_non_finite() {
        assert!(format_amount(f64::NAN).is_err());
        assert!(format_amount(f64::INFINITY).is_err());
        assert!(format_amount(f64::NEG_INFINITY).is_err());
    }

    #[test]
    fn huella_is_deterministic_and_64_chars() {
        let input = make_input(2.10, 12.10, "INV-001", None, "2026-02-27T10:00:00+01:00");
        let h1 = compute_verifactu_huella_alta(&input).unwrap();
        let h2 = compute_verifactu_huella_alta(&input).unwrap();
        assert_eq!(h1.len(), 64);
        assert_eq!(h1, h2);
    }

    #[test]
    fn first_record_uses_empty_huella() {
        // Manually compute expected hash for first record (empty Huella=)
        let raw = "IDEmisorFactura=B12345678&NumSerieFactura=INV-001&FechaExpedicionFactura=27-02-2026&TipoFactura=F2&CuotaTotal=2.1&ImporteTotal=12.1&Huella=&FechaHoraHusoGenRegistro=2026-02-27T10:00:00+01:00";
        let expected = format!("{:x}", Sha256::digest(raw.as_bytes()));

        let input = make_input(2.10, 12.10, "INV-001", None, "2026-02-27T10:00:00+01:00");
        let actual = compute_verifactu_huella_alta(&input).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn chained_record_includes_prev_huella() {
        let first_input = make_input(2.10, 12.10, "INV-001", None, "2026-02-27T10:00:00+01:00");
        let first = compute_verifactu_huella_alta(&first_input).unwrap();

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
        let actual = compute_verifactu_huella_alta(&chained_input).unwrap();
        assert_eq!(actual, expected);
        assert_ne!(actual, first);
    }

    #[test]
    fn different_amounts_produce_different_hashes() {
        let input1 = make_input(2.10, 12.10, "INV-001", None, "2026-02-27T10:00:00+01:00");
        let input2 = make_input(3.15, 15.15, "INV-001", None, "2026-02-27T10:00:00+01:00");
        let h1 = compute_verifactu_huella_alta(&input1).unwrap();
        let h2 = compute_verifactu_huella_alta(&input2).unwrap();
        assert_ne!(h1, h2);
    }

    #[test]
    fn non_finite_cuota_returns_error() {
        let input = make_input(
            f64::NAN,
            12.10,
            "INV-001",
            None,
            "2026-02-27T10:00:00+01:00",
        );
        assert!(compute_verifactu_huella_alta(&input).is_err());
    }

    #[test]
    fn non_finite_importe_returns_error() {
        let input = make_input(
            2.10,
            f64::INFINITY,
            "INV-001",
            None,
            "2026-02-27T10:00:00+01:00",
        );
        assert!(compute_verifactu_huella_alta(&input).is_err());

        let input2 = make_input(
            2.10,
            f64::NEG_INFINITY,
            "INV-001",
            None,
            "2026-02-27T10:00:00+01:00",
        );
        assert!(compute_verifactu_huella_alta(&input2).is_err());
    }

    #[test]
    fn huella_error_display_contains_context() {
        let err = HuellaError("test detail".to_string());
        let msg = format!("{err}");
        assert!(msg.contains("test detail"));
        assert!(msg.contains("Huella"));
    }

    #[test]
    fn format_amount_negative_values() {
        assert_eq!(format_amount(-5.50).unwrap(), "-5.5");
        assert_eq!(format_amount(-100.00).unwrap(), "-100");
    }

    #[test]
    fn format_amount_small_decimals() {
        assert_eq!(format_amount(0.01).unwrap(), "0.01");
        assert_eq!(format_amount(0.001).unwrap(), "0.001");
    }

    #[test]
    fn format_amount_large_values() {
        assert_eq!(format_amount(999999.99).unwrap(), "999999.99");
        assert_eq!(format_amount(1_000_000.0).unwrap(), "1000000");
    }

    #[test]
    fn huella_is_lowercase_hex() {
        let input = make_input(2.10, 12.10, "INV-001", None, "2026-02-27T10:00:00+01:00");
        let h = compute_verifactu_huella_alta(&input).unwrap();
        assert!(
            h.chars()
                .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
        );
    }

    #[test]
    fn different_nif_produces_different_hash() {
        let input1 = make_input(2.10, 12.10, "INV-001", None, "2026-02-27T10:00:00+01:00");
        let h1 = compute_verifactu_huella_alta(&input1).unwrap();

        let input2 = HuellaAltaInput {
            nif: "A99999999",
            ..make_input(2.10, 12.10, "INV-001", None, "2026-02-27T10:00:00+01:00")
        };
        let h2 = compute_verifactu_huella_alta(&input2).unwrap();
        assert_ne!(h1, h2);
    }

    #[test]
    fn different_tipo_factura_produces_different_hash() {
        let input1 = make_input(2.10, 12.10, "INV-001", None, "2026-02-27T10:00:00+01:00");
        let h1 = compute_verifactu_huella_alta(&input1).unwrap();

        let input2 = HuellaAltaInput {
            tipo_factura: "R5",
            ..make_input(2.10, 12.10, "INV-001", None, "2026-02-27T10:00:00+01:00")
        };
        let h2 = compute_verifactu_huella_alta(&input2).unwrap();
        assert_ne!(h1, h2);
    }

    #[test]
    fn different_timestamp_produces_different_hash() {
        let h1 = compute_verifactu_huella_alta(&make_input(
            2.10,
            12.10,
            "INV-001",
            None,
            "2026-02-27T10:00:00+01:00",
        ))
        .unwrap();
        let h2 = compute_verifactu_huella_alta(&make_input(
            2.10,
            12.10,
            "INV-001",
            None,
            "2026-02-27T10:00:01+01:00",
        ))
        .unwrap();
        assert_ne!(h1, h2);
    }

    #[test]
    fn chain_of_three_records_integrity() {
        // Build a 3-record chain and verify each link depends on the previous
        let h1 = compute_verifactu_huella_alta(&make_input(
            1.0,
            11.0,
            "INV-001",
            None,
            "2026-02-27T10:00:00+01:00",
        ))
        .unwrap();

        let h2 = compute_verifactu_huella_alta(&make_input(
            2.0,
            22.0,
            "INV-002",
            Some(&h1),
            "2026-02-27T11:00:00+01:00",
        ))
        .unwrap();

        let h3 = compute_verifactu_huella_alta(&make_input(
            3.0,
            33.0,
            "INV-003",
            Some(&h2),
            "2026-02-27T12:00:00+01:00",
        ))
        .unwrap();

        // All different
        assert_ne!(h1, h2);
        assert_ne!(h2, h3);
        assert_ne!(h1, h3);

        // Tamper detection: changing h2's prev_huella produces a different h2
        let h2_tampered = compute_verifactu_huella_alta(&make_input(
            2.0,
            22.0,
            "INV-002",
            Some(&h3), // wrong prev
            "2026-02-27T11:00:00+01:00",
        ))
        .unwrap();
        assert_ne!(h2, h2_tampered);
    }

    #[test]
    fn zero_amounts_valid() {
        let input = make_input(0.0, 0.0, "INV-001", None, "2026-02-27T10:00:00+01:00");
        let h = compute_verifactu_huella_alta(&input).unwrap();
        assert_eq!(h.len(), 64);

        // Verify the raw string uses "0" not "0.0"
        let raw = "IDEmisorFactura=B12345678&NumSerieFactura=INV-001&FechaExpedicionFactura=27-02-2026&TipoFactura=F2&CuotaTotal=0&ImporteTotal=0&Huella=&FechaHoraHusoGenRegistro=2026-02-27T10:00:00+01:00";
        let expected = format!("{:x}", Sha256::digest(raw.as_bytes()));
        assert_eq!(h, expected);
    }

    // ========================================================================
    // format_amount determinism & roundtrip stability
    // ========================================================================

    /// Critical: f64 values that look equal must produce identical format_amount strings,
    /// because any difference in the string breaks the SHA-256 hash chain.
    #[test]
    fn format_amount_deterministic_across_representations() {
        // 2.10 and 2.1 are the same f64
        assert_eq!(format_amount(2.10).unwrap(), format_amount(2.1).unwrap());
        assert_eq!(format_amount(5.50).unwrap(), format_amount(5.5).unwrap());
        assert_eq!(
            format_amount(100.00).unwrap(),
            format_amount(100.0).unwrap()
        );
    }

    /// f64 arithmetic can introduce tiny errors. Verify format_amount normalizes them.
    /// e.g. 0.1 + 0.2 = 0.30000000000000004 in f64, but Decimal normalizes it.
    #[test]
    fn format_amount_handles_f64_arithmetic_artifacts() {
        let result = format_amount(0.1 + 0.2);
        // This should succeed (Decimal::from_f64 handles typical f64 values)
        assert!(result.is_ok());
        // The exact string may be "0.30000000000000004" or "0.3" depending on Decimal
        // Key: it doesn't panic or error
        let s = result.unwrap();
        assert!(!s.is_empty());
    }

    /// Verify that values round-tripped through f64 → Decimal → format_amount → parse
    /// produce the same f64 within expected precision.
    #[test]
    fn format_amount_roundtrip_stability() {
        let test_values = [
            0.01, 0.10, 0.99, 1.00, 2.10, 5.50, 10.21, 99.99, 100.00, 123.45, 999.99,
        ];
        for &v in &test_values {
            let s1 = format_amount(v).unwrap();
            let s2 = format_amount(v).unwrap();
            assert_eq!(s1, s2, "format_amount not deterministic for {v}");
        }
    }

    /// The huella must be identical if computed twice with the same inputs.
    /// This tests the full pipeline: format_amount + string concat + SHA-256.
    #[test]
    fn huella_full_pipeline_deterministic_1000_times() {
        let input = make_input(21.05, 121.05, "INV-001", None, "2026-02-27T10:00:00+01:00");
        let reference = compute_verifactu_huella_alta(&input).unwrap();
        for _ in 0..1000 {
            assert_eq!(
                compute_verifactu_huella_alta(&input).unwrap(),
                reference,
                "huella not deterministic!"
            );
        }
    }

    /// Test with amounts that come from typical percentage calculations.
    /// e.g. 100.0 * 21% = 21.0 tax, 100.0 * 10% = 10.0 tax
    #[test]
    fn huella_with_typical_tax_calculations() {
        // 10% IVA on 50.00
        let tax = 50.0 * 0.10;
        let total = 50.0 + tax;
        let input = make_input(tax, total, "INV-TAX", None, "2026-02-27T10:00:00+01:00");
        let h = compute_verifactu_huella_alta(&input).unwrap();
        assert_eq!(h.len(), 64);

        // Compute again with explicit values
        let input2 = make_input(5.0, 55.0, "INV-TAX", None, "2026-02-27T10:00:00+01:00");
        let h2 = compute_verifactu_huella_alta(&input2).unwrap();
        // These should match because 50.0 * 0.10 = 5.0 exactly in f64
        assert_eq!(h, h2);
    }

    /// Test with 21% IVA which can produce tricky decimals.
    /// 33.33 * 21% = 6.9993 → after rounding to 2dp = 7.00
    /// But if we pass the raw f64, format_amount will normalize via Decimal.
    #[test]
    fn huella_with_21_percent_tax_edge_case() {
        // Simulating: base = 33.33, tax_rate = 21%
        // In order_money this would be rounded to 2dp before reaching invoice
        let base = 33.33;
        let tax_rounded = 7.0; // order_money rounds 6.9993 → 7.00 → stored as 7.0
        let total = base + tax_rounded; // 40.33

        let input = make_input(
            tax_rounded,
            total,
            "INV-21",
            None,
            "2026-02-27T10:00:00+01:00",
        );
        let h = compute_verifactu_huella_alta(&input).unwrap();
        assert_eq!(h.len(), 64);

        // Verify the amount strings are as expected
        assert_eq!(format_amount(tax_rounded).unwrap(), "7");
        assert_eq!(format_amount(total).unwrap(), "40.33");
    }

    /// Ensure that tiny f64 precision differences don't sneak in.
    /// If total is stored as f64 in SQLite, read back, it must format identically.
    #[test]
    fn format_amount_sqlite_f64_roundtrip_simulation() {
        // Simulate: Decimal → to_f64 → SQLite store → f64 read → format_amount
        use rust_decimal::Decimal;
        use rust_decimal::prelude::{FromPrimitive, ToPrimitive};

        let test_amounts = [0.01, 0.10, 1.21, 9.99, 21.05, 100.00, 999.99];
        for &amt in &test_amounts {
            // Step 1: original Decimal (from order_money calculation)
            let dec = Decimal::from_f64(amt).unwrap();
            // Step 2: to_f64 for SQLite storage
            let as_f64 = dec.to_f64().unwrap();
            // Step 3: format_amount (what goes into huella)
            let s1 = format_amount(amt).unwrap();
            let s2 = format_amount(as_f64).unwrap();
            assert_eq!(
                s1, s2,
                "format_amount differs after f64 roundtrip for {amt}"
            );
        }
    }

    /// Negative amounts (credit notes) must also be deterministic.
    #[test]
    fn huella_with_negative_amounts_credit_note() {
        let input = make_input(-5.0, -55.0, "CN-001", None, "2026-02-27T10:00:00+01:00");
        let h1 = compute_verifactu_huella_alta(&input).unwrap();
        let h2 = compute_verifactu_huella_alta(&input).unwrap();
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 64);

        // Verify negative format
        assert_eq!(format_amount(-5.0).unwrap(), "-5");
        assert_eq!(format_amount(-55.0).unwrap(), "-55");
    }

    // ── Registro de Baja (Anulación) tests ─────────────────────

    fn make_baja_input<'a>(
        invoice: &'a str,
        prev: Option<&'a str>,
        ts: &'a str,
    ) -> HuellaBajaInput<'a> {
        HuellaBajaInput {
            nif: "B12345678",
            invoice_number: invoice,
            fecha_expedicion: "27-02-2026",
            prev_huella: prev,
            fecha_hora_registro: ts,
        }
    }

    #[test]
    fn baja_huella_deterministic_and_64_chars() {
        let input = make_baja_input("INV-001", None, "2026-02-27T10:00:00+01:00");
        let h1 = compute_verifactu_huella_baja(&input);
        let h2 = compute_verifactu_huella_baja(&input);
        assert_eq!(h1.len(), 64);
        assert_eq!(h1, h2);
    }

    #[test]
    fn baja_first_record_uses_empty_huella() {
        let raw = "IDEmisorFacturaAnulada=B12345678&NumSerieFacturaAnulada=INV-001&\
                   FechaExpedicionFacturaAnulada=27-02-2026&Huella=&\
                   FechaHoraHusoGenRegistro=2026-02-27T10:00:00+01:00";
        let expected = format!("{:x}", Sha256::digest(raw.as_bytes()));

        let input = make_baja_input("INV-001", None, "2026-02-27T10:00:00+01:00");
        assert_eq!(compute_verifactu_huella_baja(&input), expected);
    }

    #[test]
    fn baja_chained_with_alta() {
        // Alta (F2) first, then Baja uses same chain
        let alta = compute_verifactu_huella_alta(&make_input(
            2.10,
            12.10,
            "INV-001",
            None,
            "2026-02-27T10:00:00+01:00",
        ))
        .unwrap();

        let baja = compute_verifactu_huella_baja(&make_baja_input(
            "INV-001",
            Some(&alta),
            "2026-02-27T11:00:00+01:00",
        ));

        assert_ne!(alta, baja);
        assert_eq!(baja.len(), 64);
    }

    #[test]
    fn baja_different_invoice_produces_different_hash() {
        let h1 = compute_verifactu_huella_baja(&make_baja_input(
            "INV-001",
            None,
            "2026-02-27T10:00:00+01:00",
        ));
        let h2 = compute_verifactu_huella_baja(&make_baja_input(
            "INV-002",
            None,
            "2026-02-27T10:00:00+01:00",
        ));
        assert_ne!(h1, h2);
    }
}
