//! 系统 API Commands
//!
//! 通过 ClientBridge -> CrabClient -> EdgeServer REST API

use std::sync::Arc;
use tauri::State;

use crate::core::response::{ApiResponse, DeleteData, ErrorCode, RolePermission};
use crate::core::ClientBridge;
use shared::models::{
    Employee, EmployeeCreate, EmployeeUpdate, InitGenesisRequest, LabelTemplate,
    LabelTemplateCreate, LabelTemplateUpdate, PriceRule, PriceRuleCreate, PriceRuleUpdate, Role,
    RoleCreate, RoleUpdate, StoreInfo, StoreInfoUpdate, SystemState, SystemStateUpdate,
    UpdateLastOrderRequest, UpdateSyncStateRequest,
};

// ============ System State ============

#[tauri::command]
pub async fn get_system_state(
    bridge: State<'_, Arc<ClientBridge>>,
) -> Result<ApiResponse<SystemState>, String> {
    match bridge.get("/api/system-state").await {
        Ok(state) => Ok(ApiResponse::success(state)),
        Err(e) => Ok(ApiResponse::error_with_code(
            ErrorCode::DatabaseError,
            e.to_string(),
        )),
    }
}

#[tauri::command]
pub async fn update_system_state(
    bridge: State<'_, Arc<ClientBridge>>,
    data: SystemStateUpdate,
) -> Result<ApiResponse<SystemState>, String> {
    match bridge.put("/api/system-state", &data).await {
        Ok(state) => Ok(ApiResponse::success(state)),
        Err(e) => Ok(ApiResponse::error_with_code(
            ErrorCode::DatabaseError,
            e.to_string(),
        )),
    }
}

#[tauri::command]
pub async fn init_genesis(
    bridge: State<'_, Arc<ClientBridge>>,
    data: InitGenesisRequest,
) -> Result<ApiResponse<SystemState>, String> {
    match bridge.post("/api/system-state/genesis", &data).await {
        Ok(state) => Ok(ApiResponse::success(state)),
        Err(e) => Ok(ApiResponse::error_with_code(
            ErrorCode::DatabaseError,
            e.to_string(),
        )),
    }
}

#[tauri::command]
pub async fn update_last_order(
    bridge: State<'_, Arc<ClientBridge>>,
    data: UpdateLastOrderRequest,
) -> Result<ApiResponse<SystemState>, String> {
    match bridge.put("/api/system-state/last-order", &data).await {
        Ok(state) => Ok(ApiResponse::success(state)),
        Err(e) => Ok(ApiResponse::error_with_code(
            ErrorCode::DatabaseError,
            e.to_string(),
        )),
    }
}

#[tauri::command]
pub async fn update_sync_state(
    bridge: State<'_, Arc<ClientBridge>>,
    data: UpdateSyncStateRequest,
) -> Result<ApiResponse<SystemState>, String> {
    match bridge.put("/api/system-state/sync-state", &data).await {
        Ok(state) => Ok(ApiResponse::success(state)),
        Err(e) => Ok(ApiResponse::error_with_code(
            ErrorCode::DatabaseError,
            e.to_string(),
        )),
    }
}

// ============ Store Info ============

#[tauri::command]
pub async fn get_store_info(
    bridge: State<'_, Arc<ClientBridge>>,
) -> Result<ApiResponse<StoreInfo>, String> {
    match bridge.get("/api/store-info").await {
        Ok(info) => Ok(ApiResponse::success(info)),
        Err(e) => Ok(ApiResponse::error_with_code(
            ErrorCode::DatabaseError,
            e.to_string(),
        )),
    }
}

#[tauri::command]
pub async fn update_store_info(
    bridge: State<'_, Arc<ClientBridge>>,
    data: StoreInfoUpdate,
) -> Result<ApiResponse<StoreInfo>, String> {
    match bridge.put("/api/store-info", &data).await {
        Ok(info) => Ok(ApiResponse::success(info)),
        Err(e) => Ok(ApiResponse::error_with_code(
            ErrorCode::DatabaseError,
            e.to_string(),
        )),
    }
}

