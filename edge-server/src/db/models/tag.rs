//! Tag Model

use serde::{Deserialize, Serialize};
use surrealdb::sql::Thing;

pub type TagId = Thing;

/// Tag model matching SurrealDB schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tag {
    pub id: Option<TagId>,
    pub name: String,
    #[serde(default = "default_color")]
    pub color: String,
    #[serde(default)]
    pub display_order: i32,
    #[serde(default = "default_true")]
    pub is_active: bool,
}

fn default_color() -> String {
    "#3B82F6".to_string()
}

fn default_true() -> bool {
    true
}

impl Tag {
    pub fn new(name: String) -> Self {
        Self {
            id: None,
            name,
            color: default_color(),
            display_order: 0,
            is_active: true,
        }
    }
}

/// Tag for creation (without id)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagCreate {
    pub name: String,
    pub color: Option<String>,
    pub display_order: Option<i32>,
}

/// Tag for update (all optional)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagUpdate {
    pub name: Option<String>,
    pub color: Option<String>,
    pub display_order: Option<i32>,
    pub is_active: Option<bool>,
}
