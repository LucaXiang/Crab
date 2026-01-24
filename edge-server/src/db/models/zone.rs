//! Zone Model

use super::serde_helpers;
use serde::{Deserialize, Serialize};
use surrealdb::RecordId;

/// Zone entity (区域：大厅、露台、包厢等)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Zone {
    #[serde(default, with = "serde_helpers::option_record_id")]
    pub id: Option<RecordId>,
    pub name: String,
    pub description: Option<String>,
    #[serde(
        default = "default_true",
        deserialize_with = "serde_helpers::bool_true"
    )]
    pub is_active: bool,
}

fn default_true() -> bool {
    true
}

/// Create zone payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZoneCreate {
    pub name: String,
    pub description: Option<String>,
}

/// Update zone payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZoneUpdate {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_active: Option<bool>,
}
