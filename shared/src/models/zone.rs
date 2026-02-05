//! Zone Model

use serde::{Deserialize, Serialize};

/// Zone entity (区域：大厅、露台、包厢等)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Zone {
    pub id: Option<String>,
    pub name: String,
    pub description: Option<String>,
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
}
