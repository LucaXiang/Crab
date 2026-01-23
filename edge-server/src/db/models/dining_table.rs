//! Dining Table Model

use super::serde_helpers;
use super::serde_thing;
use serde::{Deserialize, Serialize};
use surrealdb::sql::Thing;

/// Dining table entity (桌台)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiningTable {
    #[serde(default, with = "serde_thing::option")]
    pub id: Option<Thing>,
    pub name: String,
    /// Zone reference
    #[serde(with = "serde_thing")]
    pub zone: Thing,
    #[serde(default)]
    pub capacity: i32,
    #[serde(
        default = "default_true",
        deserialize_with = "serde_helpers::bool_true"
    )]
    pub is_active: bool,
}

fn default_true() -> bool {
    true
}

/// Create dining table payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiningTableCreate {
    pub name: String,
    #[serde(with = "super::serde_thing")]
    pub zone: Thing,
    pub capacity: Option<i32>,
}

/// Update dining table payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiningTableUpdate {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, with = "super::serde_thing::option", skip_serializing_if = "Option::is_none")]
    pub zone: Option<Thing>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capacity: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_active: Option<bool>,
}
