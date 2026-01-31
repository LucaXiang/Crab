//! Shift API Handlers

use axum::{
    Json,
    extract::{Extension, Path, Query, State},
};
use chrono::NaiveTime;
use serde::Deserialize;

use crate::audit::AuditAction;
use crate::audit_log;
use crate::auth::CurrentUser;
use crate::core::ServerState;
use crate::db::models::{Shift, ShiftClose, ShiftCreate, ShiftForceClose, ShiftUpdate};
use crate::db::repository::{ShiftRepository, StoreInfoRepository};
use crate::utils::{AppError, AppResult};

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

    let shifts = if let (Some(start), Some(end)) = (query.start_date, query.end_date) {
        repo.find_by_date_range(&start, &end).await
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

/// POST /api/shifts/recover - 恢复跨天的僵尸班次
///
/// 根据 store_info.business_day_cutoff 计算当前营业日起始时间，
/// 自动关闭所有在此之前开启的僵尸班次。
pub async fn recover_stale(
    State(state): State<ServerState>,
) -> AppResult<Json<Vec<Shift>>> {
    // 读取营业日分界时间
    let store_repo = StoreInfoRepository::new(state.db.clone());
    let cutoff = store_repo
        .get()
        .await
        .ok()
        .flatten()
        .map(|s| s.business_day_cutoff)
        .unwrap_or_else(|| "00:00".to_string());

    let cutoff_time = NaiveTime::parse_from_str(&cutoff, "%H:%M")
        .unwrap_or(NaiveTime::MIN);

    // 计算当前营业日起始时间
    // 如果当前时间 < cutoff，说明还在"昨天"的营业日
    let tz = state.config.timezone;
    let now = chrono::Utc::now().with_timezone(&tz);
    let today_business_date = if now.time() < cutoff_time {
        (now - chrono::Duration::days(1)).date_naive()
    } else {
        now.date_naive()
    };
    let business_day_start = today_business_date.and_time(cutoff_time).and_utc().timestamp_millis();

    let repo = ShiftRepository::new(state.db.clone());
    let recovered = repo
        .recover_stale_shifts(business_day_start)
        .await
        ?;

    // 广播每个恢复的班次
    for shift in &recovered {
        let id = shift
            .id
            .as_ref()
            .map(|id| id.to_string())
            .unwrap_or_default();
        state
            .broadcast_sync(RESOURCE, "recovered", &id, Some(shift))
            .await;
    }

    Ok(Json(recovered))
}

/// POST /api/shifts/debug/simulate-auto-close
/// Debug: 强制关闭所有 OPEN 班次并广播，模拟自动关闭调度器行为
/// @TEST 上线前删除
pub async fn debug_simulate_auto_close(
    State(state): State<ServerState>,
) -> AppResult<Json<Vec<Shift>>> {
    // 关闭所有 OPEN 班次（不判断 business_day_start）
    let mut result = state
        .db
        .query(
            r#"
            UPDATE shift SET
                status = 'CLOSED',
                end_time = $now,
                abnormal_close = true,
                note = 'Debug: 模拟自动关闭',
                updated_at = $now
            WHERE status = 'OPEN'
            RETURN AFTER
            "#,
        )
        .bind(("now", shared::util::now_millis()))
        .await
        .map_err(crate::db::repository::surreal_err_to_app)?;

    let closed: Vec<Shift> = result
        .take(0)
        .map_err(crate::db::repository::surreal_err_to_app)?;

    for shift in &closed {
        let id = shift
            .id
            .as_ref()
            .map(|id| id.to_string())
            .unwrap_or_default();
        state
            .broadcast_sync(RESOURCE, "recovered", &id, Some(shift))
            .await;
    }

    tracing::info!("🐛 Debug: simulated auto-close for {} shift(s)", closed.len());

    Ok(Json(closed))
}
