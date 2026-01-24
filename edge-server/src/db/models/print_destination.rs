//! Print Destination Model

use super::serde_helpers;
use serde::{Deserialize, Serialize};
use surrealdb::RecordId;

pub type PrintDestinationId = RecordId;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddedPrinter {
    pub printer_type: String, // "network" | "driver"
    /// Printer format: "escpos" (厨房单/小票) | "label" (标签)
    #[serde(default = "default_printer_format")]
    pub printer_format: String,
    pub ip: Option<String>,
    pub port: Option<u16>,
    pub driver_name: Option<String>,
    #[serde(default)]
    pub priority: i32,
    #[serde(
        default = "default_true",
        deserialize_with = "serde_helpers::bool_true"
    )]
    pub is_active: bool,
}

fn default_printer_format() -> String {
    "escpos".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrintDestination {
    #[serde(default, with = "serde_helpers::option_record_id")]
    pub id: Option<PrintDestinationId>,
    pub name: String,
    pub description: Option<String>,
    #[serde(default)]
    pub printers: Vec<EmbeddedPrinter>,
    #[serde(
        default = "default_true",
        deserialize_with = "serde_helpers::bool_true"
    )]
    pub is_active: bool,
}

fn default_true() -> bool {
    true
}

impl PrintDestination {
    pub fn new(name: String) -> Self {
        Self {
            id: None,
            name,
            description: None,
            printers: Vec::new(),
            is_active: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrintDestinationCreate {
    pub name: String,
    pub description: Option<String>,
    #[serde(default)]
    pub printers: Vec<EmbeddedPrinter>,
    #[serde(default = "default_true")]
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrintDestinationUpdate {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub printers: Option<Vec<EmbeddedPrinter>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_active: Option<bool>,
}
