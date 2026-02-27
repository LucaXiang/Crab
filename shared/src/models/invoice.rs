//! Verifactu Invoice (factura) Model

use serde::{Deserialize, Serialize};

/// Tipo de factura según Verifactu: F2 (simplified) or R5 (rectificativa simplificada)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TipoFactura {
    F2,
    R5,
}

impl TipoFactura {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::F2 => "F2",
            Self::R5 => "R5",
        }
    }
}

impl std::str::FromStr for TipoFactura {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "F2" => Ok(Self::F2),
            "R5" => Ok(Self::R5),
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
}

impl InvoiceSourceType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Order => "ORDER",
            Self::CreditNote => "CREDIT_NOTE",
        }
    }
}

/// AEAT submission status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
    pub nif: String,
    pub nombre_razon: String,

    // Rectificativa reference
    pub factura_rectificada_id: Option<i64>,
    pub factura_rectificada_num: Option<String>,

    // Sync / status
    pub cloud_synced: bool,
    pub aeat_status: AeatStatus,

    pub created_at: i64,
}

/// Invoice tax breakdown line (desglose)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "db", derive(sqlx::FromRow))]
pub struct InvoiceDesglose {
    pub id: i64,
    pub invoice_id: i64,
    /// Tax rate in basis points (e.g. 2100 = 21%)
    pub tax_rate: i64,
    pub base_amount: f64,
    pub tax_amount: f64,
}
