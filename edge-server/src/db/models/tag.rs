//! Tag Model

use super::serde_helpers;
use super::serde_thing;
use serde::{Deserialize, Serialize};
use surrealdb::sql::Thing;

pub type TagId = Thing;

/// Tag model matching SurrealDB schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tag {
    #[serde(default, with = "serde_thing::option")]
    pub id: Option<TagId>,
    pub name: String,
    #[serde(default = "default_color")]
    pub color: String,
    #[serde(default)]
    pub display_order: i32,
    #[serde(
        default = "default_true",
        deserialize_with = "serde_helpers::bool_true"
    )]
    pub is_active: bool,
    /// System tag (e.g., "热卖", "新品"), cannot be deleted
    #[serde(default, deserialize_with = "serde_helpers::bool_false")]
    pub is_system: bool,
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
            is_system: false,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_order: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_active: Option<bool>,
}
