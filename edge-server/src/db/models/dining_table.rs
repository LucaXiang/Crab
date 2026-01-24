//! Dining Table Model

use super::serde_helpers;
use serde::{Deserialize, Serialize};
use surrealdb::RecordId;

/// Dining table entity (桌台)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiningTable {
    #[serde(default, with = "serde_helpers::option_record_id")]
    pub id: Option<RecordId>,
    pub name: String,
    /// Zone reference
    #[serde(with = "serde_helpers::record_id")]
    pub zone: RecordId,
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
    #[serde(with = "serde_helpers::record_id")]
    pub zone: RecordId,
    pub capacity: Option<i32>,
}

/// Update dining table payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiningTableUpdate {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "serde_helpers::option_record_id"
    )]
    pub zone: Option<RecordId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capacity: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_active: Option<bool>,
}
