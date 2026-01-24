//! System State Model (Singleton)

use super::serde_helpers;
use serde::{Deserialize, Serialize};
use surrealdb::RecordId;

/// System state entity (哈希链状态缓存)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemState {
    #[serde(default, with = "serde_helpers::option_record_id")]
    pub id: Option<RecordId>,
    /// Genesis hash
    pub genesis_hash: Option<String>,
    /// Last order reference
    #[serde(default, with = "serde_helpers::option_record_id")]
    pub last_order: Option<RecordId>,
    pub last_order_hash: Option<String>,
    /// Sync state
    #[serde(default, with = "serde_helpers::option_record_id")]
    pub synced_up_to: Option<RecordId>,
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
    #[serde(default, with = "serde_helpers::option_record_id")]
    pub last_order: Option<RecordId>,
    pub last_order_hash: Option<String>,
    #[serde(default, with = "serde_helpers::option_record_id")]
    pub synced_up_to: Option<RecordId>,
    pub synced_up_to_hash: Option<String>,
    pub last_sync_time: Option<String>,
    pub order_count: Option<i32>,
}
