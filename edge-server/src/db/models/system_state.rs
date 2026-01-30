//! System State Model (Singleton)
//!
//! 内部使用，不暴露给前端 API，直接使用原生 RecordId。

use serde::{Deserialize, Serialize};
use surrealdb::RecordId;

/// System state entity (哈希链状态缓存)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemState {
    pub id: Option<RecordId>,
    /// Genesis hash
    pub genesis_hash: Option<String>,
    /// Last order reference
    pub last_order: Option<RecordId>,
    pub last_order_hash: Option<String>,
    /// Sync state
    pub synced_up_to: Option<RecordId>,
    pub synced_up_to_hash: Option<String>,
    pub last_sync_time: Option<i64>,
    /// Statistics
    pub order_count: i32,
    pub created_at: i64,
    pub updated_at: i64,
}

/// Update system state payload
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SystemStateUpdate {
    pub genesis_hash: Option<String>,
    pub last_order: Option<RecordId>,
    pub last_order_hash: Option<String>,
    pub synced_up_to: Option<RecordId>,
    pub synced_up_to_hash: Option<String>,
    pub last_sync_time: Option<i64>,
    pub order_count: Option<i32>,
}
