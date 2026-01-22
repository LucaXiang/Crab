//! Product Model

use serde::{Deserialize, Serialize};

/// 嵌入式规格
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddedSpec {
    pub name: String,
    /// Price in currency unit (e.g., 10.50 = ¥10.50)
    #[serde(default)]
    pub price: f64,
    #[serde(default)]
    pub display_order: i32,
    #[serde(default)]
    pub is_default: bool,
    #[serde(default = "default_true")]
    pub is_active: bool,
    pub external_id: Option<i64>,
}

fn default_true() -> bool {
    true
}

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
    pub receipt_name: Option<String>,
    pub kitchen_print_name: Option<String>,
    /// 打印目的地
    #[serde(default)]
    pub print_destinations: Vec<String>,
    pub is_label_print_enabled: i32,
    pub is_active: bool,
    /// Tag references (String IDs)
    #[serde(default)]
    pub tags: Vec<String>,
    /// 嵌入式规格 (至少 1 个)
    #[serde(default)]
    pub specs: Vec<EmbeddedSpec>,
}

/// Create product payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductCreate {
    pub name: String,
    pub image: Option<String>,
    pub category: String,
    pub sort_order: Option<i32>,
    pub tax_rate: Option<i32>,
    pub receipt_name: Option<String>,
    pub kitchen_print_name: Option<String>,
    pub print_destinations: Option<Vec<String>>,
    pub is_label_print_enabled: Option<i32>,
    pub tags: Option<Vec<String>>,
    /// 规格列表 (至少 1 个)
    pub specs: Vec<EmbeddedSpec>,
}

/// Update product payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductUpdate {
    pub name: Option<String>,
    pub image: Option<String>,
    pub category: Option<String>,
    pub sort_order: Option<i32>,
    pub tax_rate: Option<i32>,
    pub receipt_name: Option<String>,
    pub kitchen_print_name: Option<String>,
    pub print_destinations: Option<Vec<String>>,
    pub is_label_print_enabled: Option<i32>,
    pub is_active: Option<bool>,
    pub tags: Option<Vec<String>>,
    /// 规格列表 (更新时可选)
    pub specs: Option<Vec<EmbeddedSpec>>,
}

/// Product attribute binding with full attribute data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductAttributeBinding {
    /// Relation ID (has_attribute edge)
    pub id: Option<String>,
    /// Full attribute object
    pub attribute: super::attribute::Attribute,
    pub is_required: bool,
    pub display_order: i32,
}

/// Full product with all related data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductFull {
    pub id: Option<String>,
    pub name: String,
    pub image: String,
    pub category: String,
    pub sort_order: i32,
    pub tax_rate: i32,
    pub receipt_name: Option<String>,
    pub kitchen_print_name: Option<String>,
    /// 打印目的地
    #[serde(default)]
    pub print_destinations: Vec<String>,
    pub is_label_print_enabled: i32,
    pub is_active: bool,
    /// 嵌入式规格
    pub specs: Vec<EmbeddedSpec>,
    /// Attribute bindings with full attribute data
    pub attributes: Vec<ProductAttributeBinding>,
    /// Tags attached to this product
    pub tags: Vec<super::tag::Tag>,
}
