//! Dining Table Model

use super::serde_helpers;
use serde::{Deserialize, Serialize};
use surrealdb::sql::Thing;

/// Dining table entity (桌台)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiningTable {
    pub id: Option<Thing>,
    pub name: String,
    pub zone: Thing,
    #[serde(default)]
    pub capacity: i32,
    #[serde(default = "default_true", deserialize_with = "serde_helpers::bool_true")]
    pub is_active: bool,
}

fn default_true() -> bool {
    true
}

/// Create dining table payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiningTableCreate {
    pub name: String,
    pub zone: Thing,
    pub capacity: Option<i32>,
}

/// Update dining table payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiningTableUpdate {
    pub name: Option<String>,
    pub zone: Option<Thing>,
    pub capacity: Option<i32>,
    pub is_active: Option<bool>,
}
