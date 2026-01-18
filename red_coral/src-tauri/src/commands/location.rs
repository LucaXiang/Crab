//! 位置 API Commands (Zones, Tables)
//!
//! 通过 ClientBridge -> CrabClient -> EdgeServer REST API

use std::sync::Arc;
use tauri::State;
use tokio::sync::RwLock;

use crate::core::ClientBridge;
use shared::models::{
    Zone, ZoneCreate, ZoneUpdate,
    DiningTable, DiningTableCreate, DiningTableUpdate,
};

// ============ Zones ============

#[tauri::command]
pub async fn list_zones(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
) -> Result<Vec<Zone>, String> {
    let bridge = bridge.read().await;
    bridge.get("/api/zones").await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_zone(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    id: String,
) -> Result<Zone, String> {
    let bridge = bridge.read().await;
    bridge.get(&format!("/api/zones/{}", id)).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn create_zone(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    data: ZoneCreate,
) -> Result<Zone, String> {
    let bridge = bridge.read().await;
    bridge.post("/api/zones", &data).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_zone(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    id: String,
    data: ZoneUpdate,
) -> Result<Zone, String> {
    let bridge = bridge.read().await;
    bridge.put(&format!("/api/zones/{}", id), &data).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_zone(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    id: String,
) -> Result<bool, String> {
    let bridge = bridge.read().await;
    bridge.delete(&format!("/api/zones/{}", id)).await.map_err(|e| e.to_string())
}

// ============ Dining Tables ============

#[tauri::command]
pub async fn list_tables(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
) -> Result<Vec<DiningTable>, String> {
    let bridge = bridge.read().await;
    bridge.get("/api/tables").await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_tables_by_zone(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    zone_id: String,
) -> Result<Vec<DiningTable>, String> {
    let bridge = bridge.read().await;
    bridge.get(&format!("/api/tables/zone/{}", zone_id)).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_table(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    id: String,
) -> Result<DiningTable, String> {
    let bridge = bridge.read().await;
    bridge.get(&format!("/api/tables/{}", id)).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn create_table(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    data: DiningTableCreate,
) -> Result<DiningTable, String> {
    let bridge = bridge.read().await;
    bridge.post("/api/tables", &data).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_table(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    id: String,
    data: DiningTableUpdate,
) -> Result<DiningTable, String> {
    let bridge = bridge.read().await;
    bridge.put(&format!("/api/tables/{}", id), &data).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_table(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    id: String,
) -> Result<bool, String> {
    let bridge = bridge.read().await;
    bridge.delete(&format!("/api/tables/{}", id)).await.map_err(|e| e.to_string())
}
