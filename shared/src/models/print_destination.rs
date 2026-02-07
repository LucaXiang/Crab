//! Print Destination Model

use serde::{Deserialize, Serialize};

/// Printer entity (independent table, was EmbeddedPrinter)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "db", derive(sqlx::FromRow))]
pub struct Printer {
    pub id: i64,
    pub print_destination_id: i64,
    pub printer_type: String,
    /// Printer format: "escpos" (厨房单/小票) | "label" (标签)
    #[serde(default = "default_printer_format")]
    pub printer_format: String,
    pub ip: Option<String>,
    pub port: Option<i32>,
    pub driver_name: Option<String>,
    pub priority: i32,
    pub is_active: bool,
}

fn default_printer_format() -> String {
    "escpos".to_string()
}

/// Print destination entity
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "db", derive(sqlx::FromRow))]
pub struct PrintDestination {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub is_active: bool,

    // -- Relations (populated by application code, skipped by FromRow) --

    #[cfg_attr(feature = "db", sqlx(skip))]
    #[serde(default)]
    pub printers: Vec<Printer>,
}

/// Printer input (for create/update, without id/print_destination_id)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrinterInput {
    pub printer_type: String,
    #[serde(default = "default_printer_format")]
    pub printer_format: String,
    pub ip: Option<String>,
    pub port: Option<i32>,
    pub driver_name: Option<String>,
    #[serde(default)]
    pub priority: i32,
    #[serde(default = "default_true")]
    pub is_active: bool,
}

fn default_true() -> bool {
    true
}

/// Create print destination payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrintDestinationCreate {
    pub name: String,
    pub description: Option<String>,
    #[serde(default)]
    pub printers: Vec<PrinterInput>,
    #[serde(default = "default_true")]
    pub is_active: bool,
}

/// Update print destination payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrintDestinationUpdate {
    pub name: Option<String>,
    pub description: Option<String>,
    pub printers: Option<Vec<PrinterInput>>,
    pub is_active: Option<bool>,
}
