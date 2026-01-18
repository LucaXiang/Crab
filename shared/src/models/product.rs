//! Product Model

use serde::{Deserialize, Serialize};

/// Product entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Product {
    pub id: Option<String>,
    pub name: String,
    pub image: String,
    /// Category reference (String ID, required)
    pub category: String,
    pub sort_order: i32,
    /// Tax rate in percentage (e.g., 10 = 10%)
    pub tax_rate: i32,
    pub has_multi_spec: bool,
    pub receipt_name: Option<String>,
    pub kitchen_print_name: Option<String>,
    /// Kitchen printer reference (override category setting)
    pub kitchen_printer: Option<String>,
    /// -1=inherit, 0=disabled, 1=enabled
    pub is_kitchen_print_enabled: i32,
    pub is_label_print_enabled: i32,
    pub is_active: bool,
}

/// Create product payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductCreate {
    pub name: String,
    pub image: Option<String>,
    pub category: String,
    pub sort_order: Option<i32>,
    pub tax_rate: Option<i32>,
    pub has_multi_spec: Option<bool>,
    pub receipt_name: Option<String>,
    pub kitchen_print_name: Option<String>,
    pub kitchen_printer: Option<String>,
    pub is_kitchen_print_enabled: Option<i32>,
    pub is_label_print_enabled: Option<i32>,
}

/// Update product payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductUpdate {
    pub name: Option<String>,
    pub image: Option<String>,
    pub category: Option<String>,
    pub sort_order: Option<i32>,
    pub tax_rate: Option<i32>,
    pub has_multi_spec: Option<bool>,
    pub receipt_name: Option<String>,
    pub kitchen_print_name: Option<String>,
    pub kitchen_printer: Option<String>,
    pub is_kitchen_print_enabled: Option<i32>,
    pub is_label_print_enabled: Option<i32>,
    pub is_active: Option<bool>,
}

/// Product specification entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductSpecification {
    pub id: Option<String>,
    /// Product reference (String ID)
    pub product: String,
    pub name: String,
    /// Price in cents
    pub price: i64,
    pub display_order: i32,
    pub is_default: bool,
    pub is_active: bool,
    /// Is root spec (single-spec product's only spec)
    pub is_root: bool,
    pub external_id: Option<i64>,
    /// Tag references (String IDs)
    pub tags: Vec<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

/// Create specification payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductSpecificationCreate {
    pub product: String,
    pub name: String,
    pub price: i64,
    pub display_order: Option<i32>,
    pub is_default: Option<bool>,
    pub is_root: Option<bool>,
    pub external_id: Option<i64>,
    pub tags: Option<Vec<String>>,
}

/// Update specification payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductSpecificationUpdate {
    pub name: Option<String>,
    pub price: Option<i64>,
    pub display_order: Option<i32>,
    pub is_default: Option<bool>,
    pub is_active: Option<bool>,
    pub is_root: Option<bool>,
    pub external_id: Option<i64>,
    pub tags: Option<Vec<String>>,
}
