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
    /// 小票显示名称（如 "L", "M", "大杯"）
    pub receipt_name: Option<String>,
    /// 根规格，不可删除（每个商品至少保留一个）
    #[serde(default)]
    pub is_root: bool,
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
    /// 厨房打印目的地
    #[serde(default)]
    pub kitchen_print_destinations: Vec<String>,
    /// 标签打印目的地
    #[serde(default)]
    pub label_print_destinations: Vec<String>,
    /// 厨房打印启用状态 (-1=继承, 0=禁用, 1=启用)
    pub is_kitchen_print_enabled: i32,
    /// 标签打印启用状态 (-1=继承, 0=禁用, 1=启用)
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
    /// 厨房打印目的地
    pub kitchen_print_destinations: Option<Vec<String>>,
    /// 标签打印目的地
    pub label_print_destinations: Option<Vec<String>>,
    /// 厨房打印启用状态 (-1=继承, 0=禁用, 1=启用)
    pub is_kitchen_print_enabled: Option<i32>,
    /// 标签打印启用状态 (-1=继承, 0=禁用, 1=启用)
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
    /// 厨房打印目的地
    pub kitchen_print_destinations: Option<Vec<String>>,
    /// 标签打印目的地
    pub label_print_destinations: Option<Vec<String>>,
    /// 厨房打印启用状态 (-1=继承, 0=禁用, 1=启用)
    pub is_kitchen_print_enabled: Option<i32>,
    /// 标签打印启用状态 (-1=继承, 0=禁用, 1=启用)
    pub is_label_print_enabled: Option<i32>,
    pub is_active: Option<bool>,
    pub tags: Option<Vec<String>>,
    /// 规格列表 (更新时可选)
    pub specs: Option<Vec<EmbeddedSpec>>,
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
    /// 厨房打印目的地
    #[serde(default)]
    pub kitchen_print_destinations: Vec<String>,
    /// 标签打印目的地
    #[serde(default)]
    pub label_print_destinations: Vec<String>,
    /// 厨房打印启用状态 (-1=继承, 0=禁用, 1=启用)
    pub is_kitchen_print_enabled: i32,
    /// 标签打印启用状态 (-1=继承, 0=禁用, 1=启用)
    pub is_label_print_enabled: i32,
    pub is_active: bool,
    /// 嵌入式规格
    pub specs: Vec<EmbeddedSpec>,
    /// Attribute bindings with full attribute data
    pub attributes: Vec<super::attribute::AttributeBindingFull>,
    /// Tags attached to this product
    pub tags: Vec<super::tag::Tag>,
}
