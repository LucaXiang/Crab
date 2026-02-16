//! Attribute Model

use serde::{Deserialize, Serialize};

/// Attribute option (independent table)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "db", derive(sqlx::FromRow))]
pub struct AttributeOption {
    pub id: i64,
    pub attribute_id: i64,
    pub name: String,
    /// Price modifier in currency unit (positive=add, negative=subtract)
    pub price_modifier: f64,
    pub display_order: i32,
    pub is_active: bool,
    pub receipt_name: Option<String>,
    pub kitchen_print_name: Option<String>,
    /// Enable quantity control for this option
    pub enable_quantity: bool,
    /// Maximum quantity allowed (only effective when enable_quantity=true)
    pub max_quantity: Option<i32>,
}

/// Attribute entity
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "db", derive(sqlx::FromRow))]
pub struct Attribute {
    pub id: i64,
    pub name: String,
    pub is_multi_select: bool,
    pub max_selections: Option<i32>,
    /// Default option IDs (JSON array of int in DB)
    #[cfg_attr(feature = "db", sqlx(json))]
    pub default_option_ids: Option<Vec<i32>>,
    pub display_order: i32,
    pub is_active: bool,
    pub show_on_receipt: bool,
    pub receipt_name: Option<String>,
    pub show_on_kitchen_print: bool,
    pub kitchen_print_name: Option<String>,

    // -- Relations (populated by application code, skipped by FromRow) --
    /// Embedded options
    #[cfg_attr(feature = "db", sqlx(skip))]
    #[serde(default)]
    pub options: Vec<AttributeOption>,
}

/// Attribute option input (for create/update, without id/attribute_id)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttributeOptionInput {
    pub name: String,
    #[serde(default)]
    pub price_modifier: f64,
    #[serde(default)]
    pub display_order: i32,
    pub receipt_name: Option<String>,
    pub kitchen_print_name: Option<String>,
    #[serde(default)]
    pub enable_quantity: bool,
    pub max_quantity: Option<i32>,
}

/// Create attribute payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttributeCreate {
    pub name: String,
    pub is_multi_select: Option<bool>,
    pub max_selections: Option<i32>,
    pub default_option_ids: Option<Vec<i32>>,
    pub display_order: Option<i32>,
    pub show_on_receipt: Option<bool>,
    pub receipt_name: Option<String>,
    pub show_on_kitchen_print: Option<bool>,
    pub kitchen_print_name: Option<String>,
    pub options: Option<Vec<AttributeOptionInput>>,
}

/// Update attribute payload
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AttributeUpdate {
    pub name: Option<String>,
    pub is_multi_select: Option<bool>,
    pub max_selections: Option<i32>,
    pub default_option_ids: Option<Vec<i32>>,
    pub display_order: Option<i32>,
    pub show_on_receipt: Option<bool>,
    pub receipt_name: Option<String>,
    pub show_on_kitchen_print: Option<bool>,
    pub kitchen_print_name: Option<String>,
    pub options: Option<Vec<AttributeOptionInput>>,
    pub is_active: Option<bool>,
}

/// Attribute binding (owner_type + owner_id polymorphic FK)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "db", derive(sqlx::FromRow))]
pub struct AttributeBinding {
    pub id: i64,
    /// Owner type: "product" or "category"
    pub owner_type: String,
    pub owner_id: i64,
    pub attribute_id: i64,
    pub is_required: bool,
    pub display_order: i32,
    /// Override attribute's default option IDs (JSON array of int in DB)
    #[cfg_attr(feature = "db", sqlx(json))]
    pub default_option_ids: Option<Vec<i32>>,
}

/// Attribute binding with full attribute data (for API responses)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttributeBindingFull {
    pub id: i64,
    /// Full attribute object (with options)
    pub attribute: Attribute,
    pub is_required: bool,
    pub display_order: i32,
    pub default_option_ids: Option<Vec<i32>>,
    /// Whether this binding is inherited from the product's category
    #[serde(default)]
    pub is_inherited: bool,
}
