//! Analytics endpoints: stats, overview, red flags

use axum::{
    Extension, Json,
    extract::{Path, Query, State},
};
use serde::Deserialize;
use shared::error::{AppError, ErrorCode};

use crate::auth::tenant_auth::TenantIdentity;
use crate::db::tenant_queries;
use crate::state::AppState;

use super::{ApiResult, verify_store};

/// Maximum query range: 90 days in milliseconds
const MAX_RANGE_MS: i64 = 90 * 24 * 3600 * 1000;

/// Validate that a time range does not exceed 90 days.
fn validate_range(from: i64, to: i64) -> Result<(), AppError> {
    if to <= from {
        return Err(AppError::with_message(
            ErrorCode::ValidationFailed,
            "End date must be after start date",
        ));
    }
    if to - from > MAX_RANGE_MS {
        return Err(AppError::with_message(
            ErrorCode::ValidationFailed,
            "Query range cannot exceed 90 days",
        ));
    }
    Ok(())
}

/// GET /api/tenant/stores/:id/stats?from=YYYY-MM-DD&to=YYYY-MM-DD
#[derive(Deserialize)]
pub struct StatsQuery {
    pub from: Option<String>,
    pub to: Option<String>,
}

pub async fn get_stats(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path(store_id): Path<i64>,
    Query(query): Query<StatsQuery>,
) -> ApiResult<Vec<tenant_queries::DailyReportEntry>> {
    verify_store(&state, store_id, identity.tenant_id).await?;

    let reports = tenant_queries::list_daily_reports(
        &state.pool,
        store_id,
        identity.tenant_id,
        query.from.as_deref(),
        query.to.as_deref(),
    )
    .await
    .map_err(|e| {
        tracing::error!("Stats query error: {e}");
        AppError::new(ErrorCode::InternalError)
    })?;

    Ok(Json(reports))
}

/// GET /api/tenant/overview?from=&to=
#[derive(Deserialize)]
pub struct OverviewQuery {
    pub from: i64,
    pub to: i64,
}

pub async fn get_tenant_overview(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Query(query): Query<OverviewQuery>,
) -> ApiResult<tenant_queries::StoreOverview> {
    validate_range(query.from, query.to)?;

    let overview =
        tenant_queries::get_tenant_overview(&state.pool, identity.tenant_id, query.from, query.to)
            .await
            .map_err(|e| {
                tracing::error!("Tenant overview query error: {e}");
                AppError::new(ErrorCode::InternalError)
            })?;

    Ok(Json(overview))
}

/// GET /api/tenant/stores/:id/overview?from=&to=
pub async fn get_store_overview(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path(store_id): Path<i64>,
    Query(query): Query<OverviewQuery>,
) -> ApiResult<tenant_queries::StoreOverview> {
    validate_range(query.from, query.to)?;
    verify_store(&state, store_id, identity.tenant_id).await?;

    let overview = tenant_queries::get_store_overview(
        &state.pool,
        store_id,
        identity.tenant_id,
        query.from,
        query.to,
    )
    .await
    .map_err(|e| {
        tracing::error!("Overview query error: {e}");
        AppError::new(ErrorCode::InternalError)
    })?;

    Ok(Json(overview))
}

/// GET /api/tenant/stores/:id/red-flags?from=&to=
pub async fn get_store_red_flags(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path(store_id): Path<i64>,
    Query(query): Query<OverviewQuery>,
) -> ApiResult<tenant_queries::RedFlagsResponse> {
    validate_range(query.from, query.to)?;
    verify_store(&state, store_id, identity.tenant_id).await?;

    let red_flags = tenant_queries::get_red_flags(
        &state.pool,
        store_id,
        identity.tenant_id,
        query.from,
        query.to,
    )
    .await
    .map_err(|e| {
        tracing::error!("Red flags query error: {e}");
        AppError::new(ErrorCode::InternalError)
    })?;

    Ok(Json(red_flags))
}

/// GET /api/tenant/stores/:id/red-flags/log?from=&to=&event_type=&operator_id=&page=
pub async fn get_store_red_flag_log(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path(store_id): Path<i64>,
    Query(query): Query<RedFlagLogQuery>,
) -> ApiResult<tenant_queries::RedFlagLogResponse> {
    validate_range(query.from, query.to)?;
    verify_store(&state, store_id, identity.tenant_id).await?;

    let log = tenant_queries::get_red_flag_log(
        &state.pool,
        store_id,
        identity.tenant_id,
        query.from,
        query.to,
        query.event_type.as_deref(),
        query.operator_id,
        query.page.unwrap_or(1),
    )
    .await
    .map_err(|e| {
        tracing::error!("Red flag log query error: {e}");
        AppError::new(ErrorCode::InternalError)
    })?;

    Ok(Json(log))
}

#[derive(Deserialize)]
pub struct RedFlagLogQuery {
    pub from: i64,
    pub to: i64,
    pub event_type: Option<String>,
    pub operator_id: Option<i64>,
    pub page: Option<i32>,
}

/// GET /api/tenant/stores/:id/shifts
pub async fn list_shifts(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path(store_id): Path<i64>,
) -> ApiResult<Vec<tenant_queries::ShiftEntry>> {
    verify_store(&state, store_id, identity.tenant_id).await?;

    let shifts = tenant_queries::list_shifts(&state.pool, store_id, identity.tenant_id)
        .await
        .map_err(|e| {
            tracing::error!("Shifts query error: {e}");
            AppError::new(ErrorCode::InternalError)
        })?;

    Ok(Json(shifts))
}

/// GET /api/tenant/stores/:id/reports/:date
pub async fn get_report_detail(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path((store_id, date)): Path<(i64, String)>,
) -> ApiResult<tenant_queries::DailyReportDetail> {
    // Validate YYYY-MM-DD format
    if date.len() != 10 || !date.bytes().all(|b| b.is_ascii_digit() || b == b'-') {
        return Err(AppError::validation(
            "Invalid date format, expected YYYY-MM-DD",
        ));
    }

    verify_store(&state, store_id, identity.tenant_id).await?;

    let detail =
        tenant_queries::get_daily_report_detail(&state.pool, store_id, identity.tenant_id, &date)
            .await
            .map_err(|e| {
                tracing::error!("Report detail query error: {e}");
                AppError::new(ErrorCode::InternalError)
            })?
            .ok_or_else(|| {
                AppError::with_message(ErrorCode::NotFound, "Daily report not found for this date")
            })?;

    Ok(Json(detail))
}
