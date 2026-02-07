//! System State Model

use serde::{Deserialize, Serialize};

/// System state entity (哈希链状态缓存)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "db", derive(sqlx::FromRow))]
pub struct SystemState {
    pub id: i64,
    pub genesis_hash: Option<String>,
    pub last_order_id: Option<String>,
    pub last_order_hash: Option<String>,
    pub synced_up_to_id: Option<String>,
    pub synced_up_to_hash: Option<String>,
    pub last_sync_time: Option<i64>,
    pub order_count: i32,
    pub created_at: i64,
    pub updated_at: i64,
}

/// Update system state payload
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SystemStateUpdate {
    pub genesis_hash: Option<String>,
    pub last_order_id: Option<String>,
    pub last_order_hash: Option<String>,
    pub synced_up_to_id: Option<String>,
    pub synced_up_to_hash: Option<String>,
    pub last_sync_time: Option<i64>,
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
