//! Tag Model

use serde::{Deserialize, Serialize};

/// Tag entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tag {
    pub id: Option<String>,
    pub name: String,
    pub color: String,
    pub display_order: i32,
    pub is_active: bool,
    /// 系统标签（"热卖"、"新品"等），不可删除/改名
    #[serde(default)]
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
