//! Zone Model

use serde::{Deserialize, Serialize};
use surrealdb::sql::Thing;

/// Zone entity (区域：大厅、露台、包厢等)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Zone {
    pub id: Option<Thing>,
    pub name: String,
    pub description: Option<String>,
    pub is_active: bool,
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
    pub name: Option<String>,
    pub description: Option<String>,
    pub is_active: Option<bool>,
}
