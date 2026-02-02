//! Daily Report API Handlers

use axum::{
    Json,
    extract::{Extension, Path, Query, State},
};
use serde::Deserialize;

use crate::auth::CurrentUser;
use crate::core::ServerState;
use crate::db::models::{DailyReport, DailyReportGenerate};
use crate::db::repository::DailyReportRepository;
use crate::utils::{AppError, AppResult};
use crate::utils::time;

const RESOURCE: &str = "daily_report";

/// Query params for listing reports
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

/// GET /api/daily-reports - 获取日结报告列表
pub async fn list(
    State(state): State<ServerState>,
    Query(query): Query<ListQuery>,
) -> AppResult<Json<Vec<DailyReport>>> {
    let repo = DailyReportRepository::new(state.db.clone());

    let reports = if let (Some(start), Some(end)) = (query.start_date, query.end_date) {
        repo.find_by_date_range(&start, &end).await
    } else {
        repo.find_all(query.limit, query.offset).await
    }
    ?;

    Ok(Json(reports))
}

/// GET /api/daily-reports/:id - 获取单个日结报告
pub async fn get_by_id(
    State(state): State<ServerState>,
    Path(id): Path<String>,
) -> AppResult<Json<DailyReport>> {
    let repo = DailyReportRepository::new(state.db.clone());
    let report = repo
        .find_by_id(&id)
        .await
        ?
        .ok_or_else(|| AppError::not_found(format!("Daily report {} not found", id)))?;
    Ok(Json(report))
}

/// GET /api/daily-reports/date/:date - 按日期获取日结报告
pub async fn get_by_date(
    State(state): State<ServerState>,
    Path(date): Path<String>,
) -> AppResult<Json<DailyReport>> {
    let repo = DailyReportRepository::new(state.db.clone());
    let report = repo
        .find_by_date(&date)
        .await
        ?
        .ok_or_else(|| AppError::not_found(format!("Daily report for {} not found", date)))?;
    Ok(Json(report))
}

/// POST /api/daily-reports/generate - 生成日结报告
pub async fn generate(
    State(state): State<ServerState>,
    Extension(current_user): Extension<CurrentUser>,
    Json(payload): Json<DailyReportGenerate>,
) -> AppResult<Json<DailyReport>> {
    tracing::info!(
        user_id = %current_user.id,
        username = %current_user.username,
        "Generating daily report"
    );

    // 日期验证 + 时区转换 (handler 层职责)
    let tz = state.config.timezone;
    let date = time::parse_date(&payload.business_date)?;
    time::validate_not_future(date, tz)?;
    let start_millis = time::day_start_millis(date, tz);
    let next_day = date.succ_opt().unwrap_or(date);
    let end_millis = time::day_start_millis(next_day, tz);

    let operator_id = Some(current_user.id);
    let operator_name = Some(current_user.display_name);

    let repo = DailyReportRepository::new(state.db.clone());
    let report = repo
        .generate(payload, start_millis, end_millis, operator_id, operator_name)
        .await
        ?;

    let id = report
        .id
        .as_ref()
        .map(|id| id.to_string())
        .unwrap_or_default();
    state
        .broadcast_sync(RESOURCE, "generated", &id, Some(&report))
        .await;

    Ok(Json(report))
}

/// DELETE /api/daily-reports/:id - 删除日结报告 (管理员)
pub async fn delete(
    State(state): State<ServerState>,
    Path(id): Path<String>,
) -> AppResult<Json<bool>> {
    let repo = DailyReportRepository::new(state.db.clone());
    let result = repo
        .delete(&id)
        .await
        ?;

    if result {
        state
            .broadcast_sync::<()>(RESOURCE, "deleted", &id, None)
            .await;
    }

    Ok(Json(result))
}
