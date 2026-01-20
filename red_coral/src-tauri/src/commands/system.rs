//! 系统 API Commands
//!
//! 通过 ClientBridge -> CrabClient -> EdgeServer REST API

use std::sync::Arc;
use tauri::State;
use tokio::sync::RwLock;
use urlencoding::encode;

use crate::core::response::{
    ApiResponse, DeleteData, EmployeeListData, OrderListData, PriceRuleListData, Role,
    RoleListData, RolePermissionListData,
};
use crate::core::ClientBridge;
use shared::models::{
    EmployeeCreate,
    // Employees
    EmployeeResponse,
    EmployeeUpdate,
    InitGenesisRequest,
    // Order for pending sync
    Order,
    // Price Rules
    PriceRule,
    PriceRuleCreate,
    PriceRuleUpdate,
    // System State
    SystemState,
    SystemStateUpdate,
    UpdateLastOrderRequest,
    UpdateSyncStateRequest,
};

// ============ System State ============

#[tauri::command]
pub async fn get_system_state(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
) -> Result<ApiResponse<SystemState>, String> {
    let bridge = bridge.read().await;
    match bridge.get("/api/system-state").await {
        Ok(state) => Ok(ApiResponse::success(state)),
        Err(e) => Ok(ApiResponse::error("SYSTEM_STATE_FAILED", e.to_string())),
    }
}

#[tauri::command]
pub async fn update_system_state(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    data: SystemStateUpdate,
) -> Result<ApiResponse<SystemState>, String> {
    let bridge = bridge.read().await;
    match bridge.put("/api/system-state", &data).await {
        Ok(state) => Ok(ApiResponse::success(state)),
        Err(e) => Ok(ApiResponse::error(
            "SYSTEM_STATE_UPDATE_FAILED",
            e.to_string(),
        )),
    }
}

#[tauri::command]
pub async fn init_genesis(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    data: InitGenesisRequest,
) -> Result<ApiResponse<SystemState>, String> {
    let bridge = bridge.read().await;
    match bridge.post("/api/system-state/genesis", &data).await {
        Ok(state) => Ok(ApiResponse::success(state)),
        Err(e) => Ok(ApiResponse::error("GENESIS_FAILED", e.to_string())),
    }
}

#[tauri::command]
pub async fn update_last_order(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    data: UpdateLastOrderRequest,
) -> Result<ApiResponse<SystemState>, String> {
    let bridge = bridge.read().await;
    match bridge.put("/api/system-state/last-order", &data).await {
        Ok(state) => Ok(ApiResponse::success(state)),
        Err(e) => Ok(ApiResponse::error(
            "UPDATE_LAST_ORDER_FAILED",
            e.to_string(),
        )),
    }
}

#[tauri::command]
pub async fn update_sync_state(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    data: UpdateSyncStateRequest,
) -> Result<ApiResponse<SystemState>, String> {
    let bridge = bridge.read().await;
    match bridge.put("/api/system-state/sync-state", &data).await {
        Ok(state) => Ok(ApiResponse::success(state)),
        Err(e) => Ok(ApiResponse::error(
            "UPDATE_SYNC_STATE_FAILED",
            e.to_string(),
        )),
    }
}

#[tauri::command]
pub async fn get_pending_sync_orders(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
) -> Result<ApiResponse<OrderListData>, String> {
    let bridge = bridge.read().await;
    match bridge
        .get::<Vec<Order>>("/api/system-state/pending-sync")
        .await
    {
        Ok(orders) => Ok(ApiResponse::success(OrderListData { orders })),
        Err(e) => Ok(ApiResponse::error("GET_PENDING_SYNC_FAILED", e.to_string())),
    }
}

// ============ Employees ============

#[tauri::command]
pub async fn list_employees(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
) -> Result<ApiResponse<EmployeeListData>, String> {
    let bridge = bridge.read().await;
    match bridge.get::<Vec<EmployeeResponse>>("/api/employees").await {
        Ok(employees) => Ok(ApiResponse::success(EmployeeListData { employees })),
        Err(e) => Ok(ApiResponse::error("LIST_EMPLOYEES_FAILED", e.to_string())),
    }
}

#[tauri::command]
pub async fn list_all_employees(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
) -> Result<ApiResponse<EmployeeListData>, String> {
    let bridge = bridge.read().await;
    match bridge
        .get::<Vec<EmployeeResponse>>("/api/employees/all")
        .await
    {
        Ok(employees) => Ok(ApiResponse::success(EmployeeListData { employees })),
        Err(e) => Ok(ApiResponse::error(
            "LIST_ALL_EMPLOYEES_FAILED",
            e.to_string(),
        )),
    }
}

