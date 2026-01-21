//! Attribute Model (Graph DB style)
//!
//! Options are embedded directly in the attribute record.
//! Use RELATE to connect products/categories to attributes.

use super::serde_helpers;
use serde::{Deserialize, Serialize};
use surrealdb::sql::Thing;

pub type AttributeId = Thing;

/// Attribute Option (embedded in Attribute)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttributeOption {
    pub name: String,
    pub value_code: Option<String>,
    /// Price modifier in cents (positive=add, negative=subtract)
    #[serde(default)]
    pub price_modifier: i64,
    #[serde(default, deserialize_with = "serde_helpers::bool_false")]
    pub is_default: bool,
    #[serde(default)]
    pub display_order: i32,
    #[serde(default = "default_true", deserialize_with = "serde_helpers::bool_true")]
    pub is_active: bool,
    pub receipt_name: Option<String>,
}

fn default_true() -> bool {
    true
}

impl AttributeOption {
    pub fn new(name: String) -> Self {
        Self {
            name,
            value_code: None,
            price_modifier: 0,
            is_default: false,
            display_order: 0,
            is_active: true,
            receipt_name: None,
        }
    }
}

/// Attribute model (with embedded options)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attribute {
    pub id: Option<AttributeId>,
    pub name: String,
    /// Attribute type: single_select, multi_select
    #[serde(default = "default_attr_type")]
    pub attr_type: String,
    #[serde(default)]
    pub display_order: i32,
    #[serde(default = "default_true", deserialize_with = "serde_helpers::bool_true")]
    pub is_active: bool,
    #[serde(default, deserialize_with = "serde_helpers::bool_false")]
    pub show_on_receipt: bool,
    pub receipt_name: Option<String>,
    pub kitchen_printer: Option<Thing>,
    #[serde(default, deserialize_with = "serde_helpers::bool_false")]
    pub is_global: bool,
    /// Embedded options (Graph DB style - no join table)
    #[serde(default)]
    pub options: Vec<AttributeOption>,
}

fn default_attr_type() -> String {
    "single_select".to_string()
}

impl Attribute {
    pub fn new(name: String) -> Self {
        Self {
            id: None,
            name,
            attr_type: default_attr_type(),
            display_order: 0,
            is_active: true,
            show_on_receipt: false,
            receipt_name: None,
            kitchen_printer: None,
            is_global: false,
            options: vec![],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttributeCreate {
    pub name: String,
    pub attr_type: Option<String>,
    pub display_order: Option<i32>,
    pub show_on_receipt: Option<bool>,
    pub receipt_name: Option<String>,
    pub kitchen_printer: Option<Thing>,
    pub is_global: Option<bool>,
    pub options: Option<Vec<AttributeOption>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttributeUpdate {
    pub name: Option<String>,
    pub attr_type: Option<String>,
    pub display_order: Option<i32>,
    pub is_active: Option<bool>,
    pub show_on_receipt: Option<bool>,
    pub receipt_name: Option<String>,
    pub kitchen_printer: Option<Thing>,
    pub is_global: Option<bool>,
    pub options: Option<Vec<AttributeOption>>,
}

/// Edge relation: has_attribute (product/category -> attribute)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HasAttribute {
    pub id: Option<Thing>,
    #[serde(rename = "in")]
    pub from: Thing,  // product or category
    #[serde(rename = "out")]
    pub to: Thing,    // attribute
    #[serde(default)]
    pub is_required: bool,
    #[serde(default)]
    pub display_order: i32,
    pub default_option_idx: Option<i32>,
}
