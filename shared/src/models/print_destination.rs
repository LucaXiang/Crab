//! Print Destination Model

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddedPrinter {
    pub printer_type: String, // "network" | "driver"
    /// Printer format: "escpos" (厨房单/小票) | "label" (标签)
    #[serde(default = "default_printer_format")]
    pub printer_format: String,
    pub ip: Option<String>,
    pub port: Option<u16>,
    pub driver_name: Option<String>,
    pub priority: i32,
    pub is_active: bool,
}

fn default_printer_format() -> String {
    "escpos".to_string()
}

/// Print destination entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrintDestination {
    pub id: Option<String>,
    pub name: String,
    pub description: Option<String>,
    pub printers: Vec<EmbeddedPrinter>,
    pub is_active: bool,
}

/// Create print destination payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrintDestinationCreate {
    pub name: String,
    pub description: Option<String>,
    #[serde(default)]
    pub printers: Vec<EmbeddedPrinter>,
    #[serde(default = "default_true")]
    pub is_active: bool,
}

fn default_true() -> bool {
    true
}

/// Update print destination payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrintDestinationUpdate {
    pub name: Option<String>,
    pub description: Option<String>,
    pub printers: Option<Vec<EmbeddedPrinter>>,
    pub is_active: Option<bool>,
}