#[tauri::command]
pub async fn get_employee(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    id: String,
) -> Result<ApiResponse<EmployeeResponse>, String> {
    let bridge = bridge.read().await;
    match bridge.get(&format!("/api/employees/{}", encode(&id))).await {
        Ok(employee) => Ok(ApiResponse::success(employee)),
        Err(e) => Ok(ApiResponse::error("GET_EMPLOYEE_FAILED", e.to_string())),
    }
}

#[tauri::command]
pub async fn create_employee(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    data: EmployeeCreate,
) -> Result<ApiResponse<EmployeeResponse>, String> {
    let bridge = bridge.read().await;
    match bridge.post("/api/employees", &data).await {
        Ok(employee) => Ok(ApiResponse::success(employee)),
        Err(e) => Ok(ApiResponse::error("CREATE_EMPLOYEE_FAILED", e.to_string())),
    }
}

#[tauri::command]
pub async fn update_employee(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    id: String,
    data: EmployeeUpdate,
) -> Result<ApiResponse<EmployeeResponse>, String> {
    let bridge = bridge.read().await;
    match bridge
        .put(&format!("/api/employees/{}", encode(&id)), &data)
        .await
    {
        Ok(employee) => Ok(ApiResponse::success(employee)),
        Err(e) => Ok(ApiResponse::error("UPDATE_EMPLOYEE_FAILED", e.to_string())),
    }
}

#[tauri::command]
pub async fn delete_employee(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    id: String,
) -> Result<ApiResponse<DeleteData>, String> {
    let bridge = bridge.read().await;
    match bridge
        .delete::<()>(&format!("/api/employees/{}", encode(&id)))
        .await
    {
        Ok(_) => Ok(ApiResponse::success(DeleteData::success())),
        Err(e) => Ok(ApiResponse::error("DELETE_EMPLOYEE_FAILED", e.to_string())),
    }
}

// ============ Price Rules ============

#[tauri::command]
pub async fn list_price_rules(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
) -> Result<ApiResponse<PriceRuleListData>, String> {
    let bridge = bridge.read().await;
    match bridge.get::<Vec<PriceRule>>("/api/price-rules").await {
        Ok(rules) => Ok(ApiResponse::success(PriceRuleListData { rules })),
        Err(e) => Ok(ApiResponse::error("LIST_PRICE_RULES_FAILED", e.to_string())),
    }
}

#[tauri::command]
pub async fn list_active_price_rules(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
) -> Result<ApiResponse<PriceRuleListData>, String> {
    let bridge = bridge.read().await;
    match bridge
        .get::<Vec<PriceRule>>("/api/price-rules/active")
        .await
    {
        Ok(rules) => Ok(ApiResponse::success(PriceRuleListData { rules })),
        Err(e) => Ok(ApiResponse::error(
            "LIST_ACTIVE_PRICE_RULES_FAILED",
            e.to_string(),
        )),
    }
}

#[tauri::command]
pub async fn get_price_rule(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    id: String,
) -> Result<ApiResponse<PriceRule>, String> {
    let bridge = bridge.read().await;
    match bridge
        .get(&format!("/api/price-rules/{}", encode(&id)))
        .await
    {
        Ok(rule) => Ok(ApiResponse::success(rule)),
        Err(e) => Ok(ApiResponse::error("GET_PRICE_RULE_FAILED", e.to_string())),
    }
}

#[tauri::command]
pub async fn create_price_rule(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    data: PriceRuleCreate,
) -> Result<ApiResponse<PriceRule>, String> {
    let bridge = bridge.read().await;
    match bridge.post("/api/price-rules", &data).await {
        Ok(rule) => Ok(ApiResponse::success(rule)),
        Err(e) => Ok(ApiResponse::error(
            "CREATE_PRICE_RULE_FAILED",
            e.to_string(),
        )),
    }
}

#[tauri::command]
pub async fn update_price_rule(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    id: String,
    data: PriceRuleUpdate,
) -> Result<ApiResponse<PriceRule>, String> {
    let bridge = bridge.read().await;
    match bridge
        .put(&format!("/api/price-rules/{}", encode(&id)), &data)
        .await
    {
        Ok(rule) => Ok(ApiResponse::success(rule)),
        Err(e) => Ok(ApiResponse::error(
            "UPDATE_PRICE_RULE_FAILED",
            e.to_string(),
        )),
    }
}

