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
    #[serde(default)]
    pub price: i64,
    #[serde(default)]
    pub display_order: i32,
    #[serde(default)]
    pub is_default: bool,
    #[serde(default = "default_true")]
    pub is_active: bool,
    pub external_id: Option<i64>,
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
    /// Record link to kitchen_printer (override category setting)
    pub kitchen_printer: Option<Thing>,
    /// -1=inherit, 0=disabled, 1=enabled
    #[serde(default = "default_inherit")]
    pub is_kitchen_print_enabled: i32,
    #[serde(default = "default_inherit")]
    pub is_label_print_enabled: i32,
    #[serde(default = "default_true", deserialize_with = "serde_helpers::bool_true")]
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
            kitchen_printer: None,
            is_kitchen_print_enabled: -1,
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
    pub price: Option<i64>,
    pub sort_order: Option<i32>,
    pub tax_rate: Option<i32>,
    pub receipt_name: Option<String>,
    pub kitchen_print_name: Option<String>,
    #[serde(default, with = "serde_thing::option")]
    pub kitchen_printer: Option<Thing>,
    pub is_kitchen_print_enabled: Option<i32>,
    pub is_label_print_enabled: Option<i32>,
    #[serde(default, with = "serde_thing::option_vec")]
    pub tags: Option<Vec<Thing>>,
    /// 嵌入式规格 (创建时可选，默认创建 root spec)
    #[serde(default)]
    pub specs: Option<Vec<EmbeddedSpec>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductUpdate {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
    #[serde(default, with = "serde_thing::option", skip_serializing_if = "Option::is_none")]
    pub category: Option<Thing>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort_order: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tax_rate: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub receipt_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kitchen_print_name: Option<String>,
    #[serde(default, with = "serde_thing::option", skip_serializing_if = "Option::is_none")]
    pub kitchen_printer: Option<Thing>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_kitchen_print_enabled: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_label_print_enabled: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_active: Option<bool>,
    #[serde(default, with = "serde_thing::option_vec", skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<Thing>>,
    /// 嵌入式规格
    #[serde(skip_serializing_if = "Option::is_none")]
    pub specs: Option<Vec<EmbeddedSpec>>,
}