// ============ Label Templates ============

#[tauri::command]
pub async fn list_label_templates(
    bridge: State<'_, Arc<ClientBridge>>,
) -> Result<ApiResponse<Vec<LabelTemplate>>, String> {
    match bridge.get("/api/label-templates").await {
        Ok(templates) => Ok(ApiResponse::success(templates)),
        Err(e) => Ok(ApiResponse::error_with_code(
            ErrorCode::DatabaseError,
            e.to_string(),
        )),
    }
}

#[tauri::command]
pub async fn get_label_template(
    bridge: State<'_, Arc<ClientBridge>>,
    id: i64,
) -> Result<ApiResponse<LabelTemplate>, String> {
    match bridge.get(&format!("/api/label-templates/{}", id)).await {
        Ok(template) => Ok(ApiResponse::success(template)),
        Err(e) => Ok(ApiResponse::error_with_code(
            ErrorCode::NotFound,
            e.to_string(),
        )),
    }
}

#[tauri::command]
pub async fn create_label_template(
    bridge: State<'_, Arc<ClientBridge>>,
    data: LabelTemplateCreate,
) -> Result<ApiResponse<LabelTemplate>, String> {
    match bridge.post("/api/label-templates", &data).await {
        Ok(template) => Ok(ApiResponse::success(template)),
        Err(e) => Ok(ApiResponse::error_with_code(
            ErrorCode::DatabaseError,
            e.to_string(),
        )),
    }
}

#[tauri::command]
pub async fn update_label_template(
    bridge: State<'_, Arc<ClientBridge>>,
    id: i64,
    data: LabelTemplateUpdate,
) -> Result<ApiResponse<LabelTemplate>, String> {
    match bridge
        .put(&format!("/api/label-templates/{}", id), &data)
        .await
    {
        Ok(template) => Ok(ApiResponse::success(template)),
        Err(e) => Ok(ApiResponse::error_with_code(
            ErrorCode::DatabaseError,
            e.to_string(),
        )),
    }
}

#[tauri::command]
pub async fn delete_label_template(
    bridge: State<'_, Arc<ClientBridge>>,
    id: i64,
) -> Result<ApiResponse<DeleteData>, String> {
    match bridge
        .delete::<bool>(&format!("/api/label-templates/{}", id))
        .await
    {
        Ok(deleted) => Ok(ApiResponse::success(DeleteData { deleted })),
        Err(e) => Ok(ApiResponse::error_with_code(
            ErrorCode::DatabaseError,
            e.to_string(),
        )),
    }
}

// ============ Employees ============

#[tauri::command]
pub async fn list_employees(
    bridge: State<'_, Arc<ClientBridge>>,
) -> Result<ApiResponse<Vec<Employee>>, String> {
    match bridge.get::<Vec<Employee>>("/api/employees").await {
        Ok(employees) => Ok(ApiResponse::success(employees)),
        Err(e) => Ok(ApiResponse::error_with_code(
            ErrorCode::DatabaseError,
            e.to_string(),
        )),
    }
}

#[tauri::command]
pub async fn list_all_employees(
    bridge: State<'_, Arc<ClientBridge>>,
) -> Result<ApiResponse<Vec<Employee>>, String> {
    match bridge.get::<Vec<Employee>>("/api/employees/all").await {
        Ok(employees) => Ok(ApiResponse::success(employees)),
        Err(e) => Ok(ApiResponse::error_with_code(
            ErrorCode::DatabaseError,
            e.to_string(),
        )),
    }
}

#[tauri::command]
pub async fn get_employee(
    bridge: State<'_, Arc<ClientBridge>>,
    id: i64,
) -> Result<ApiResponse<Employee>, String> {
    match bridge.get(&format!("/api/employees/{}", id)).await {
        Ok(employee) => Ok(ApiResponse::success(employee)),
        Err(e) => Ok(ApiResponse::error_with_code(
            ErrorCode::EmployeeNotFound,
            e.to_string(),
        )),
    }
}

