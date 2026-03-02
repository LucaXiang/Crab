//! Order endpoints: list orders, order detail (with edge RPC fallback)

use axum::{
    Extension, Json,
    extract::{Path, Query, State},
};
use serde::Deserialize;
use shared::error::{AppError, ErrorCode};

use crate::auth::tenant_auth::TenantIdentity;
use crate::db::tenant_queries;
use crate::services::rpc::call_edge_rpc;
use crate::state::AppState;

use super::{ApiResult, verify_store};

/// GET /api/tenant/stores/:id/orders
#[derive(Deserialize)]
pub struct OrdersQuery {
    pub page: Option<i32>,
    pub per_page: Option<i32>,
    pub status: Option<String>,
}

pub async fn list_orders(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path(store_id): Path<i64>,
    Query(query): Query<OrdersQuery>,
) -> ApiResult<Vec<tenant_queries::ArchivedOrderSummary>> {
    verify_store(&state, store_id, identity.tenant_id).await?;

    let per_page = query.per_page.unwrap_or(20).min(100);
    let page = query.page.unwrap_or(1).max(1);
    let offset = (page - 1) * per_page;

    let orders = tenant_queries::list_orders(
        &state.pool,
        store_id,
        identity.tenant_id,
        query.status.as_deref(),
        per_page,
        offset,
    )
    .await
    .map_err(|e| {
        tracing::error!("Orders query error: {e}");
        AppError::new(ErrorCode::InternalError)
    })?;

    Ok(Json(orders))
}

/// GET /api/tenant/stores/:id/orders/:order_id/detail
///
/// Permanent detail store first, fallback to on-demand edge fetch if not yet synced.
pub async fn get_order_detail(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path((store_id, order_id)): Path<(i64, i64)>,
) -> ApiResult<shared::cloud::OrderDetailResponse> {
    verify_store(&state, store_id, identity.tenant_id).await?;

    // 1. Check permanent detail store (relational tables)
    if let Some(detail) =
        tenant_queries::get_order_detail(&state.pool, store_id, identity.tenant_id, order_id)
            .await
            .map_err(|e| {
                tracing::error!("Order detail query error: {e}");
                AppError::new(ErrorCode::InternalError)
            })?
    {
        let desglose =
            tenant_queries::get_order_desglose(&state.pool, store_id, identity.tenant_id, order_id)
                .await
                .map_err(|e| {
                    tracing::error!(order_id, "Failed to query desglose: {e}");
                    AppError::new(ErrorCode::InternalError)
                })?;

        return Ok(Json(shared::cloud::OrderDetailResponse {
            source: "cache".to_string(),
            detail,
            desglose,
        }));
    }

    // 2. Not yet synced — fetch from edge via RPC
    let rpc = shared::cloud::ws::CloudRpc::GetOrderDetail { order_id };
    let result = call_edge_rpc(&state.edges, store_id, rpc).await?;

    let (success, data, error) = match result {
        shared::cloud::ws::CloudRpcResult::Json {
            success,
            data,
            error,
        } => (success, data, error),
        _ => {
            return Err(AppError::with_message(
                ErrorCode::InternalError,
                "Unexpected RPC result type",
            ));
        }
    };

    if !success {
        return Err(AppError::with_message(
            ErrorCode::NotFound,
            error.unwrap_or_else(|| "Edge could not find order detail".to_string()),
        ));
    }

    let Some(data) = data else {
        return Err(AppError::with_message(
            ErrorCode::NotFound,
            "Edge returned empty order detail",
        ));
    };

    let detail_sync: shared::cloud::OrderDetailSync =
        serde_json::from_value(data).map_err(|e| {
            tracing::error!("Failed to deserialize on-demand OrderDetailSync: {e}");
            AppError::new(ErrorCode::InternalError)
        })?;

    // Detail will be written via normal sync pipeline — no best-effort caching needed

    Ok(Json(shared::cloud::OrderDetailResponse {
        source: "edge".to_string(),
        detail: detail_sync.detail,
        desglose: detail_sync.desglose,
    }))
}

