//! 系统 API Commands
//!
//! 通过 ClientBridge -> CrabClient -> EdgeServer REST API

use std::sync::Arc;
use tauri::State;
use tokio::sync::RwLock;

use crate::core::ClientBridge;
use shared::models::{
    // System State
    SystemState, SystemStateUpdate,
    InitGenesisRequest, UpdateLastOrderRequest, UpdateSyncStateRequest,
    // Employees
    EmployeeResponse, EmployeeCreate, EmployeeUpdate,
    // Price Rules
    PriceRule, PriceRuleCreate, PriceRuleUpdate,
    // Order for pending sync
    Order,
};

// ============ System State ============

#[tauri::command]
pub async fn get_system_state(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
) -> Result<SystemState, String> {
    let bridge = bridge.read().await;
    bridge.get("/api/system-state").await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_system_state(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    data: SystemStateUpdate,
) -> Result<SystemState, String> {
    let bridge = bridge.read().await;
    bridge.put("/api/system-state", &data).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn init_genesis(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    data: InitGenesisRequest,
) -> Result<SystemState, String> {
    let bridge = bridge.read().await;
    bridge.post("/api/system-state/genesis", &data).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_last_order(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    data: UpdateLastOrderRequest,
) -> Result<SystemState, String> {
    let bridge = bridge.read().await;
    bridge.put("/api/system-state/last-order", &data).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_sync_state(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    data: UpdateSyncStateRequest,
) -> Result<SystemState, String> {
    let bridge = bridge.read().await;
    bridge.put("/api/system-state/sync-state", &data).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_pending_sync_orders(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
) -> Result<Vec<Order>, String> {
    let bridge = bridge.read().await;
    bridge.get("/api/system-state/pending-sync").await.map_err(|e| e.to_string())
}

// ============ Employees ============

#[tauri::command]
pub async fn list_employees(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
) -> Result<Vec<EmployeeResponse>, String> {
    let bridge = bridge.read().await;
    bridge.get("/api/employees").await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_all_employees(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
) -> Result<Vec<EmployeeResponse>, String> {
    let bridge = bridge.read().await;
    bridge.get("/api/employees/all").await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_employee(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    id: String,
) -> Result<EmployeeResponse, String> {
    let bridge = bridge.read().await;
    bridge.get(&format!("/api/employees/{}", id)).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn create_employee(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    data: EmployeeCreate,
) -> Result<EmployeeResponse, String> {
    let bridge = bridge.read().await;
    bridge.post("/api/employees", &data).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_employee(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    id: String,
    data: EmployeeUpdate,
) -> Result<EmployeeResponse, String> {
    let bridge = bridge.read().await;
    bridge.put(&format!("/api/employees/{}", id), &data).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_employee(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    id: String,
) -> Result<bool, String> {
    let bridge = bridge.read().await;
    bridge.delete(&format!("/api/employees/{}", id)).await.map_err(|e| e.to_string())
}

// ============ Price Rules ============

#[tauri::command]
pub async fn list_price_rules(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
) -> Result<Vec<PriceRule>, String> {
    let bridge = bridge.read().await;
    bridge.get("/api/price-rules").await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_active_price_rules(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
) -> Result<Vec<PriceRule>, String> {
    let bridge = bridge.read().await;
    bridge.get("/api/price-rules/active").await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_price_rule(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    id: String,
) -> Result<PriceRule, String> {
    let bridge = bridge.read().await;
    bridge.get(&format!("/api/price-rules/{}", id)).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn create_price_rule(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    data: PriceRuleCreate,
) -> Result<PriceRule, String> {
    let bridge = bridge.read().await;
    bridge.post("/api/price-rules", &data).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_price_rule(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    id: String,
    data: PriceRuleUpdate,
) -> Result<PriceRule, String> {
    let bridge = bridge.read().await;
    bridge.put(&format!("/api/price-rules/{}", id), &data).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_price_rule(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    id: String,
) -> Result<bool, String> {
    let bridge = bridge.read().await;
    bridge.delete(&format!("/api/price-rules/{}", id)).await.map_err(|e| e.to_string())
}

// ============ Roles ============

#[tauri::command]
pub async fn list_roles(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
) -> Result<Vec<serde_json::Value>, String> {
    let bridge = bridge.read().await;
    bridge.get("/api/roles").await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_role(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    id: String,
) -> Result<serde_json::Value, String> {
    let bridge = bridge.read().await;
    bridge.get(&format!("/api/roles/{}", id)).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn create_role(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    data: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let bridge = bridge.read().await;
    bridge.post("/api/roles", &data).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_role(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    id: String,
    data: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let bridge = bridge.read().await;
    bridge.put(&format!("/api/roles/{}", id), &data).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_role(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    id: String,
) -> Result<bool, String> {
    let bridge = bridge.read().await;
    bridge.delete(&format!("/api/roles/{}", id)).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_role_permissions(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    role_id: String,
) -> Result<serde_json::Value, String> {
    let bridge = bridge.read().await;
    bridge.get(&format!("/api/roles/{}/permissions", role_id)).await.map_err(|e| e.to_string())
}
