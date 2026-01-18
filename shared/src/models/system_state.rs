//! System State Model

use serde::{Deserialize, Serialize};

/// System state entity (哈希链状态缓存)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemState {
    pub id: Option<String>,
    pub genesis_hash: Option<String>,
    /// Last order reference (String ID)
    pub last_order: Option<String>,
    pub last_order_hash: Option<String>,
    /// Sync state reference (String ID)
    pub synced_up_to: Option<String>,
    pub synced_up_to_hash: Option<String>,
    pub last_sync_time: Option<String>,
    pub order_count: i32,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

/// Update system state payload
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SystemStateUpdate {
    pub genesis_hash: Option<String>,
    pub last_order: Option<String>,
    pub last_order_hash: Option<String>,
    pub synced_up_to: Option<String>,
    pub synced_up_to_hash: Option<String>,
    pub last_sync_time: Option<String>,
    pub order_count: Option<i32>,
}

/// Init genesis payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitGenesisRequest {
    pub genesis_hash: String,
}

/// Update last order payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateLastOrderRequest {
    pub order_id: String,
    pub order_hash: String,
}

/// Update sync state payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateSyncStateRequest {
    pub synced_up_to_id: String,
    pub synced_up_to_hash: String,
}
