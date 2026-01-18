//! Dining Table Model

use serde::{Deserialize, Serialize};
use surrealdb::sql::Thing;

/// Dining table entity (桌台)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiningTable {
    pub id: Option<Thing>,
    pub name: String,
    pub zone: Thing,
    pub capacity: i32,
    pub is_active: bool,
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
