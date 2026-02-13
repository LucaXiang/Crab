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
use crate::db::repository::{shift, store_info};
use crate::utils::{AppError, AppResult};
use crate::utils::time;
use crate::utils::validation::{validate_required_text, validate_optional_text, MAX_NAME_LEN, MAX_NOTE_LEN};
use shared::models::{Shift, ShiftClose, ShiftCreate, ShiftForceClose, ShiftUpdate};

const RESOURCE: &str = "shift";

/// Validate a cash amount is finite and non-negative
fn validate_cash(value: f64, field: &str) -> AppResult<()> {
    if !value.is_finite() {
        return Err(AppError::validation(format!("{field} must be a finite number")));
    }
    if value < 0.0 {
        return Err(AppError::validation(format!("{field} must be non-negative, got {value}")));
    }
    Ok(())
}

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
    let tz = state.config.timezone;
    let shifts = if let (Some(start), Some(end)) = (query.start_date, query.end_date) {
        let start_date = time::parse_date(&start)?;
        let end_date = time::parse_date(&end)?;
        shift::find_by_date_range(
            &state.pool,
            time::day_start_millis(start_date, tz),
            time::day_end_millis(end_date, tz),
        )
        .await
    } else {
        shift::find_all(&state.pool, query.limit, query.offset).await
    }?;

    Ok(Json(shifts))
}

/// GET /api/shifts/:id - 获取单个班次
pub async fn get_by_id(
    State(state): State<ServerState>,
    Path(id): Path<i64>,
) -> AppResult<Json<Shift>> {
    let shift = shift::find_by_id(&state.pool, id)
        .await?
        .ok_or_else(|| AppError::not_found(format!("Shift {} not found", id)))?;
    Ok(Json(shift))
}

/// GET /api/shifts/current - 获取当前班次 (全局单班次)
pub async fn get_current(
    State(state): State<ServerState>,
) -> AppResult<Json<Option<Shift>>> {
    let current = shift::find_any_open(&state.pool).await?;
    Ok(Json(current))
}

/// POST /api/shifts - 开班
pub async fn create(
    State(state): State<ServerState>,
    Extension(current_user): Extension<CurrentUser>,
    Json(payload): Json<ShiftCreate>,
) -> AppResult<Json<Shift>> {
    validate_cash(payload.starting_cash, "starting_cash")?;
    validate_required_text(&payload.operator_name, "operator_name", MAX_NAME_LEN)?;
    validate_optional_text(&payload.note, "note", MAX_NOTE_LEN)?;

    let s = shift::create(&state.pool, payload).await?;

    let id = s.id.to_string();

    audit_log!(
        state.audit_service,
        AuditAction::ShiftOpened,
        "shift", &id,
        operator_id = Some(current_user.id),
        operator_name = Some(current_user.display_name.clone()),
        details = serde_json::json!({
            "starting_cash": s.starting_cash,
            "opened_at": s.start_time,
        })
    );

    state
        .broadcast_sync(RESOURCE, "created", &id, Some(&s))
        .await;

    Ok(Json(s))
}

/// PUT /api/shifts/:id - 更新班次
pub async fn update(
    State(state): State<ServerState>,
    Extension(current_user): Extension<CurrentUser>,
    Path(id): Path<i64>,
    Json(payload): Json<ShiftUpdate>,
) -> AppResult<Json<Shift>> {
    if let Some(cash) = payload.starting_cash {
        validate_cash(cash, "starting_cash")?;
    }
    validate_optional_text(&payload.note, "note", MAX_NOTE_LEN)?;

    let old = shift::find_by_id(&state.pool, id)
        .await?
        .ok_or_else(|| AppError::not_found(format!("Shift {} not found", id)))?;

    let s = shift::update(&state.pool, id, payload).await?;

    let id_str = id.to_string();

    audit_log!(
        state.audit_service,
        AuditAction::ShiftUpdated,
        "shift", &id_str,
        operator_id = Some(current_user.id),
        operator_name = Some(current_user.display_name.clone()),
        details = crate::audit::create_diff(&old, &s, "shift")
    );

    state
        .broadcast_sync(RESOURCE, "updated", &id_str, Some(&s))
        .await;

    Ok(Json(s))
}

/// POST /api/shifts/:id/close - 收班
pub async fn close(
    State(state): State<ServerState>,
    Extension(current_user): Extension<CurrentUser>,
    Path(id): Path<i64>,
    Json(payload): Json<ShiftClose>,
) -> AppResult<Json<Shift>> {
    validate_cash(payload.actual_cash, "actual_cash")?;
    validate_optional_text(&payload.note, "note", MAX_NOTE_LEN)?;

    let s = shift::close(&state.pool, id, payload).await?;

    let id_str = id.to_string();

    audit_log!(
        state.audit_service,
        AuditAction::ShiftClosed,
        "shift", &id_str,
        operator_id = Some(current_user.id),
        operator_name = Some(current_user.display_name.clone()),
        details = serde_json::json!({
            "starting_cash": s.starting_cash,
            "expected_cash": s.expected_cash,
            "actual_cash": s.actual_cash,
            "cash_variance": s.cash_variance,
            "closed_at": s.end_time,
        })
    );

    state
        .broadcast_sync(RESOURCE, "closed", &id_str, Some(&s))
        .await;

    Ok(Json(s))
}

/// POST /api/shifts/:id/force-close - 强制关闭
pub async fn force_close(
    State(state): State<ServerState>,
    Extension(current_user): Extension<CurrentUser>,
    Path(id): Path<i64>,
    Json(payload): Json<ShiftForceClose>,
) -> AppResult<Json<Shift>> {
    validate_optional_text(&payload.note, "note", MAX_NOTE_LEN)?;

    let s = shift::force_close(&state.pool, id, payload).await?;

    let id_str = id.to_string();

    audit_log!(
        state.audit_service,
        AuditAction::ShiftClosed,
        "shift", &id_str,
        operator_id = Some(current_user.id),
        operator_name = Some(current_user.display_name.clone()),
        details = serde_json::json!({
            "forced": true,
            "starting_cash": s.starting_cash,
            "expected_cash": s.expected_cash,
            "closed_at": s.end_time,
        })
    );

    state
        .broadcast_sync(RESOURCE, "force_closed", &id_str, Some(&s))
        .await;

    Ok(Json(s))
}

/// POST /api/shifts/:id/heartbeat - 心跳更新
pub async fn heartbeat(
    State(state): State<ServerState>,
    Path(id): Path<i64>,
) -> AppResult<Json<bool>> {
    shift::heartbeat(&state.pool, id).await?;
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
    let cutoff_str = store_info::get(&state.pool)
        .await
        .ok()
        .flatten()
        .map(|s| s.business_day_cutoff)
        .unwrap_or_else(|| "02:00".to_string());

    let cutoff = time::parse_cutoff(&cutoff_str);
    let today = time::current_business_date(cutoff, tz);
    let business_day_start = time::date_cutoff_millis(today, cutoff, tz);

    let stale = shift::find_stale_shifts(&state.pool, business_day_start).await?;

    for s in &stale {
        let id = s.id.to_string();

        state
            .broadcast_sync(RESOURCE, "settlement_required", &id, Some(s))
            .await;
    }

    Ok(Json(stale))
}
