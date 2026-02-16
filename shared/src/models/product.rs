//! Product Model

use serde::{Deserialize, Serialize};

/// Product spec (independent table, was EmbeddedSpec)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "db", derive(sqlx::FromRow))]
pub struct ProductSpec {
    pub id: i64,
    pub product_id: i64,
    pub name: String,
    pub price: f64,
    pub display_order: i32,
    pub is_default: bool,
    pub is_active: bool,
    pub receipt_name: Option<String>,
    /// 根规格，不可删除（每个商品至少保留一个）
    pub is_root: bool,
}

/// Product entity
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "db", derive(sqlx::FromRow))]
pub struct Product {
    pub id: i64,
    pub name: String,
    pub image: String,
    pub category_id: i64,
    pub sort_order: i32,
    /// Tax rate in percentage (e.g., 10 = 10%)
    pub tax_rate: i32,
    pub receipt_name: Option<String>,
    pub kitchen_print_name: Option<String>,
    /// 厨房打印启用状态 (-1=继承, 0=禁用, 1=启用)
    pub is_kitchen_print_enabled: i32,
    /// 标签打印启用状态 (-1=继承, 0=禁用, 1=启用)
    pub is_label_print_enabled: i32,
    pub is_active: bool,
    /// 菜品编号 (POS 集成)
    pub external_id: Option<i64>,

    // -- Relations (populated by application code, skipped by FromRow) --
    /// Tag IDs (junction table product_tag)
    #[cfg_attr(feature = "db", sqlx(skip))]
    #[serde(default)]
    pub tags: Vec<i64>,
    /// Product specs (child table product_spec)
    #[cfg_attr(feature = "db", sqlx(skip))]
    #[serde(default)]
    pub specs: Vec<ProductSpec>,
}

/// Create product payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductCreate {
    pub name: String,
    pub image: Option<String>,
    pub category_id: i64,
    pub sort_order: Option<i32>,
    pub tax_rate: Option<i32>,
    pub receipt_name: Option<String>,
    pub kitchen_print_name: Option<String>,
    pub is_kitchen_print_enabled: Option<i32>,
    pub is_label_print_enabled: Option<i32>,
    pub external_id: Option<i64>,
    pub tags: Option<Vec<i64>>,
    /// 规格列表 (至少 1 个)
    pub specs: Vec<ProductSpecInput>,
}

/// Update product payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductUpdate {
    pub name: Option<String>,
    pub image: Option<String>,
    pub category_id: Option<i64>,
    pub sort_order: Option<i32>,
    pub tax_rate: Option<i32>,
    pub receipt_name: Option<String>,
    pub kitchen_print_name: Option<String>,
    pub is_kitchen_print_enabled: Option<i32>,
    pub is_label_print_enabled: Option<i32>,
    pub is_active: Option<bool>,
    pub external_id: Option<i64>,
    pub tags: Option<Vec<i64>>,
    pub specs: Option<Vec<ProductSpecInput>>,
}

/// Product spec input (for create/update, without id/product_id)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductSpecInput {
    pub name: String,
    #[serde(default)]
    pub price: f64,
    #[serde(default)]
    pub display_order: i32,
    #[serde(default)]
    pub is_default: bool,
    #[serde(default = "default_true")]
    pub is_active: bool,
    pub receipt_name: Option<String>,
    #[serde(default)]
    pub is_root: bool,
}

fn default_true() -> bool {
    true
}

/// Full product with all related data (for API responses)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductFull {
    pub id: i64,
    pub name: String,
    pub image: String,
    pub category_id: i64,
    pub sort_order: i32,
    pub tax_rate: i32,
    pub receipt_name: Option<String>,
    pub kitchen_print_name: Option<String>,
    pub is_kitchen_print_enabled: i32,
    pub is_label_print_enabled: i32,
    pub is_active: bool,
    pub external_id: Option<i64>,
    pub specs: Vec<ProductSpec>,
    /// Attribute bindings with full attribute data
    pub attributes: Vec<super::attribute::AttributeBindingFull>,
    /// Tags attached to this product
    pub tags: Vec<super::tag::Tag>,
}
