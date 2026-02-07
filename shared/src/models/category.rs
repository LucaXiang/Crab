//! Category Model

use serde::{Deserialize, Serialize};

/// Category entity
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "db", derive(sqlx::FromRow))]
pub struct Category {
    pub id: i64,
    pub name: String,
    pub sort_order: i32,
    pub is_kitchen_print_enabled: bool,
    pub is_label_print_enabled: bool,
    pub is_active: bool,
    /// Whether this is a virtual category (filters by tags instead of direct assignment)
    pub is_virtual: bool,
    /// Match mode for virtual category: "any" or "all"
    #[serde(default = "default_match_mode")]
    pub match_mode: String,
    /// Whether this category is visible in the POS display
    #[serde(default = "default_true")]
    pub is_display: bool,

    // -- Relations (populated by application code, skipped by FromRow) --

    /// Kitchen print destination IDs (junction table)
    #[cfg_attr(feature = "db", sqlx(skip))]
    #[serde(default)]
    pub kitchen_print_destinations: Vec<i64>,
    /// Label print destination IDs (junction table)
    #[cfg_attr(feature = "db", sqlx(skip))]
    #[serde(default)]
    pub label_print_destinations: Vec<i64>,
    /// Tag IDs for virtual category filtering (junction table)
    #[cfg_attr(feature = "db", sqlx(skip))]
    #[serde(default)]
    pub tag_ids: Vec<i64>,
}

fn default_match_mode() -> String {
    "any".to_string()
}

fn default_true() -> bool {
    true
}

/// Create category payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryCreate {
    pub name: String,
    pub sort_order: Option<i32>,
    #[serde(default)]
    pub kitchen_print_destinations: Vec<i64>,
    #[serde(default)]
    pub label_print_destinations: Vec<i64>,
    pub is_kitchen_print_enabled: Option<bool>,
    pub is_label_print_enabled: Option<bool>,
    pub is_virtual: Option<bool>,
    #[serde(default)]
    pub tag_ids: Vec<i64>,
    pub match_mode: Option<String>,
    pub is_display: Option<bool>,
}

/// Update category payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryUpdate {
    pub name: Option<String>,
    pub sort_order: Option<i32>,
    pub kitchen_print_destinations: Option<Vec<i64>>,
    pub label_print_destinations: Option<Vec<i64>>,
    pub is_kitchen_print_enabled: Option<bool>,
    pub is_label_print_enabled: Option<bool>,
    pub is_virtual: Option<bool>,
    pub tag_ids: Option<Vec<i64>>,
    pub match_mode: Option<String>,
    pub is_active: Option<bool>,
    pub is_display: Option<bool>,
}
