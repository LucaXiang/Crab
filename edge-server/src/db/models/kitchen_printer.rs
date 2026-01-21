//! Kitchen Printer Model

use super::serde_helpers;
use serde::{Deserialize, Serialize};
use surrealdb::sql::Thing;

pub type KitchenPrinterId = Thing;

/// Kitchen Printer model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KitchenPrinter {
    pub id: Option<KitchenPrinterId>,
    pub name: String,
    pub printer_name: Option<String>,
    pub description: Option<String>,
    #[serde(default = "default_true", deserialize_with = "serde_helpers::bool_true")]
    pub is_active: bool,
}

fn default_true() -> bool {
    true
}

impl KitchenPrinter {
    pub fn new(name: String) -> Self {
        Self {
            id: None,
            name,
            printer_name: None,
            description: None,
            is_active: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KitchenPrinterCreate {
    pub name: String,
    pub printer_name: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KitchenPrinterUpdate {
    pub name: Option<String>,
    pub printer_name: Option<String>,
    pub description: Option<String>,
    pub is_active: Option<bool>,
}
