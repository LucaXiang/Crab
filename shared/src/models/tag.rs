//! Tag Model

use serde::{Deserialize, Serialize};

/// Tag entity
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "db", derive(sqlx::FromRow))]
pub struct Tag {
    pub id: i64,
    pub name: String,
    pub color: String,
    pub display_order: i32,
    pub is_active: bool,
    pub is_system: bool,
}

/// Create tag payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagCreate {
    pub name: String,
    pub color: Option<String>,
    pub display_order: Option<i32>,
}

/// Update tag payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagUpdate {
    pub name: Option<String>,
    pub color: Option<String>,
    pub display_order: Option<i32>,
    pub is_active: Option<bool>,
}
