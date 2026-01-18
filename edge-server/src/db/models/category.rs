//! Category Model

use serde::{Deserialize, Serialize};
use surrealdb::sql::Thing;

pub type CategoryId = Thing;

/// Category model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Category {
    pub id: Option<CategoryId>,
    pub name: String,
    #[serde(default)]
    pub sort_order: i32,
    /// Record link to kitchen_printer
    pub kitchen_printer: Option<Thing>,
    #[serde(default = "default_true")]
    pub is_kitchen_print_enabled: bool,
    #[serde(default = "default_true")]
    pub is_label_print_enabled: bool,
    #[serde(default = "default_true")]
    pub is_active: bool,
}

fn default_true() -> bool {
    true
}

impl Category {
    pub fn new(name: String) -> Self {
        Self {
            id: None,
            name,
            sort_order: 0,
            kitchen_printer: None,
            is_kitchen_print_enabled: true,
            is_label_print_enabled: true,
            is_active: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryCreate {
    pub name: String,
    pub sort_order: Option<i32>,
    /// Kitchen printer ID (string)
    pub kitchen_printer: Option<String>,
    pub is_kitchen_print_enabled: Option<bool>,
    pub is_label_print_enabled: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryUpdate {
    pub name: Option<String>,
    pub sort_order: Option<i32>,
    pub kitchen_printer: Option<String>,
    pub is_kitchen_print_enabled: Option<bool>,
    pub is_label_print_enabled: Option<bool>,
    pub is_active: Option<bool>,
}
