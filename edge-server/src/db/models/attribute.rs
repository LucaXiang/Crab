//! Attribute Model (Graph DB style)
//!
//! Options are embedded directly in the attribute record.
//! Use RELATE to connect products/categories to attributes.

use super::serde_helpers;
use serde::{Deserialize, Serialize};
use surrealdb::RecordId;

pub type AttributeId = RecordId;

/// Attribute Option (embedded in Attribute)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttributeOption {
    pub name: String,
    /// Price modifier in currency unit (positive=add, negative=subtract, e.g., 2.50 = €2.50)
    pub price_modifier: f64,
    pub display_order: i32,
    pub is_active: bool,
    pub receipt_name: Option<String>,
    pub kitchen_print_name: Option<String>,

    // === Quantity Control ===
    /// Enable quantity control for this option (default: false)
    pub enable_quantity: bool,
    /// Maximum quantity allowed (only effective when enable_quantity=true)
    pub max_quantity: Option<i32>,
}

impl AttributeOption {
    pub fn new(name: String) -> Self {
        Self {
            name,
            price_modifier: 0.0,
            display_order: 0,
            is_active: true,
            receipt_name: None,
            kitchen_print_name: None,
            enable_quantity: false,
            max_quantity: None,
        }
    }
}

/// Attribute model (with embedded options)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attribute {
    #[serde(default, with = "serde_helpers::option_record_id")]
    pub id: Option<AttributeId>,
    pub name: String,

    // 选择模式
    pub is_multi_select: bool,
    /// Max selections (null = unlimited)
    pub max_selections: Option<i32>,

    // 默认值 (支持多选属性的多个默认)
    pub default_option_indices: Option<Vec<i32>>,

    // 显示
    pub display_order: i32,
    pub is_active: bool,

    // 小票
    pub show_on_receipt: bool,
    pub receipt_name: Option<String>,

    // 厨打
    pub show_on_kitchen_print: bool,
    pub kitchen_print_name: Option<String>,

    /// Embedded options (Graph DB style - no join table)
    pub options: Vec<AttributeOption>,
}

impl Attribute {
    pub fn new(name: String) -> Self {
        Self {
            id: None,
            name,
            is_multi_select: false,
            max_selections: None,
            default_option_indices: None,
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
    pub is_multi_select: Option<bool>,
    pub max_selections: Option<i32>,
    pub default_option_indices: Option<Vec<i32>>,
    pub display_order: Option<i32>,
    pub show_on_receipt: Option<bool>,
    pub receipt_name: Option<String>,
    pub show_on_kitchen_print: Option<bool>,
    pub kitchen_print_name: Option<String>,
    pub options: Option<Vec<AttributeOption>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttributeUpdate {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_multi_select: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_selections: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_option_indices: Option<Vec<i32>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_order: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_active: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub show_on_receipt: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub receipt_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub show_on_kitchen_print: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kitchen_print_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<Vec<AttributeOption>>,
}

/// Edge relation: has_attribute (product/category -> attribute)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttributeBinding {
    #[serde(default, with = "serde_helpers::option_record_id")]
    pub id: Option<RecordId>,
    #[serde(rename = "in", with = "serde_helpers::record_id")]
    pub from: RecordId, // product or category
    #[serde(rename = "out", with = "serde_helpers::record_id")]
    pub to: RecordId, // attribute
    pub is_required: bool,
    pub display_order: i32,
    /// Override attribute's default options (supports multi-select)
    pub default_option_indices: Option<Vec<i32>>,
}

/// Attribute binding with full attribute data (for API responses)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttributeBindingFull {
    /// Relation ID
    #[serde(default, with = "serde_helpers::option_record_id")]
    pub id: Option<RecordId>,
    /// Full attribute object
    pub attribute: Attribute,
    pub is_required: bool,
    pub display_order: i32,
    /// Override attribute's default options (supports multi-select)
    pub default_option_indices: Option<Vec<i32>>,
    /// Whether this binding is inherited from the product's category (computed field, not from DB)
    pub is_inherited: bool,
}
