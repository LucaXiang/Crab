//! Shift API Commands (班次管理)
//!
//! 通过 ClientBridge -> CrabClient -> EdgeServer REST API

use std::sync::Arc;
use tauri::State;
use tokio::sync::RwLock;
use urlencoding::encode;

use crate::core::{ApiResponse, ClientBridge};
use shared::models::{
    DailyReport, DailyReportGenerate, Shift, ShiftClose, ShiftCreate, ShiftForceClose, ShiftUpdate,
};

// ============ Shifts ============

#[tauri::command]
pub async fn list_shifts(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    limit: Option<i32>,
    offset: Option<i32>,
    start_date: Option<String>,
    end_date: Option<String>,
) -> Result<ApiResponse<Vec<Shift>>, String> {
    let bridge = bridge.read().await;

    let mut query_params = vec![];
    if let Some(l) = limit {
        query_params.push(format!("limit={}", l));
    }
    if let Some(o) = offset {
        query_params.push(format!("offset={}", o));
    }
    if let Some(s) = start_date {
        query_params.push(format!("start_date={}", encode(&s)));
    }
    if let Some(e) = end_date {
        query_params.push(format!("end_date={}", encode(&e)));
    }

    let path = if query_params.is_empty() {
        "/api/shifts".to_string()
    } else {
        format!("/api/shifts?{}", query_params.join("&"))
    };

    match bridge.get::<Vec<Shift>>(&path).await {
        Ok(shifts) => Ok(ApiResponse::success(shifts)),
        Err(e) => Ok(ApiResponse::from_bridge_error(e)),
    }
}

#[tauri::command]
pub async fn get_shift(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    id: String,
) -> Result<ApiResponse<Shift>, String> {
    let bridge = bridge.read().await;
    match bridge
        .get::<Shift>(&format!("/api/shifts/{}", encode(&id)))
        .await
    {
        Ok(shift) => Ok(ApiResponse::success(shift)),
        Err(e) => Ok(ApiResponse::from_bridge_error(e)),
    }
}

#[tauri::command]
pub async fn get_current_shift(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    operator_id: Option<String>,
) -> Result<ApiResponse<Option<Shift>>, String> {
    let bridge = bridge.read().await;

    let path = match operator_id {
        Some(id) => format!("/api/shifts/current?operator_id={}", encode(&id)),
        None => "/api/shifts/current".to_string(),
    };

    match bridge.get::<Option<Shift>>(&path).await {
        Ok(shift) => Ok(ApiResponse::success(shift)),
        Err(e) => Ok(ApiResponse::from_bridge_error(e)),
    }
}

#[tauri::command]
pub async fn open_shift(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    data: ShiftCreate,
) -> Result<ApiResponse<Shift>, String> {
    let bridge = bridge.read().await;
    match bridge.post::<Shift, _>("/api/shifts", &data).await {
        Ok(shift) => Ok(ApiResponse::success(shift)),
        Err(e) => Ok(ApiResponse::from_bridge_error(e)),
    }
}

#[tauri::command]
pub async fn update_shift(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    id: String,
    data: ShiftUpdate,
) -> Result<ApiResponse<Shift>, String> {
    let bridge = bridge.read().await;
    match bridge
        .put::<Shift, _>(&format!("/api/shifts/{}", encode(&id)), &data)
        .await
    {
        Ok(shift) => Ok(ApiResponse::success(shift)),
        Err(e) => Ok(ApiResponse::from_bridge_error(e)),
    }
}

#[tauri::command]
pub async fn close_shift(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    id: String,
    data: ShiftClose,
) -> Result<ApiResponse<Shift>, String> {
    let bridge = bridge.read().await;
    match bridge
        .post::<Shift, _>(&format!("/api/shifts/{}/close", encode(&id)), &data)
        .await
    {
        Ok(shift) => Ok(ApiResponse::success(shift)),
        Err(e) => Ok(ApiResponse::from_bridge_error(e)),
    }
}

#[tauri::command]
pub async fn force_close_shift(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    id: String,
    data: ShiftForceClose,
) -> Result<ApiResponse<Shift>, String> {
    let bridge = bridge.read().await;
    match bridge
        .post::<Shift, _>(&format!("/api/shifts/{}/force-close", encode(&id)), &data)
        .await
    {
        Ok(shift) => Ok(ApiResponse::success(shift)),
        Err(e) => Ok(ApiResponse::from_bridge_error(e)),
    }
}

