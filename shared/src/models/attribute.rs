//! Attribute Model

use serde::{Deserialize, Serialize};

/// Attribute option (embedded in Attribute)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttributeOption {
    pub name: String,
    /// Price modifier in currency unit (positive=add, negative=subtract, e.g., 2.50 = €2.50)
    pub price_modifier: f64,
    pub display_order: i32,
    pub receipt_name: Option<String>,
    pub kitchen_print_name: Option<String>,

    // === Quantity Control ===
    /// Enable quantity control for this option (default: false)
    /// When false, the option can only be selected once (quantity implicitly 1)
    /// When true, user can select multiple quantities with +/- buttons
    #[serde(default)]
    pub enable_quantity: bool,
    /// Maximum quantity allowed (only effective when enable_quantity=true)
    /// None = unlimited
    #[serde(default)]
    pub max_quantity: Option<i32>,
}

/// Attribute entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attribute {
    pub id: Option<String>,
    pub name: String,

    // 选择模式
    pub is_multi_select: bool,
    /// Max selections (null = unlimited)
    pub max_selections: Option<i32>,

    // 默认值 (支持多选属性的多个默认)
    pub default_option_indices: Option<Vec<i32>>,

    // 显示
    pub display_order: i32,

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

/// Update attribute payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttributeUpdate {
    pub name: Option<String>,
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

/// Attribute binding relation (product/category -> attribute)
///
/// 用于建立商品或分类与属性的关联关系
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttributeBinding {
    pub id: Option<String>,
    /// Source: product ID or category ID (SurrealDB `in` field)
    #[serde(rename = "in")]
    pub from: String,
    /// Target: attribute ID (SurrealDB `out` field)
    #[serde(rename = "out")]
    pub to: String,
    #[serde(default)]
    pub is_required: bool,
    #[serde(default)]
    pub display_order: i32,
    /// Override attribute's default options (optional, supports multi-select)
    pub default_option_indices: Option<Vec<i32>>,
}

/// Attribute binding with full attribute data (for API responses)
///
/// 查询 API 响应时使用，包含完整的属性对象
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttributeBindingFull {
    /// Relation ID
    pub id: Option<String>,
    /// Full attribute object
    pub attribute: Attribute,
    pub is_required: bool,
    pub display_order: i32,
    /// Override attribute's default options (optional, supports multi-select)
    pub default_option_indices: Option<Vec<i32>>,
    /// Whether this binding is inherited from the product's category
    #[serde(default)]
    pub is_inherited: bool,
}
