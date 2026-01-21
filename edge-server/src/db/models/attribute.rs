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
    /// Price modifier in cents (positive=add, negative=subtract)
    #[serde(default)]
    pub price_modifier: i64,
    #[serde(default)]
    pub display_order: i32,
    #[serde(
        default = "default_true",
        deserialize_with = "serde_helpers::bool_true"
    )]
    pub is_active: bool,
    pub receipt_name: Option<String>,
    pub kitchen_print_name: Option<String>,
}

fn default_true() -> bool {
    true
}

impl AttributeOption {
    pub fn new(name: String) -> Self {
        Self {
            name,
            price_modifier: 0,
            display_order: 0,
            is_active: true,
            receipt_name: None,
            kitchen_print_name: None,
        }
    }
}

/// Attribute model (with embedded options)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attribute {
    pub id: Option<AttributeId>,
    pub name: String,

    // 作用域
    /// Scope: "global" | "inherited"
    #[serde(default = "default_scope")]
    pub scope: String,
    /// Excluded categories (only for global scope)
    #[serde(default)]
    pub excluded_categories: Vec<Thing>,

    // 选择模式
    #[serde(default, deserialize_with = "serde_helpers::bool_false")]
    pub is_multi_select: bool,
    /// Max selections (null = unlimited)
    pub max_selections: Option<i32>,

    // 默认值
    pub default_option_idx: Option<i32>,

    // 显示
    #[serde(default)]
    pub display_order: i32,
    #[serde(
        default = "default_true",
        deserialize_with = "serde_helpers::bool_true"
    )]
    pub is_active: bool,

    // 小票
    #[serde(default, deserialize_with = "serde_helpers::bool_false")]
    pub show_on_receipt: bool,
    pub receipt_name: Option<String>,

    // 厨打
    #[serde(default, deserialize_with = "serde_helpers::bool_false")]
    pub show_on_kitchen_print: bool,
    pub kitchen_print_name: Option<String>,

    /// Embedded options (Graph DB style - no join table)
    #[serde(default)]
    pub options: Vec<AttributeOption>,
}

fn default_scope() -> String {
    "inherited".to_string()
}

impl Attribute {
    pub fn new(name: String) -> Self {
        Self {
            id: None,
            name,
            scope: default_scope(),
            excluded_categories: vec![],
            is_multi_select: false,
            max_selections: None,
            default_option_idx: None,
            display_order: 0,
            is_active: true,
            show_on_receipt: false,
            receipt_name: None,
            show_on_kitchen_print: false,
            kitchen_print_name: None,
            options: vec![],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttributeCreate {
    pub name: String,
    pub scope: Option<String>,
    pub excluded_categories: Option<Vec<Thing>>,
    pub is_multi_select: Option<bool>,
    pub max_selections: Option<i32>,
    pub default_option_idx: Option<i32>,
    pub display_order: Option<i32>,
    pub show_on_receipt: Option<bool>,
    pub receipt_name: Option<String>,
    pub show_on_kitchen_print: Option<bool>,
    pub kitchen_print_name: Option<String>,
    pub options: Option<Vec<AttributeOption>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttributeUpdate {
    pub name: Option<String>,
    pub scope: Option<String>,
    pub excluded_categories: Option<Vec<Thing>>,
    pub is_multi_select: Option<bool>,
    pub max_selections: Option<i32>,
    pub default_option_idx: Option<i32>,
    pub display_order: Option<i32>,
    pub is_active: Option<bool>,
    pub show_on_receipt: Option<bool>,
    pub receipt_name: Option<String>,
    pub show_on_kitchen_print: Option<bool>,
    pub kitchen_print_name: Option<String>,
    pub options: Option<Vec<AttributeOption>>,
}

/// Edge relation: has_attribute (product/category -> attribute)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HasAttribute {
    pub id: Option<Thing>,
    #[serde(rename = "in")]
    pub from: Thing, // product or category
    #[serde(rename = "out")]
    pub to: Thing, // attribute
    #[serde(default)]
    pub is_required: bool,
    #[serde(default)]
    pub display_order: i32,
}
