//! Category Model

use super::serde_helpers;
use serde::{Deserialize, Serialize};
use surrealdb::sql::Thing;

pub type CategoryId = Thing;

/// Category model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Category {
    pub id: Option<CategoryId>,
    pub name: String,
    #[serde(default)]
    pub sort_order: i32,
    /// Record links to print_destination
    #[serde(default)]
    pub print_destinations: Vec<Thing>,
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
    #[serde(default)]
    pub tag_ids: Vec<Thing>,
    /// Match mode for virtual category: "any" or "all"
    #[serde(default = "default_match_mode")]
    pub match_mode: String,
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
            print_destinations: Vec::new(),
            is_label_print_enabled: true,
            is_active: true,
            is_virtual: false,
            tag_ids: Vec::new(),
            match_mode: "any".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryCreate {
    pub name: String,
    pub sort_order: Option<i32>,
    /// Print destination IDs (strings)
    #[serde(default)]
    pub print_destinations: Vec<String>,
    pub is_label_print_enabled: Option<bool>,
    /// Whether this is a virtual category
    pub is_virtual: Option<bool>,
    /// Tag IDs for virtual category filtering
    #[serde(default)]
    pub tag_ids: Vec<String>,
    /// Match mode: "any" or "all"
    pub match_mode: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryUpdate {
    pub name: Option<String>,
    pub sort_order: Option<i32>,
    /// Print destination IDs (strings)
    #[serde(default)]
    pub print_destinations: Option<Vec<String>>,
    pub is_label_print_enabled: Option<bool>,
    pub is_active: Option<bool>,
    /// Whether this is a virtual category
    pub is_virtual: Option<bool>,
    /// Tag IDs for virtual category filtering
    #[serde(default)]
    pub tag_ids: Option<Vec<String>>,
    /// Match mode: "any" or "all"
    pub match_mode: Option<String>,
}
