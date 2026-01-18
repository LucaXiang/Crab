//! Category Model

use serde::{Deserialize, Serialize};

/// Category entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Category {
    pub id: Option<String>,
    pub name: String,
    pub sort_order: i32,
    /// Kitchen printer reference (String ID)
    pub kitchen_printer: Option<String>,
    pub is_kitchen_print_enabled: bool,
    pub is_label_print_enabled: bool,
    pub is_active: bool,
}

/// Create category payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryCreate {
    pub name: String,
    pub sort_order: Option<i32>,
    pub kitchen_printer: Option<String>,
    pub is_kitchen_print_enabled: Option<bool>,
    pub is_label_print_enabled: Option<bool>,
}

/// Update category payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryUpdate {
    pub name: Option<String>,
    pub sort_order: Option<i32>,
    pub kitchen_printer: Option<String>,
    pub is_kitchen_print_enabled: Option<bool>,
    pub is_label_print_enabled: Option<bool>,
    pub is_active: Option<bool>,
}
