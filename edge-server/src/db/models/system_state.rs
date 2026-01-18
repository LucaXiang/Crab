//! System State Model (Singleton)

use serde::{Deserialize, Serialize};
use surrealdb::sql::Thing;

/// System state entity (哈希链状态缓存)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemState {
    pub id: Option<Thing>,
    /// Genesis hash
    pub genesis_hash: Option<String>,
    /// Last order reference
    pub last_order: Option<Thing>,
    pub last_order_hash: Option<String>,
    /// Sync state
    pub synced_up_to: Option<Thing>,
    pub synced_up_to_hash: Option<String>,
    pub last_sync_time: Option<String>,
    /// Statistics
    pub order_count: i32,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

/// Update system state payload
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SystemStateUpdate {
    pub genesis_hash: Option<String>,
    pub last_order: Option<Thing>,
    pub last_order_hash: Option<String>,
    pub synced_up_to: Option<Thing>,
    pub synced_up_to_hash: Option<String>,
    pub last_sync_time: Option<String>,
    pub order_count: Option<i32>,
}
