//! Category Model

use super::serde_helpers;
use serde::{Deserialize, Serialize};
use surrealdb::RecordId;

pub type CategoryId = RecordId;

/// Category model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Category {
    #[serde(default, with = "serde_helpers::option_record_id")]
    pub id: Option<CategoryId>,
    pub name: String,
    #[serde(default)]
    pub sort_order: i32,
    /// Kitchen print destination references
    #[serde(default, with = "serde_helpers::vec_record_id")]
    pub kitchen_print_destinations: Vec<RecordId>,
    /// Label print destination references
    #[serde(default, with = "serde_helpers::vec_record_id")]
    pub label_print_destinations: Vec<RecordId>,
    /// Whether kitchen printing is enabled for this category
    #[serde(
        default = "default_true",
        deserialize_with = "serde_helpers::bool_true"
    )]
    pub is_kitchen_print_enabled: bool,
    #[serde(
        default = "default_true",
        deserialize_with = "serde_helpers::bool_true"
    )]
    pub is_label_print_enabled: bool,
    #[serde(
        default = "default_true",
        deserialize_with = "serde_helpers::bool_true"
    )]
    pub is_active: bool,
    /// Whether this is a virtual category (filters by tags instead of direct assignment)
    #[serde(default)]
    pub is_virtual: bool,
    /// Tag IDs for virtual category filtering
    #[serde(default, with = "serde_helpers::vec_record_id")]
    pub tag_ids: Vec<RecordId>,
    /// Match mode for virtual category: "any" or "all"
    #[serde(default = "default_match_mode")]
    pub match_mode: String,
    /// Whether to display this category in POS (for virtual categories)
    #[serde(
        default = "default_true",
        deserialize_with = "serde_helpers::bool_true"
    )]
    pub is_display: bool,
}

fn default_true() -> bool {
    true
}

fn default_match_mode() -> String {
    "any".to_string()
}

impl Category {
    pub fn new(name: String) -> Self {
        Self {
            id: None,
            name,
            sort_order: 0,
            kitchen_print_destinations: Vec::new(),
            label_print_destinations: Vec::new(),
            is_kitchen_print_enabled: true,
            is_label_print_enabled: true,
            is_active: true,
            is_virtual: false,
            tag_ids: Vec::new(),
            match_mode: "any".to_string(),
            is_display: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryCreate {
    pub name: String,
    pub sort_order: Option<i32>,
    /// Kitchen print destination IDs
    #[serde(default)]
    pub kitchen_print_destinations: Vec<String>,
    /// Label print destination IDs
    #[serde(default)]
    pub label_print_destinations: Vec<String>,
    /// Whether kitchen printing is enabled
    pub is_kitchen_print_enabled: Option<bool>,
    pub is_label_print_enabled: Option<bool>,
    /// Whether this is a virtual category
    pub is_virtual: Option<bool>,
    /// Tag IDs for virtual category filtering
    #[serde(default)]
    pub tag_ids: Vec<String>,
    /// Match mode: "any" or "all"
    pub match_mode: Option<String>,
    /// Whether to display this category in POS
    pub is_display: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryUpdate {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort_order: Option<i32>,
    /// Kitchen print destination IDs
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kitchen_print_destinations: Option<Vec<String>>,
    /// Label print destination IDs
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label_print_destinations: Option<Vec<String>>,
    /// Whether kitchen printing is enabled
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_kitchen_print_enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_label_print_enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_active: Option<bool>,
    /// Whether this is a virtual category
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_virtual: Option<bool>,
    /// Tag IDs for virtual category filtering
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tag_ids: Option<Vec<String>>,
    /// Match mode: "any" or "all"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub match_mode: Option<String>,
    /// Whether to display this category in POS
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_display: Option<bool>,
}
