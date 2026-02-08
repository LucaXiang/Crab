//! 位置 API Commands (Zones, Tables)
//!
//! 通过 ClientBridge -> CrabClient -> EdgeServer REST API

use std::sync::Arc;
use tauri::State;

use crate::core::response::{ApiResponse, ErrorCode, TableListData, ZoneListData};
use crate::core::ClientBridge;
use shared::models::{
    DiningTable, DiningTableCreate, DiningTableUpdate, Zone, ZoneCreate, ZoneUpdate,
};

// ============ Zones ============

#[tauri::command]
pub async fn list_zones(
    bridge: State<'_, Arc<ClientBridge>>,
) -> Result<ApiResponse<ZoneListData>, String> {
    match bridge.get::<Vec<Zone>>("/api/zones").await {
        Ok(zones) => Ok(ApiResponse::success(ZoneListData { zones })),
        Err(e) => Ok(ApiResponse::error_with_code(
            ErrorCode::DatabaseError,
            e.to_string(),
        )),
    }
}

#[tauri::command]
pub async fn get_zone(
    bridge: State<'_, Arc<ClientBridge>>,
    id: i64,
) -> Result<ApiResponse<Zone>, String> {
    match bridge
        .get::<Zone>(&format!("/api/zones/{}", id))
        .await
    {
        Ok(zone) => Ok(ApiResponse::success(zone)),
        Err(e) => Ok(ApiResponse::error_with_code(
            ErrorCode::ZoneNotFound,
            e.to_string(),
        )),
    }
}

#[tauri::command]
pub async fn create_zone(
    bridge: State<'_, Arc<ClientBridge>>,
    data: ZoneCreate,
) -> Result<ApiResponse<Zone>, String> {
    match bridge.post::<Zone, _>("/api/zones", &data).await {
        Ok(zone) => Ok(ApiResponse::success(zone)),
        Err(e) => Ok(ApiResponse::error_with_code(
            ErrorCode::DatabaseError,
            e.to_string(),
        )),
    }
}

#[tauri::command]
pub async fn update_zone(
    bridge: State<'_, Arc<ClientBridge>>,
    id: i64,
    data: ZoneUpdate,
) -> Result<ApiResponse<Zone>, String> {
    match bridge
        .put::<Zone, _>(&format!("/api/zones/{}", id), &data)
        .await
    {
        Ok(zone) => Ok(ApiResponse::success(zone)),
        Err(e) => Ok(ApiResponse::error_with_code(
            ErrorCode::DatabaseError,
            e.to_string(),
        )),
    }
}

#[tauri::command]
pub async fn delete_zone(
    bridge: State<'_, Arc<ClientBridge>>,
    id: i64,
) -> Result<ApiResponse<crate::core::DeleteData>, String> {
    match bridge
        .delete::<bool>(&format!("/api/zones/{}", id))
        .await
    {
        Ok(success) => Ok(ApiResponse::success(crate::core::DeleteData {
            deleted: success,
        })),
        Err(e) => Ok(ApiResponse::error_with_code(
            ErrorCode::DatabaseError,
            e.to_string(),
        )),
    }
}

// ============ Dining Tables ============

#[tauri::command]
pub async fn list_tables(
    bridge: State<'_, Arc<ClientBridge>>,
) -> Result<ApiResponse<TableListData>, String> {
    match bridge.get::<Vec<DiningTable>>("/api/tables").await {
        Ok(tables) => Ok(ApiResponse::success(TableListData { tables })),
        Err(e) => Ok(ApiResponse::error_with_code(
            ErrorCode::DatabaseError,
            e.to_string(),
        )),
    }
}

#[tauri::command]
pub async fn list_tables_by_zone(
    bridge: State<'_, Arc<ClientBridge>>,
    zone_id: i64,
) -> Result<ApiResponse<TableListData>, String> {
    match bridge
        .get::<Vec<DiningTable>>(&format!("/api/tables/zone/{}", zone_id))
        .await
    {
        Ok(tables) => Ok(ApiResponse::success(TableListData { tables })),
        Err(e) => Ok(ApiResponse::error_with_code(
            ErrorCode::DatabaseError,
            e.to_string(),
        )),
    }
}

#[tauri::command]
pub async fn get_table(
    bridge: State<'_, Arc<ClientBridge>>,
    id: i64,
) -> Result<ApiResponse<DiningTable>, String> {
    match bridge
        .get::<DiningTable>(&format!("/api/tables/{}", id))
        .await
    {
        Ok(table) => Ok(ApiResponse::success(table)),
        Err(e) => Ok(ApiResponse::error_with_code(
            ErrorCode::TableNotFound,
            e.to_string(),
        )),
    }
}

#[tauri::command]
pub async fn create_table(
    bridge: State<'_, Arc<ClientBridge>>,
    data: DiningTableCreate,
) -> Result<ApiResponse<DiningTable>, String> {
    match bridge.post::<DiningTable, _>("/api/tables", &data).await {
        Ok(table) => Ok(ApiResponse::success(table)),
        Err(e) => Ok(ApiResponse::error_with_code(
            ErrorCode::DatabaseError,
            e.to_string(),
        )),
    }
}

#[tauri::command]
pub async fn update_table(
    bridge: State<'_, Arc<ClientBridge>>,
    id: i64,
    data: DiningTableUpdate,
) -> Result<ApiResponse<DiningTable>, String> {
    match bridge
        .put::<DiningTable, _>(&format!("/api/tables/{}", id), &data)
        .await
    {
        Ok(table) => Ok(ApiResponse::success(table)),
        Err(e) => Ok(ApiResponse::error_with_code(
            ErrorCode::DatabaseError,
            e.to_string(),
        )),
    }
}

#[tauri::command]
pub async fn delete_table(
    bridge: State<'_, Arc<ClientBridge>>,
    id: i64,
) -> Result<ApiResponse<crate::core::DeleteData>, String> {
    match bridge
        .delete::<bool>(&format!("/api/tables/{}", id))
        .await
    {
        Ok(success) => Ok(ApiResponse::success(crate::core::DeleteData {
            deleted: success,
        })),
        Err(e) => Ok(ApiResponse::error_with_code(
            ErrorCode::DatabaseError,
            e.to_string(),
        )),
    }
}
