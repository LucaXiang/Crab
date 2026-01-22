//! Product Model

use super::serde_helpers;
use super::serde_thing;
use serde::{Deserialize, Serialize};
use surrealdb::sql::Thing;

pub type ProductId = Thing;

/// 嵌入式规格 (文档数据库风格)
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
    /// Receipt display name (e.g., "L", "M", "大杯")
    pub receipt_name: Option<String>,
    /// Root spec, cannot be deleted (each product must have at least one)
    #[serde(default)]
    pub is_root: bool,
}

/// Product model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Product {
    pub id: Option<ProductId>,
    pub name: String,
    #[serde(default)]
    pub image: String,
    /// Record link to category
    pub category: Thing,
    #[serde(default)]
    pub sort_order: i32,
    /// Tax rate in percentage (e.g., 10 = 10%)
    #[serde(default)]
    pub tax_rate: i32,
    pub receipt_name: Option<String>,
    pub kitchen_print_name: Option<String>,
    /// 打印目的地
    #[serde(default)]
    pub print_destinations: Vec<Thing>,
    #[serde(default = "default_inherit")]
    pub is_label_print_enabled: i32,
    #[serde(
        default = "default_true",
        deserialize_with = "serde_helpers::bool_true"
    )]
    pub is_active: bool,
    /// Array of record links to tags
    #[serde(default)]
    pub tags: Vec<Thing>,
    /// 嵌入式规格数组
    #[serde(default)]
    pub specs: Vec<EmbeddedSpec>,
}

fn default_inherit() -> i32 {
    -1
}

fn default_true() -> bool {
    true
}

impl Product {
    pub fn new(name: String, category: Thing) -> Self {
        Self {
            id: None,
            name,
            image: String::new(),
            category,
            sort_order: 0,
            tax_rate: 0,
            receipt_name: None,
            kitchen_print_name: None,
            print_destinations: vec![],
            is_label_print_enabled: -1,
            is_active: true,
            tags: vec![],
            specs: vec![],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductCreate {
    pub name: String,
    pub image: Option<String>,
    #[serde(with = "serde_thing")]
    pub category: Thing,
    /// Price in currency unit (e.g., 10.50 = ¥10.50)
    pub price: Option<f64>,
    pub sort_order: Option<i32>,
    pub tax_rate: Option<i32>,
    pub receipt_name: Option<String>,
    pub kitchen_print_name: Option<String>,
    #[serde(default, with = "serde_thing::option_vec")]
    pub print_destinations: Option<Vec<Thing>>,
    pub is_label_print_enabled: Option<i32>,
    #[serde(default, with = "serde_thing::option_vec")]
    pub tags: Option<Vec<Thing>>,
    /// 嵌入式规格 (必需，至少一个规格)
    pub specs: Vec<EmbeddedSpec>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductUpdate {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
    #[serde(
        default,
        with = "serde_thing::option",
        skip_serializing_if = "Option::is_none"
    )]
    pub category: Option<Thing>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort_order: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tax_rate: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub receipt_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kitchen_print_name: Option<String>,
    #[serde(
        default,
        with = "serde_thing::option_vec",
        skip_serializing_if = "Option::is_none"
    )]
    pub print_destinations: Option<Vec<Thing>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_label_print_enabled: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_active: Option<bool>,
    #[serde(
        default,
        with = "serde_thing::option_vec",
        skip_serializing_if = "Option::is_none"
    )]
    pub tags: Option<Vec<Thing>>,
    /// 嵌入式规格
    #[serde(skip_serializing_if = "Option::is_none")]
    pub specs: Option<Vec<EmbeddedSpec>>,
}
