//! Print Destination Model

use super::serde_helpers;
use serde::{Deserialize, Serialize};
use surrealdb::sql::Thing;

pub type PrintDestinationId = Thing;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddedPrinter {
    pub printer_type: String, // "network" | "driver"
    pub ip: Option<String>,
    pub port: Option<u16>,
    pub driver_name: Option<String>,
    #[serde(default)]
    pub priority: i32,
    #[serde(default = "default_true", deserialize_with = "serde_helpers::bool_true")]
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrintDestination {
    pub id: Option<PrintDestinationId>,
    pub name: String,
    pub description: Option<String>,
    #[serde(default)]
    pub printers: Vec<EmbeddedPrinter>,
    #[serde(default = "default_true", deserialize_with = "serde_helpers::bool_true")]
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
    pub name: Option<String>,
    pub description: Option<String>,
    pub printers: Option<Vec<EmbeddedPrinter>>,
    pub is_active: Option<bool>,
}
