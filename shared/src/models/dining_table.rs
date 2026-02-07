//! Dining Table Model

use serde::{Deserialize, Serialize};

/// Dining table entity (桌台)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "db", derive(sqlx::FromRow))]
pub struct DiningTable {
    pub id: i64,
    pub name: String,
    pub zone_id: i64,
    pub capacity: i32,
    pub is_active: bool,
}

/// Create dining table payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiningTableCreate {
    pub name: String,
    pub zone_id: i64,
    pub capacity: Option<i32>,
}

/// Update dining table payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiningTableUpdate {
    pub name: Option<String>,
    pub zone_id: Option<i64>,
    pub capacity: Option<i32>,
    pub is_active: Option<bool>,
}
