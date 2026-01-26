//! Daily Report API Handlers

use axum::{
    Json,
    extract::{Path, Query, State},
};
use serde::Deserialize;

use crate::core::ServerState;
use crate::db::models::{DailyReport, DailyReportGenerate};
use crate::db::repository::DailyReportRepository;
use crate::utils::{AppError, AppResult};

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
    .map_err(|e| AppError::database(e.to_string()))?;

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
        .map_err(|e| AppError::database(e.to_string()))?
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
        .map_err(|e| AppError::database(e.to_string()))?
        .ok_or_else(|| AppError::not_found(format!("Daily report for {} not found", date)))?;
    Ok(Json(report))
}

/// POST /api/daily-reports/generate - 生成日结报告
pub async fn generate(
    State(state): State<ServerState>,
    Json(payload): Json<DailyReportGenerate>,
) -> AppResult<Json<DailyReport>> {
    // TODO: Extract operator info from JWT auth context
    let operator_id = None;
    let operator_name = None;

    let repo = DailyReportRepository::new(state.db.clone());
    let report = repo
        .generate(payload, operator_id, operator_name)
        .await
        .map_err(|e| AppError::database(e.to_string()))?;

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
        .map_err(|e| AppError::database(e.to_string()))?;

    if result {
        state
            .broadcast_sync::<()>(RESOURCE, "deleted", &id, None)
            .await;
    }

    Ok(Json(result))
}
