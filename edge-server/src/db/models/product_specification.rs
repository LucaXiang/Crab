//! Product Specification Model

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use surrealdb::sql::Thing;

pub type ProductSpecificationId = Thing;

/// Product Specification model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductSpecification {
    pub id: Option<ProductSpecificationId>,
    /// Record link to product
    pub product: Thing,
    pub name: String,
    /// Price in cents
    #[serde(default)]
    pub price: i64,
    #[serde(default)]
    pub display_order: i32,
    #[serde(default)]
    pub is_default: bool,
    #[serde(default = "default_true")]
    pub is_active: bool,
    /// Is root spec (single-spec product's only spec)
    #[serde(default)]
    pub is_root: bool,
    pub external_id: Option<i64>,
    /// Array of record links to tags (Graph DB style)
    #[serde(default)]
    pub tags: Vec<Thing>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

fn default_true() -> bool {
    true
}

impl ProductSpecification {
    pub fn new(product: Thing, name: String, price: i64) -> Self {
        Self {
            id: None,
            product,
            name,
            price,
            display_order: 0,
            is_default: false,
            is_active: true,
            is_root: false,
            external_id: None,
            tags: vec![],
            created_at: None,
            updated_at: None,
        }
    }

    pub fn root(product: Thing, name: String, price: i64) -> Self {
        Self {
            id: None,
            product,
            name,
            price,
            display_order: 0,
            is_default: true,
            is_active: true,
            is_root: true,
            external_id: None,
            tags: vec![],
            created_at: None,
            updated_at: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductSpecificationCreate {
    pub product: Thing,
    pub name: String,
    pub price: i64,
    pub display_order: Option<i32>,
    pub is_default: Option<bool>,
    pub is_root: Option<bool>,
    pub external_id: Option<i64>,
    pub tags: Option<Vec<Thing>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductSpecificationUpdate {
    pub name: Option<String>,
    pub price: Option<i64>,
    pub display_order: Option<i32>,
    pub is_default: Option<bool>,
    pub is_active: Option<bool>,
    pub is_root: Option<bool>,
    pub external_id: Option<i64>,
    pub tags: Option<Vec<Thing>>,
}