#[tauri::command]
pub async fn delete_price_rule(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    id: String,
) -> Result<ApiResponse<DeleteData>, String> {
    let bridge = bridge.read().await;
    match bridge
        .delete::<()>(&format!("/api/price-rules/{}", encode(&id)))
        .await
    {
        Ok(_) => Ok(ApiResponse::success(DeleteData::success())),
        Err(e) => Ok(ApiResponse::error(
            "DELETE_PRICE_RULE_FAILED",
            e.to_string(),
        )),
    }
}

// ============ Roles ============

#[tauri::command]
pub async fn list_roles(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
) -> Result<ApiResponse<RoleListData>, String> {
    let bridge = bridge.read().await;
    match bridge.get::<Vec<Role>>("/api/roles").await {
        Ok(roles) => Ok(ApiResponse::success(RoleListData { roles })),
        Err(e) => Ok(ApiResponse::error("LIST_ROLES_FAILED", e.to_string())),
    }
}

#[tauri::command]
pub async fn get_role(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    id: String,
) -> Result<ApiResponse<Role>, String> {
    let bridge = bridge.read().await;
    match bridge.get(&format!("/api/roles/{}", encode(&id))).await {
        Ok(role) => Ok(ApiResponse::success(role)),
        Err(e) => Ok(ApiResponse::error("GET_ROLE_FAILED", e.to_string())),
    }
}

#[tauri::command]
pub async fn create_role(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    data: serde_json::Value,
) -> Result<ApiResponse<Role>, String> {
    let bridge = bridge.read().await;
    match bridge.post("/api/roles", &data).await {
        Ok(role) => Ok(ApiResponse::success(role)),
        Err(e) => Ok(ApiResponse::error("CREATE_ROLE_FAILED", e.to_string())),
    }
}

#[tauri::command]
pub async fn update_role(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    id: String,
    data: serde_json::Value,
) -> Result<ApiResponse<Role>, String> {
    let bridge = bridge.read().await;
    match bridge
        .put(&format!("/api/roles/{}", encode(&id)), &data)
        .await
    {
        Ok(role) => Ok(ApiResponse::success(role)),
        Err(e) => Ok(ApiResponse::error("UPDATE_ROLE_FAILED", e.to_string())),
    }
}

#[tauri::command]
pub async fn delete_role(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    id: String,
) -> Result<ApiResponse<DeleteData>, String> {
    let bridge = bridge.read().await;
    match bridge
        .delete::<()>(&format!("/api/roles/{}", encode(&id)))
        .await
    {
        Ok(_) => Ok(ApiResponse::success(DeleteData::success())),
        Err(e) => Ok(ApiResponse::error("DELETE_ROLE_FAILED", e.to_string())),
    }
}

#[tauri::command]
pub async fn get_role_permissions(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    role_id: String,
) -> Result<ApiResponse<RolePermissionListData>, String> {
    let bridge = bridge.read().await;
    match bridge
        .get(&format!("/api/roles/{}/permissions", encode(&role_id)))
        .await
    {
        Ok(permissions) => Ok(ApiResponse::success(RolePermissionListData { permissions })),
        Err(e) => Ok(ApiResponse::error(
            "GET_ROLE_PERMISSIONS_FAILED",
            e.to_string(),
        )),
    }
}

#[tauri::command]
pub async fn get_all_permissions(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
) -> Result<ApiResponse<Vec<String>>, String> {
    let bridge = bridge.read().await;
    match bridge.get::<Vec<String>>("/api/permissions").await {
        Ok(permissions) => Ok(ApiResponse::success(permissions)),
        Err(e) => Ok(ApiResponse::error(
            "GET_ALL_PERMISSIONS_FAILED",
            e.to_string(),
        )),
    }
}

#[tauri::command]
pub async fn update_role_permissions(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    role_id: String,
    permissions: Vec<String>,
) -> Result<ApiResponse<()>, String> {
    let bridge = bridge.read().await;
    match bridge
        .put::<(), _>(
            &format!("/api/roles/{}/permissions", encode(&role_id)),
            &permissions,
        )
        .await
    {
        Ok(_) => Ok(ApiResponse::success(())),
        Err(e) => Ok(ApiResponse::error(
            "UPDATE_ROLE_PERMISSIONS_FAILED",
            e.to_string(),
        )),
    }
}
