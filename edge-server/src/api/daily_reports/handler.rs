//! Daily Report API Handlers

use axum::{
    Json,
    extract::{Extension, Path, Query, State},
};
use serde::Deserialize;

use crate::audit::AuditAction;
use crate::audit_log;
use crate::auth::CurrentUser;
use crate::core::ServerState;
use crate::db::repository::daily_report;
use crate::utils::time;
use crate::utils::validation::{
    MAX_NOTE_LEN, MAX_SHORT_TEXT_LEN, validate_optional_text, validate_required_text,
};
use crate::utils::{AppError, AppResult};
use shared::error::ErrorCode;
use shared::message::SyncChangeType;
use shared::models::{DailyReport, DailyReportGenerate};

use shared::cloud::SyncResource;
const RESOURCE: SyncResource = SyncResource::DailyReport;

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
    let reports = if let (Some(start), Some(end)) = (query.start_date, query.end_date) {
        daily_report::find_by_date_range(&state.pool, &start, &end).await
    } else {
        daily_report::find_all(&state.pool, query.limit, query.offset).await
    }?;

    Ok(Json(reports))
}

/// GET /api/daily-reports/:id - 获取单个日结报告
pub async fn get_by_id(
    State(state): State<ServerState>,
    Path(id): Path<i64>,
) -> AppResult<Json<DailyReport>> {
    let report = daily_report::find_by_id(&state.pool, id)
        .await?
        .ok_or_else(|| {
            AppError::with_message(
                ErrorCode::DailyReportNotFound,
                format!("Daily report {} not found", id),
            )
        })?;
    Ok(Json(report))
}

/// GET /api/daily-reports/date/:date - 按日期获取日结报告
pub async fn get_by_date(
    State(state): State<ServerState>,
    Path(date): Path<String>,
) -> AppResult<Json<DailyReport>> {
    let report = daily_report::find_by_date(&state.pool, &date)
        .await?
        .ok_or_else(|| {
            AppError::with_message(
                ErrorCode::DailyReportNotFound,
                format!("Daily report for {} not found", date),
            )
        })?;
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

    validate_required_text(&payload.business_date, "business_date", MAX_SHORT_TEXT_LEN)?;
    validate_optional_text(&payload.note, "note", MAX_NOTE_LEN)?;

    // 日期验证 + 时区转换 (handler 层职责)
    let tz = state.config.timezone;
    let business_date = payload.business_date.clone();
    let date = time::parse_date(&business_date)?;
    time::validate_not_future(date, tz)?;
    let start_millis = time::day_start_millis(date, tz);
    let next_day = date.succ_opt().unwrap_or(date);
    let end_millis = time::day_start_millis(next_day, tz);

    let audit_operator_id = current_user.id;
    let audit_operator_name = current_user.display_name.clone();

    let operator_id = Some(current_user.id);
    let operator_name = Some(current_user.display_name);

    let report = daily_report::generate(
        &state.pool,
        payload,
        start_millis,
        end_millis,
        operator_id,
        operator_name,
    )
    .await?;

    let id = report.id.to_string();

    audit_log!(
        state.audit_service,
        AuditAction::DailyReportGenerated,
        "daily_report",
        &id,
        operator_id = Some(audit_operator_id),
        operator_name = Some(audit_operator_name),
        details = serde_json::json!({
            "business_date": &business_date,
        })
    );

    state
        .broadcast_sync(RESOURCE, SyncChangeType::Created, &id, Some(&report), false)
        .await;

    Ok(Json(report))
}
