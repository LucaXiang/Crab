//! Tag Model

use super::serde_helpers;
use serde::{Deserialize, Serialize};
use surrealdb::RecordId;

pub type TagId = RecordId;

/// Tag model matching SurrealDB schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tag {
    #[serde(default, with = "serde_helpers::option_record_id")]
    pub id: Option<TagId>,
    pub name: String,
    pub color: String,
    pub display_order: i32,
    pub is_active: bool,
    /// System tag (e.g., "热卖", "新品"), cannot be deleted
    pub is_system: bool,
}

impl Tag {
    pub fn new(name: String) -> Self {
        Self {
            id: None,
            name,
            color: "#3B82F6".to_string(),
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