#[tauri::command]
pub async fn heartbeat_shift(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    id: String,
) -> Result<ApiResponse<bool>, String> {
    let bridge = bridge.read().await;
    match bridge
        .post::<bool, _>(&format!("/api/shifts/{}/heartbeat", encode(&id)), &())
        .await
    {
        Ok(result) => Ok(ApiResponse::success(result)),
        Err(e) => Ok(ApiResponse::from_bridge_error(e)),
    }
}

#[tauri::command]
pub async fn recover_stale_shifts(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
) -> Result<ApiResponse<Vec<Shift>>, String> {
    let bridge = bridge.read().await;
    match bridge.post::<Vec<Shift>, _>("/api/shifts/recover", &()).await {
        Ok(shifts) => Ok(ApiResponse::success(shifts)),
        Err(e) => Ok(ApiResponse::from_bridge_error(e)),
    }
}

/// @TEST 上线前删除
#[tauri::command]
pub async fn debug_simulate_shift_auto_close(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
) -> Result<ApiResponse<Vec<Shift>>, String> {
    let bridge = bridge.read().await;
    match bridge
        .post::<Vec<Shift>, _>("/api/shifts/debug/simulate-auto-close", &())
        .await
    {
        Ok(shifts) => Ok(ApiResponse::success(shifts)),
        Err(e) => Ok(ApiResponse::from_bridge_error(e)),
    }
}

// ============ Daily Reports ============

#[tauri::command]
pub async fn list_daily_reports(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    limit: Option<i32>,
    offset: Option<i32>,
    start_date: Option<String>,
    end_date: Option<String>,
) -> Result<ApiResponse<Vec<DailyReport>>, String> {
    let bridge = bridge.read().await;

    let mut query_params = vec![];
    if let Some(l) = limit {
        query_params.push(format!("limit={}", l));
    }
    if let Some(o) = offset {
        query_params.push(format!("offset={}", o));
    }
    if let Some(s) = start_date {
        query_params.push(format!("start_date={}", encode(&s)));
    }
    if let Some(e) = end_date {
        query_params.push(format!("end_date={}", encode(&e)));
    }

    let path = if query_params.is_empty() {
        "/api/daily-reports".to_string()
    } else {
        format!("/api/daily-reports?{}", query_params.join("&"))
    };

    match bridge.get::<Vec<DailyReport>>(&path).await {
        Ok(reports) => Ok(ApiResponse::success(reports)),
        Err(e) => Ok(ApiResponse::from_bridge_error(e)),
    }
}

#[tauri::command]
pub async fn get_daily_report(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    id: String,
) -> Result<ApiResponse<DailyReport>, String> {
    let bridge = bridge.read().await;
    match bridge
        .get::<DailyReport>(&format!("/api/daily-reports/{}", encode(&id)))
        .await
    {
        Ok(report) => Ok(ApiResponse::success(report)),
        Err(e) => Ok(ApiResponse::from_bridge_error(e)),
    }
}

#[tauri::command]
pub async fn get_daily_report_by_date(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    date: String,
) -> Result<ApiResponse<DailyReport>, String> {
    let bridge = bridge.read().await;
    match bridge
        .get::<DailyReport>(&format!("/api/daily-reports/date/{}", encode(&date)))
        .await
    {
        Ok(report) => Ok(ApiResponse::success(report)),
        Err(e) => Ok(ApiResponse::from_bridge_error(e)),
    }
}

#[tauri::command]
pub async fn generate_daily_report(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    data: DailyReportGenerate,
) -> Result<ApiResponse<DailyReport>, String> {
    let bridge = bridge.read().await;
    match bridge
        .post::<DailyReport, _>("/api/daily-reports/generate", &data)
        .await
    {
        Ok(report) => Ok(ApiResponse::success(report)),
        Err(e) => Ok(ApiResponse::from_bridge_error(e)),
    }
}

#[tauri::command]
pub async fn delete_daily_report(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    id: String,
) -> Result<ApiResponse<bool>, String> {
    let bridge = bridge.read().await;
    match bridge
        .delete::<bool>(&format!("/api/daily-reports/{}", encode(&id)))
        .await
    {
        Ok(result) => Ok(ApiResponse::success(result)),
        Err(e) => Ok(ApiResponse::from_bridge_error(e)),
    }
}