#[tauri::command]
pub async fn create_employee(
    bridge: State<'_, Arc<ClientBridge>>,
    data: EmployeeCreate,
) -> Result<ApiResponse<Employee>, String> {
    match bridge.post("/api/employees", &data).await {
        Ok(employee) => Ok(ApiResponse::success(employee)),
        Err(e) => Ok(ApiResponse::error_with_code(
            ErrorCode::DatabaseError,
            e.to_string(),
        )),
    }
}

#[tauri::command]
pub async fn update_employee(
    bridge: State<'_, Arc<ClientBridge>>,
    id: i64,
    data: EmployeeUpdate,
) -> Result<ApiResponse<Employee>, String> {
    match bridge.put(&format!("/api/employees/{}", id), &data).await {
        Ok(employee) => Ok(ApiResponse::success(employee)),
        Err(e) => Ok(ApiResponse::error_with_code(
            ErrorCode::DatabaseError,
            e.to_string(),
        )),
    }
}

#[tauri::command]
pub async fn delete_employee(
    bridge: State<'_, Arc<ClientBridge>>,
    id: i64,
) -> Result<ApiResponse<DeleteData>, String> {
    match bridge
        .delete::<bool>(&format!("/api/employees/{}", id))
        .await
    {
        Ok(deleted) => Ok(ApiResponse::success(DeleteData { deleted })),
        Err(e) => Ok(ApiResponse::error_with_code(
            ErrorCode::DatabaseError,
            e.to_string(),
        )),
    }
}

// ============ Price Rules ============

#[tauri::command]
pub async fn list_price_rules(
    bridge: State<'_, Arc<ClientBridge>>,
) -> Result<ApiResponse<Vec<PriceRule>>, String> {
    match bridge.get::<Vec<PriceRule>>("/api/price-rules").await {
        Ok(rules) => Ok(ApiResponse::success(rules)),
        Err(e) => Ok(ApiResponse::error_with_code(
            ErrorCode::DatabaseError,
            e.to_string(),
        )),
    }
}

#[tauri::command]
pub async fn get_price_rule(
    bridge: State<'_, Arc<ClientBridge>>,
    id: i64,
) -> Result<ApiResponse<PriceRule>, String> {
    match bridge.get(&format!("/api/price-rules/{}", id)).await {
        Ok(rule) => Ok(ApiResponse::success(rule)),
        Err(e) => Ok(ApiResponse::error_with_code(
            ErrorCode::NotFound,
            e.to_string(),
        )),
    }
}

#[tauri::command]
pub async fn create_price_rule(
    bridge: State<'_, Arc<ClientBridge>>,
    data: PriceRuleCreate,
) -> Result<ApiResponse<PriceRule>, String> {
    match bridge.post("/api/price-rules", &data).await {
        Ok(rule) => Ok(ApiResponse::success(rule)),
        Err(e) => Ok(ApiResponse::error_with_code(
            ErrorCode::DatabaseError,
            e.to_string(),
        )),
    }
}

#[tauri::command]
pub async fn update_price_rule(
    bridge: State<'_, Arc<ClientBridge>>,
    id: i64,
    data: PriceRuleUpdate,
) -> Result<ApiResponse<PriceRule>, String> {
    match bridge.put(&format!("/api/price-rules/{}", id), &data).await {
        Ok(rule) => Ok(ApiResponse::success(rule)),
        Err(e) => Ok(ApiResponse::error_with_code(
            ErrorCode::DatabaseError,
            e.to_string(),
        )),
    }
}

#[tauri::command]
pub async fn delete_price_rule(
    bridge: State<'_, Arc<ClientBridge>>,
    id: i64,
) -> Result<ApiResponse<DeleteData>, String> {
    match bridge
        .delete::<bool>(&format!("/api/price-rules/{}", id))
        .await
    {
        Ok(deleted) => Ok(ApiResponse::success(DeleteData { deleted })),
        Err(e) => Ok(ApiResponse::error_with_code(
            ErrorCode::DatabaseError,
            e.to_string(),
        )),
    }
}

// ============ Roles ============

