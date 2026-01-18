//! Attribute Model

use serde::{Deserialize, Serialize};

/// Attribute option (embedded in Attribute)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttributeOption {
    pub name: String,
    pub value_code: Option<String>,
    /// Price modifier in cents (positive=add, negative=subtract)
    pub price_modifier: i64,
    pub is_default: bool,
    pub display_order: i32,
    pub is_active: bool,
    pub receipt_name: Option<String>,
}

/// Attribute entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attribute {
    pub id: Option<String>,
    pub name: String,
    /// Attribute type: single_select, multi_select
    pub attr_type: String,
    pub display_order: i32,
    pub is_active: bool,
    pub show_on_receipt: bool,
    pub receipt_name: Option<String>,
    /// Kitchen printer reference (String ID)
    pub kitchen_printer: Option<String>,
    pub is_global: bool,
    /// Embedded options
    pub options: Vec<AttributeOption>,
}

/// Create attribute payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttributeCreate {
    pub name: String,
    pub attr_type: Option<String>,
    pub display_order: Option<i32>,
    pub show_on_receipt: Option<bool>,
    pub receipt_name: Option<String>,
    pub kitchen_printer: Option<String>,
    pub is_global: Option<bool>,
    pub options: Option<Vec<AttributeOption>>,
}

/// Update attribute payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttributeUpdate {
    pub name: Option<String>,
    pub attr_type: Option<String>,
    pub display_order: Option<i32>,
    pub is_active: Option<bool>,
    pub show_on_receipt: Option<bool>,
    pub receipt_name: Option<String>,
    pub kitchen_printer: Option<String>,
    pub is_global: Option<bool>,
    pub options: Option<Vec<AttributeOption>>,
}

/// Has attribute relation (for querying product/category attributes)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HasAttribute {
    pub id: Option<String>,
    pub from: String,
    pub to: String,
    pub is_required: bool,
    pub display_order: i32,
    pub default_option_idx: Option<i32>,
}
