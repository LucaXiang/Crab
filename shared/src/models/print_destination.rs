//! Print Destination Model

use serde::{Deserialize, Serialize};

/// Printer entity (physical device under a print destination)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "db", derive(sqlx::FromRow))]
pub struct Printer {
    pub id: i64,
    pub print_destination_id: i64,
    /// Physical connection method: "network" | "driver"
    pub connection: String,
    /// Communication protocol: "escpos" | "tspl"
    #[serde(default = "default_protocol")]
    pub protocol: String,
    pub ip: Option<String>,
    pub port: Option<i32>,
    pub driver_name: Option<String>,
    pub priority: i32,
    pub is_active: bool,
}

fn default_protocol() -> String {
    "escpos".to_string()
}

/// Print destination entity (logical print station)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "db", derive(sqlx::FromRow))]
pub struct PrintDestination {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    /// Purpose: "kitchen" | "label"
    #[serde(default = "default_purpose")]
    pub purpose: String,
    pub is_active: bool,

    // -- Relations (populated by application code, skipped by FromRow) --

    #[cfg_attr(feature = "db", sqlx(skip))]
    #[serde(default)]
    pub printers: Vec<Printer>,
}

fn default_purpose() -> String {
    "kitchen".to_string()
}

/// Printer input (for create/update, without id/print_destination_id)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrinterInput {
    pub connection: String,
    #[serde(default = "default_protocol")]
    pub protocol: String,
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
    #[serde(default = "default_purpose")]
    pub purpose: String,
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
    pub purpose: Option<String>,
    pub printers: Option<Vec<PrinterInput>>,
    pub is_active: Option<bool>,
}
