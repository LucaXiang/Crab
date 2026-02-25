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
    verify_store(&state, store_id, &identity.tenant_id).await?;

    let reports = tenant_queries::list_daily_reports(
        &state.pool,
        store_id,
        &identity.tenant_id,
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
    let overview =
        tenant_queries::get_tenant_overview(&state.pool, &identity.tenant_id, query.from, query.to)
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
    verify_store(&state, store_id, &identity.tenant_id).await?;

    let overview = tenant_queries::get_store_overview(
        &state.pool,
        store_id,
        &identity.tenant_id,
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
    verify_store(&state, store_id, &identity.tenant_id).await?;

    let red_flags = tenant_queries::get_red_flags(
        &state.pool,
        store_id,
        &identity.tenant_id,
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
