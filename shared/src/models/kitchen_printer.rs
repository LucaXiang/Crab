//! Kitchen Printer Model

use serde::{Deserialize, Serialize};

/// Kitchen printer entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KitchenPrinter {
    pub id: Option<String>,
    pub name: String,
    pub printer_name: Option<String>,
    pub description: Option<String>,
    pub is_active: bool,
}

/// Create kitchen printer payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KitchenPrinterCreate {
    pub name: String,
    pub printer_name: Option<String>,
    pub description: Option<String>,
}

/// Update kitchen printer payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KitchenPrinterUpdate {
    pub name: Option<String>,
    pub printer_name: Option<String>,
    pub description: Option<String>,
    pub is_active: Option<bool>,
}
