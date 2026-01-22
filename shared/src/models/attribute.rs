//! Attribute Model

use serde::{Deserialize, Serialize};

/// Attribute option (embedded in Attribute)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttributeOption {
    pub name: String,
    /// Price modifier in currency unit (positive=add, negative=subtract, e.g., 2.50 = ¥2.50)
    pub price_modifier: f64,
    pub display_order: i32,
    pub is_active: bool,
    pub receipt_name: Option<String>,
    pub kitchen_print_name: Option<String>,
}

/// Attribute entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attribute {
    pub id: Option<String>,
    pub name: String,

    // 作用域
    /// Scope: "global" | "inherited"
    pub scope: String,
    /// Excluded categories (only for global scope)
    pub excluded_categories: Vec<String>,

    // 选择模式
    pub is_multi_select: bool,
    /// Max selections (null = unlimited)
    pub max_selections: Option<i32>,

    // 默认值
    pub default_option_idx: Option<i32>,

    // 显示
    pub display_order: i32,
    pub is_active: bool,

    // 小票
    pub show_on_receipt: bool,
    pub receipt_name: Option<String>,

    // 厨打
    pub show_on_kitchen_print: bool,
    pub kitchen_print_name: Option<String>,

    /// Embedded options
    pub options: Vec<AttributeOption>,
}

/// Create attribute payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttributeCreate {
    pub name: String,
    pub scope: Option<String>,
    pub excluded_categories: Option<Vec<String>>,
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

/// Update attribute payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttributeUpdate {
    pub name: Option<String>,
    pub scope: Option<String>,
    pub excluded_categories: Option<Vec<String>>,
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

/// Has attribute relation (for querying product/category attributes)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HasAttribute {
    pub id: Option<String>,
    pub from: String,
    pub to: String,
    pub is_required: bool,
    pub display_order: i32,
}
