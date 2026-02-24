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
    verify_store(&state, store_id, &identity.tenant_id).await?;

    let per_page = query.per_page.unwrap_or(20).min(100);
    let page = query.page.unwrap_or(1).max(1);
    let offset = (page - 1) * per_page;

    let orders = tenant_queries::list_orders(
        &state.pool,
        store_id,
        &identity.tenant_id,
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

/// GET /api/tenant/stores/:id/orders/:order_key/detail
///
/// 30-day cache first, fallback to on-demand edge fetch if edge is online.
pub async fn get_order_detail(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path((store_id, order_key)): Path<(i64, String)>,
) -> ApiResult<shared::cloud::OrderDetailResponse> {
    verify_store(&state, store_id, &identity.tenant_id).await?;

    // 1. Check 30-day cache
    if let Some(detail_json) =
        tenant_queries::get_order_detail(&state.pool, store_id, &identity.tenant_id, &order_key)
            .await
            .map_err(|e| {
                tracing::error!("Order detail query error: {e}");
                AppError::new(ErrorCode::InternalError)
            })?
    {
        let detail: shared::cloud::OrderDetailPayload = serde_json::from_value(detail_json)
            .map_err(|e| {
                tracing::error!("Failed to deserialize cached OrderDetailPayload: {e}");
                AppError::new(ErrorCode::InternalError)
            })?;

        let desglose = tenant_queries::get_order_desglose(
            &state.pool,
            store_id,
            &identity.tenant_id,
            &order_key,
        )
        .await
        .map_err(|e| {
            tracing::error!(order_key = %order_key, "Failed to query desglose: {e}");
            AppError::new(ErrorCode::InternalError)
        })?;

        return Ok(Json(shared::cloud::OrderDetailResponse {
            source: "cache".to_string(),
            detail,
            desglose,
        }));
    }

    // 2. Cache miss — fetch from edge via RPC
    let rpc = shared::cloud::ws::CloudRpc::GetOrderDetail {
        order_key: order_key.clone(),
    };
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

    // 3. Write fetched detail back to cache (best-effort)
    let now = shared::util::now_millis();
    let archived_order_id: Option<i64> = sqlx::query_scalar(
        "SELECT id FROM store_archived_orders WHERE edge_server_id = $1 AND tenant_id = $2 AND order_key = $3",
    )
    .bind(store_id)
    .bind(&identity.tenant_id)
    .bind(&order_key)
    .fetch_optional(&state.pool)
    .await
    .unwrap_or(None);

    if let Some(order_id) = archived_order_id
        && let Ok(detail_json) = serde_json::to_value(&detail_sync.detail)
    {
        let _ = sqlx::query(
            r#"
                INSERT INTO store_order_details (archived_order_id, detail, synced_at)
                VALUES ($1, $2, $3)
                ON CONFLICT (archived_order_id)
                DO UPDATE SET detail = EXCLUDED.detail, synced_at = EXCLUDED.synced_at
                "#,
        )
        .bind(order_id)
        .bind(&detail_json)
        .bind(now)
        .execute(&state.pool)
        .await;
    }

    // Audit
    let fetch_detail = serde_json::json!({
        "order_key": order_key,
        "store_id": store_id,
    });
    let _ = crate::db::audit::log(
        &state.pool,
        &identity.tenant_id,
        "order_detail_fetched",
        Some(&fetch_detail),
        None,
        now,
    )
    .await;

    Ok(Json(shared::cloud::OrderDetailResponse {
        source: "edge".to_string(),
        detail: detail_sync.detail,
        desglose: detail_sync.desglose,
    }))
}
