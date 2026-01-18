//! Product Model

use serde::{Deserialize, Serialize};
use surrealdb::sql::Thing;

pub type ProductId = Thing;

/// Product model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Product {
    pub id: Option<ProductId>,
    pub name: String,
    #[serde(default)]
    pub image: String,
    /// Record link to category
    pub category: Thing,
    #[serde(default)]
    pub sort_order: i32,
    /// Tax rate in percentage (e.g., 10 = 10%)
    #[serde(default)]
    pub tax_rate: i32,
    #[serde(default)]
    pub has_multi_spec: bool,
    pub receipt_name: Option<String>,
    pub kitchen_print_name: Option<String>,
    /// Record link to kitchen_printer (override category setting)
    pub kitchen_printer: Option<Thing>,
    /// -1=inherit, 0=disabled, 1=enabled
    #[serde(default = "default_inherit")]
    pub is_kitchen_print_enabled: i32,
    #[serde(default = "default_inherit")]
    pub is_label_print_enabled: i32,
    #[serde(default = "default_true")]
    pub is_active: bool,
}

fn default_inherit() -> i32 {
    -1
}

fn default_true() -> bool {
    true
}

impl Product {
    pub fn new(name: String, category: Thing) -> Self {
        Self {
            id: None,
            name,
            image: String::new(),
            category,
            sort_order: 0,
            tax_rate: 0,
            has_multi_spec: false,
            receipt_name: None,
            kitchen_print_name: None,
            kitchen_printer: None,
            is_kitchen_print_enabled: -1,
            is_label_print_enabled: -1,
            is_active: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductCreate {
    pub name: String,
    pub image: Option<String>,
    pub category: Thing,
    pub sort_order: Option<i32>,
    pub tax_rate: Option<i32>,
    pub has_multi_spec: Option<bool>,
    pub receipt_name: Option<String>,
    pub kitchen_print_name: Option<String>,
    pub kitchen_printer: Option<Thing>,
    pub is_kitchen_print_enabled: Option<i32>,
    pub is_label_print_enabled: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductUpdate {
    pub name: Option<String>,
    pub image: Option<String>,
    pub category: Option<Thing>,
    pub sort_order: Option<i32>,
    pub tax_rate: Option<i32>,
    pub has_multi_spec: Option<bool>,
    pub receipt_name: Option<String>,
    pub kitchen_print_name: Option<String>,
    pub kitchen_printer: Option<Thing>,
    pub is_kitchen_print_enabled: Option<i32>,
    pub is_label_print_enabled: Option<i32>,
    pub is_active: Option<bool>,
}