/// GET /api/tenant/stores/:id/orders/:order_id/credit-notes
pub async fn list_credit_notes(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path((store_id, order_id)): Path<(i64, i64)>,
) -> ApiResult<Vec<tenant_queries::CreditNoteSummary>> {
    verify_store(&state, store_id, identity.tenant_id).await?;

    let notes = tenant_queries::list_credit_notes_by_order(
        &state.pool,
        store_id,
        identity.tenant_id,
        order_id,
    )
    .await
    .map_err(|e| {
        tracing::error!("Credit notes query error: {e}");
        AppError::new(ErrorCode::InternalError)
    })?;

    Ok(Json(notes))
}

/// GET /api/tenant/stores/:id/chain-entries
#[derive(Deserialize)]
pub struct ChainEntriesQuery {
    pub page: Option<i32>,
    pub per_page: Option<i32>,
}

pub async fn list_chain_entries(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path(store_id): Path<i64>,
    Query(query): Query<ChainEntriesQuery>,
) -> ApiResult<Vec<tenant_queries::ChainEntryItem>> {
    verify_store(&state, store_id, identity.tenant_id).await?;

    let per_page = query.per_page.unwrap_or(20).min(100);
    let page = query.page.unwrap_or(1).max(1);
    let offset = (page - 1) * per_page;

    let entries = tenant_queries::list_chain_entries(
        &state.pool,
        store_id,
        identity.tenant_id,
        per_page,
        offset,
    )
    .await
    .map_err(|e| {
        tracing::error!("Chain entries query error: {e}");
        AppError::new(ErrorCode::InternalError)
    })?;

    Ok(Json(entries))
}

/// GET /api/tenant/stores/:id/credit-notes/:source_id
pub async fn get_credit_note_detail(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path((store_id, source_id)): Path<(i64, i64)>,
) -> ApiResult<tenant_queries::CreditNoteDetail> {
    verify_store(&state, store_id, identity.tenant_id).await?;

    let detail = tenant_queries::get_credit_note_detail(
        &state.pool,
        store_id,
        identity.tenant_id,
        source_id,
    )
    .await
    .map_err(|e| {
        tracing::error!("Credit note detail query error: {e}");
        AppError::new(ErrorCode::InternalError)
    })?;

    match detail {
        Some(d) => Ok(Json(d)),
        None => Err(AppError::with_message(
            ErrorCode::NotFound,
            "Credit note not found",
        )),
    }
}

/// GET /api/tenant/stores/:id/anulaciones/:order_id
pub async fn get_anulacion_detail(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path((store_id, order_id)): Path<(i64, i64)>,
) -> ApiResult<tenant_queries::AnulacionDetail> {
    verify_store(&state, store_id, identity.tenant_id).await?;

    let detail =
        tenant_queries::get_anulacion_detail(&state.pool, store_id, identity.tenant_id, order_id)
            .await
            .map_err(|e| {
                tracing::error!("Anulacion detail query error: {e}");
                AppError::new(ErrorCode::InternalError)
            })?;

    match detail {
        Some(d) => Ok(Json(d)),
        None => Err(AppError::with_message(
            ErrorCode::NotFound,
            "Anulacion not found",
        )),
    }
}

/// GET /api/tenant/stores/:id/upgrades/:order_id
pub async fn get_upgrade_detail(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path((store_id, order_id)): Path<(i64, i64)>,
) -> ApiResult<tenant_queries::UpgradeDetail> {
    verify_store(&state, store_id, identity.tenant_id).await?;

    let detail =
        tenant_queries::get_upgrade_detail(&state.pool, store_id, identity.tenant_id, order_id)
            .await
            .map_err(|e| {
                tracing::error!("Upgrade detail query error: {e}");
                AppError::new(ErrorCode::InternalError)
            })?;

    match detail {
        Some(d) => Ok(Json(d)),
        None => Err(AppError::with_message(
            ErrorCode::NotFound,
            "Upgrade not found",
        )),
    }
}
