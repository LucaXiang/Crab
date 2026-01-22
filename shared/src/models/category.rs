//! Category Model

use serde::{Deserialize, Serialize};

/// Category entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Category {
    pub id: Option<String>,
    pub name: String,
    pub sort_order: i32,
    /// Kitchen print destination references (String IDs)
    #[serde(default)]
    pub kitchen_print_destinations: Vec<String>,
    /// Label print destination references (String IDs)
    #[serde(default)]
    pub label_print_destinations: Vec<String>,
    /// Whether kitchen printing is enabled for this category
    #[serde(default)]
    pub is_kitchen_print_enabled: bool,
    pub is_label_print_enabled: bool,
    pub is_active: bool,
    /// Whether this is a virtual category (filters by tags instead of direct assignment)
    #[serde(default)]
    pub is_virtual: bool,
    /// Tag IDs for virtual category filtering
    #[serde(default)]
    pub tag_ids: Vec<String>,
    /// Match mode for virtual category: "any" or "all"
    #[serde(default = "default_match_mode")]
    pub match_mode: String,
}

fn default_match_mode() -> String {
    "any".to_string()
}

/// Create category payload
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
}

/// Update category payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryUpdate {
    pub name: Option<String>,
    pub sort_order: Option<i32>,
    /// Kitchen print destination IDs
    #[serde(default)]
    pub kitchen_print_destinations: Option<Vec<String>>,
    /// Label print destination IDs
    #[serde(default)]
    pub label_print_destinations: Option<Vec<String>>,
    /// Whether kitchen printing is enabled
    pub is_kitchen_print_enabled: Option<bool>,
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
