//! Shift API Handlers

use axum::{
    Json,
    extract::{Extension, Path, Query, State},
};
use serde::Deserialize;

use crate::audit::AuditAction;
use crate::audit_log;
use crate::auth::CurrentUser;
use crate::core::ServerState;
use crate::db::models::{Shift, ShiftClose, ShiftCreate, ShiftForceClose, ShiftUpdate};
use crate::db::repository::{ShiftRepository, StoreInfoRepository};
use crate::utils::{AppError, AppResult};
use crate::utils::time;

const RESOURCE: &str = "shift";

/// Query params for listing shifts
#[derive(Debug, Deserialize)]
pub struct ListQuery {
    #[serde(default = "default_limit")]
    pub limit: i32,
    #[serde(default)]
    pub offset: i32,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
}

fn default_limit() -> i32 {
    50
}

/// GET /api/shifts - 获取班次列表
pub async fn list(
    State(state): State<ServerState>,
    Query(query): Query<ListQuery>,
) -> AppResult<Json<Vec<Shift>>> {
    let repo = ShiftRepository::new(state.db.clone());

    let tz = state.config.timezone;
    let shifts = if let (Some(start), Some(end)) = (query.start_date, query.end_date) {
        let start_date = time::parse_date(&start)?;
        let end_date = time::parse_date(&end)?;
        repo.find_by_date_range(time::day_start_millis(start_date, tz), time::day_end_millis(end_date, tz)).await
    } else {
        repo.find_all(query.limit, query.offset).await
    }
    ?;

    Ok(Json(shifts))
}

/// GET /api/shifts/:id - 获取单个班次
pub async fn get_by_id(
    State(state): State<ServerState>,
    Path(id): Path<String>,
) -> AppResult<Json<Shift>> {
    let repo = ShiftRepository::new(state.db.clone());
    let shift = repo
        .find_by_id(&id)
        .await
        ?
        .ok_or_else(|| AppError::not_found(format!("Shift {} not found", id)))?;
    Ok(Json(shift))
}

/// Query params for current shift
#[derive(Debug, Deserialize)]
pub struct CurrentQuery {
    pub operator_id: Option<String>,
}

/// GET /api/shifts/current - 获取当前班次
pub async fn get_current(
    State(state): State<ServerState>,
    Query(query): Query<CurrentQuery>,
) -> AppResult<Json<Option<Shift>>> {
    let repo = ShiftRepository::new(state.db.clone());

    let shift = if let Some(operator_id) = query.operator_id {
        repo.find_open_by_operator(&operator_id).await
    } else {
        repo.find_any_open().await
    }
    ?;

    Ok(Json(shift))
}

/// POST /api/shifts - 开班
pub async fn create(
    State(state): State<ServerState>,
    Extension(current_user): Extension<CurrentUser>,
    Json(payload): Json<ShiftCreate>,
) -> AppResult<Json<Shift>> {
    let repo = ShiftRepository::new(state.db.clone());
    let shift = repo
        .create(payload)
        .await
        ?;

    let id = shift
        .id
        .as_ref()
        .map(|id| id.to_string())
        .unwrap_or_default();

    audit_log!(
        state.audit_service,
        AuditAction::ShiftOpened,
        "shift", &id,
        operator_id = Some(current_user.id.clone()),
        operator_name = Some(current_user.display_name.clone()),
        details = serde_json::json!({"starting_cash": shift.starting_cash})
    );

    state
        .broadcast_sync(RESOURCE, "created", &id, Some(&shift))
        .await;

    Ok(Json(shift))
}

/// PUT /api/shifts/:id - 更新班次
pub async fn update(
    State(state): State<ServerState>,
    Path(id): Path<String>,
    Json(payload): Json<ShiftUpdate>,
) -> AppResult<Json<Shift>> {
    let repo = ShiftRepository::new(state.db.clone());
    let shift = repo
        .update(&id, payload)
        .await
        ?;

    state
        .broadcast_sync(RESOURCE, "updated", &id, Some(&shift))
        .await;

    Ok(Json(shift))
}

/// POST /api/shifts/:id/close - 收班
pub async fn close(
    State(state): State<ServerState>,
    Extension(current_user): Extension<CurrentUser>,
    Path(id): Path<String>,
    Json(payload): Json<ShiftClose>,
) -> AppResult<Json<Shift>> {
    let repo = ShiftRepository::new(state.db.clone());
    let shift = repo
        .close(&id, payload)
        .await
        ?;

    audit_log!(
        state.audit_service,
        AuditAction::ShiftClosed,
        "shift", &id,
        operator_id = Some(current_user.id.clone()),
        operator_name = Some(current_user.display_name.clone()),
        details = serde_json::json!({
            "starting_cash": shift.starting_cash,
            "expected_cash": shift.expected_cash,
            "actual_cash": shift.actual_cash,
            "cash_variance": shift.cash_variance,
        })
    );

    state
        .broadcast_sync(RESOURCE, "closed", &id, Some(&shift))
        .await;

    Ok(Json(shift))
}

/// POST /api/shifts/:id/force-close - 强制关闭
pub async fn force_close(
    State(state): State<ServerState>,
    Extension(current_user): Extension<CurrentUser>,
    Path(id): Path<String>,
    Json(payload): Json<ShiftForceClose>,
) -> AppResult<Json<Shift>> {
    let repo = ShiftRepository::new(state.db.clone());
    let shift = repo
        .force_close(&id, payload)
        .await
        ?;

    audit_log!(
        state.audit_service,
        AuditAction::ShiftClosed,
        "shift", &id,
        operator_id = Some(current_user.id.clone()),
        operator_name = Some(current_user.display_name.clone()),
        details = serde_json::json!({
            "forced": true,
            "starting_cash": shift.starting_cash,
            "expected_cash": shift.expected_cash,
        })
    );

    state
        .broadcast_sync(RESOURCE, "force_closed", &id, Some(&shift))
        .await;

    Ok(Json(shift))
}

/// POST /api/shifts/:id/heartbeat - 心跳更新
pub async fn heartbeat(
    State(state): State<ServerState>,
    Path(id): Path<String>,
) -> AppResult<Json<bool>> {
    let repo = ShiftRepository::new(state.db.clone());
    repo.heartbeat(&id)
        .await
        ?;
    Ok(Json(true))
}

/// POST /api/shifts/recover - 检测并通知跨天的过期班次
///
/// 根据 store_info.business_day_cutoff 计算当前营业日起始时间，
/// 查询过期班次并广播 settlement_required 通知前端处理。
pub async fn recover_stale(
    State(state): State<ServerState>,
) -> AppResult<Json<Vec<Shift>>> {
    let tz = state.config.timezone;
    let store_repo = StoreInfoRepository::new(state.db.clone());
    let cutoff_str = store_repo
        .get()
        .await
        .ok()
        .flatten()
        .map(|s| s.business_day_cutoff)
        .unwrap_or_else(|| "02:00".to_string());

    let cutoff = time::parse_cutoff(&cutoff_str);
    let today = time::current_business_date(cutoff, tz);
    let business_day_start = time::date_cutoff_millis(today, cutoff, tz);

    let repo = ShiftRepository::new(state.db.clone());
    let stale = repo
        .find_stale_shifts(business_day_start)
        .await
        ?;

    for shift in &stale {
        let id = shift
            .id
            .as_ref()
            .map(|id| id.to_string())
            .unwrap_or_default();

        state
            .broadcast_sync(RESOURCE, "settlement_required", &id, Some(shift))
            .await;
    }

    Ok(Json(stale))
}