#[tauri::command]
pub async fn list_roles(
    bridge: State<'_, Arc<ClientBridge>>,
) -> Result<ApiResponse<Vec<Role>>, String> {
    match bridge.get::<Vec<Role>>("/api/roles").await {
        Ok(roles) => Ok(ApiResponse::success(roles)),
        Err(e) => Ok(ApiResponse::error_with_code(
            ErrorCode::DatabaseError,
            e.to_string(),
        )),
    }
}

#[tauri::command]
pub async fn get_role(
    bridge: State<'_, Arc<ClientBridge>>,
    id: i64,
) -> Result<ApiResponse<Role>, String> {
    match bridge.get(&format!("/api/roles/{}", id)).await {
        Ok(role) => Ok(ApiResponse::success(role)),
        Err(e) => Ok(ApiResponse::error_with_code(
            ErrorCode::RoleNotFound,
            e.to_string(),
        )),
    }
}

#[tauri::command]
pub async fn create_role(
    bridge: State<'_, Arc<ClientBridge>>,
    data: RoleCreate,
) -> Result<ApiResponse<Role>, String> {
    match bridge.post("/api/roles", &data).await {
        Ok(role) => Ok(ApiResponse::success(role)),
        Err(e) => Ok(ApiResponse::error_with_code(
            ErrorCode::DatabaseError,
            e.to_string(),
        )),
    }
}

#[tauri::command]
pub async fn update_role(
    bridge: State<'_, Arc<ClientBridge>>,
    id: i64,
    data: RoleUpdate,
) -> Result<ApiResponse<Role>, String> {
    match bridge.put(&format!("/api/roles/{}", id), &data).await {
        Ok(role) => Ok(ApiResponse::success(role)),
        Err(e) => Ok(ApiResponse::error_with_code(
            ErrorCode::DatabaseError,
            e.to_string(),
        )),
    }
}

#[tauri::command]
pub async fn delete_role(
    bridge: State<'_, Arc<ClientBridge>>,
    id: i64,
) -> Result<ApiResponse<DeleteData>, String> {
    match bridge.delete::<bool>(&format!("/api/roles/{}", id)).await {
        Ok(deleted) => Ok(ApiResponse::success(DeleteData { deleted })),
        Err(e) => Ok(ApiResponse::error_with_code(
            ErrorCode::DatabaseError,
            e.to_string(),
        )),
    }
}

#[tauri::command]
pub async fn get_role_permissions(
    bridge: State<'_, Arc<ClientBridge>>,
    role_id: i64,
) -> Result<ApiResponse<Vec<RolePermission>>, String> {
    // API 返回 Vec<String>，需要转换为 Vec<RolePermission>
    match bridge
        .get::<Vec<String>>(&format!("/api/roles/{}/permissions", role_id))
        .await
    {
        Ok(permission_strings) => {
            let permissions = permission_strings
                .into_iter()
                .map(|p| RolePermission {
                    role_id,
                    permission: p,
                })
                .collect();
            Ok(ApiResponse::success(permissions))
        }
        Err(e) => Ok(ApiResponse::error_with_code(
            ErrorCode::DatabaseError,
            e.to_string(),
        )),
    }
}

#[tauri::command]
pub async fn get_all_permissions(
    bridge: State<'_, Arc<ClientBridge>>,
) -> Result<ApiResponse<Vec<String>>, String> {
    match bridge.get::<Vec<String>>("/api/permissions").await {
        Ok(permissions) => Ok(ApiResponse::success(permissions)),
        Err(e) => Ok(ApiResponse::error_with_code(
            ErrorCode::DatabaseError,
            e.to_string(),
        )),
    }
}

#[tauri::command]
pub async fn update_role_permissions(
    bridge: State<'_, Arc<ClientBridge>>,
    role_id: i64,
    permissions: Vec<String>,
) -> Result<ApiResponse<()>, String> {
    match bridge
        .put::<(), _>(&format!("/api/roles/{}/permissions", role_id), &permissions)
        .await
    {
        Ok(_) => Ok(ApiResponse::success(())),
        Err(e) => Ok(ApiResponse::error_with_code(
            ErrorCode::DatabaseError,
            e.to_string(),
        )),
    }
}
