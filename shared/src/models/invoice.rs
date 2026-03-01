//! Verifactu Invoice (factura) Model

use serde::{Deserialize, Serialize};

/// Tipo de factura según Verifactu: F2 (simplified), R5 (rectificativa simplificada), F3 (sustitutiva)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TipoFactura {
    F2,
    R5,
    F3,
}

impl TipoFactura {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::F2 => "F2",
            Self::R5 => "R5",
            Self::F3 => "F3",
        }
    }
}

impl std::str::FromStr for TipoFactura {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "F2" => Ok(Self::F2),
            "R5" => Ok(Self::R5),
            "F3" => Ok(Self::F3),
            _ => Err(()),
        }
    }
}

impl std::fmt::Display for TipoFactura {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Source type for an invoice (what generated it)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum InvoiceSourceType {
    Order,
    CreditNote,
    Upgrade,
}

impl InvoiceSourceType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Order => "ORDER",
            Self::CreditNote => "CREDIT_NOTE",
            Self::Upgrade => "UPGRADE",
        }
    }
}

/// AEAT submission status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AeatStatus {
    Pending,
    Submitted,
    Accepted,
    Rejected,
}

impl AeatStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "PENDING",
            Self::Submitted => "SUBMITTED",
            Self::Accepted => "ACCEPTED",
            Self::Rejected => "REJECTED",
        }
    }
}

impl std::str::FromStr for InvoiceSourceType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "ORDER" => Ok(Self::Order),
            "CREDIT_NOTE" => Ok(Self::CreditNote),
            "UPGRADE" => Ok(Self::Upgrade),
            _ => Err(()),
        }
    }
}

impl std::fmt::Display for InvoiceSourceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for AeatStatus {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "PENDING" => Ok(Self::Pending),
            "SUBMITTED" => Ok(Self::Submitted),
            "ACCEPTED" => Ok(Self::Accepted),
            "REJECTED" => Ok(Self::Rejected),
            _ => Err(()),
        }
    }
}

impl std::fmt::Display for AeatStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Verifactu invoice entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Invoice {
    pub id: i64,
    pub invoice_number: String,
    pub serie: String,
    pub tipo_factura: TipoFactura,
    pub source_type: InvoiceSourceType,
    pub source_pk: i64,

    // Amounts
    pub subtotal: f64,
    pub tax: f64,
    pub total: f64,

    // Hash chain (huella)
    pub huella: String,
    pub prev_huella: Option<String>,

    // Issuer info
    pub fecha_expedicion: String,
    /// RFC 3339 timestamp used in huella computation
    pub fecha_hora_registro: String,
    pub nif: String,
    pub nombre_razon: String,

    // Rectificativa reference (R5)
    pub factura_rectificada_id: Option<i64>,
    pub factura_rectificada_num: Option<String>,

    // Sustitutiva reference (F3 replaces F2)
    pub factura_sustituida_id: Option<i64>,
    pub factura_sustituida_num: Option<String>,

    // Customer info (F3 only)
    pub customer_nif: Option<String>,
    pub customer_nombre: Option<String>,
    pub customer_address: Option<String>,
    pub customer_email: Option<String>,
    pub customer_phone: Option<String>,

    // Sync / status
    pub cloud_synced: bool,
    pub aeat_status: AeatStatus,

    pub created_at: i64,
}

// ── Anulación (RegistroFacturaBaja) ──────────────────────────

/// Reason for voiding an invoice via Anulación
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AnulacionReason {
    TestOrder,
    WrongCustomer,
    Duplicate,
    Other,
}

impl AnulacionReason {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::TestOrder => "TEST_ORDER",
            Self::WrongCustomer => "WRONG_CUSTOMER",
            Self::Duplicate => "DUPLICATE",
            Self::Other => "OTHER",
        }
    }
}

impl std::str::FromStr for AnulacionReason {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "TEST_ORDER" => Ok(Self::TestOrder),
            "WRONG_CUSTOMER" => Ok(Self::WrongCustomer),
            "DUPLICATE" => Ok(Self::Duplicate),
            "OTHER" => Ok(Self::Other),
            _ => Err(()),
        }
    }
}

impl std::fmt::Display for AnulacionReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Verifactu invoice anulación (RegistroFacturaBaja)
///
/// Represents the legal revocation of an invoice — not a refund (R5),
/// but a declaration that the invoice should never have existed.
/// Use cases: test orders, wrong customer, duplicate invoices.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvoiceAnulacion {
    pub id: i64,
    pub anulacion_number: String,
    pub serie: String,

    /// Original F2 invoice being voided
    pub original_invoice_id: i64,
    pub original_invoice_number: String,

    /// Huella chain (shared with Alta F2/R5)
    pub huella: String,
    pub prev_huella: Option<String>,

    /// AEAT-required fields
    pub fecha_expedicion: String,
    pub fecha_hora_registro: String,
    pub nif: String,
    pub nombre_razon: String,

    /// Order reference
    pub original_order_pk: i64,

    /// Reason and audit
    pub reason: AnulacionReason,
    pub note: Option<String>,
    pub operator_id: i64,
    pub operator_name: String,

    /// Sync status
    pub cloud_synced: bool,
    pub aeat_status: AeatStatus,
    pub created_at: i64,
}

/// Invoice tax breakdown line (desglose) — SQLite-specific struct.
///
/// Reads from SQLite `invoice_desglose` table with f64 values.
/// Convert to `TaxDesglose` (Decimal) at the sync boundary via `into_tax_desglose()`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "db", derive(sqlx::FromRow))]
pub struct InvoiceDesglose {
    pub id: i64,
    pub invoice_id: i64,
    /// Tax rate in basis points (e.g. 2100 = 21%)
    pub tax_rate: i32,
    pub base_amount: f64,
    pub tax_amount: f64,
}

impl InvoiceDesglose {
    /// Convert to `TaxDesglose` for cloud sync (f64 → Decimal).
    pub fn into_tax_desglose(self) -> crate::cloud::sync::TaxDesglose {
        use rust_decimal::Decimal;
        use rust_decimal::prelude::FromPrimitive;
        crate::cloud::sync::TaxDesglose {
            tax_rate: self.tax_rate,
            base_amount: Decimal::from_f64(self.base_amount).unwrap_or_default(),
            tax_amount: Decimal::from_f64(self.tax_amount).unwrap_or_default(),
        }
    }
}
