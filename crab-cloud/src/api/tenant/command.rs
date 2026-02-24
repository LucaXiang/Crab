//! Remote command endpoints: send commands to edge, list history

use axum::{
    Extension, Json,
    extract::{Path, Query, State},
};
use serde::Deserialize;
use shared::error::{AppError, ErrorCode};

use crate::auth::tenant_auth::TenantIdentity;
use crate::db::commands;
use crate::services::rpc::call_edge_rpc;
use crate::state::AppState;

use super::{ApiResult, verify_store};

/// POST /api/tenant/stores/:id/commands
#[derive(Deserialize)]
pub struct CreateCommandRequest {
    pub command_type: String,
    #[serde(default)]
    pub payload: serde_json::Value,
}

pub async fn create_command(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path(store_id): Path<i64>,
    Json(req): Json<CreateCommandRequest>,
) -> ApiResult<serde_json::Value> {
    verify_store(&state, store_id, &identity.tenant_id).await?;

    // Map command_type string → CloudRpc
    let rpc = match req.command_type.as_str() {
        "get_status" => shared::cloud::ws::CloudRpc::GetStatus,
        "refresh_subscription" => shared::cloud::ws::CloudRpc::RefreshSubscription,
        "get_order_detail" => {
            let order_key = req
                .payload
                .get("order_key")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            shared::cloud::ws::CloudRpc::GetOrderDetail { order_key }
        }
        other => {
            return Err(AppError::with_message(
                ErrorCode::ValidationFailed,
                format!("Unknown command type: {other}"),
            ));
        }
    };

    let now = shared::util::now_millis();

    // Record in DB for audit history
    let command_id = commands::create_command(
        &state.pool,
        store_id,
        &identity.tenant_id,
        &req.command_type,
        &req.payload,
        now,
    )
    .await
    .map_err(|e| {
        tracing::error!("Create command error: {e}");
        AppError::new(ErrorCode::InternalError)
    })?;

    // Send RPC to edge
    let rpc_result = call_edge_rpc(&state.edges, store_id, rpc).await?;

    // Extract result and update DB record
    let (success, data, error) = match &rpc_result {
        shared::cloud::ws::CloudRpcResult::Json {
            success,
            data,
            error,
        } => (*success, data.clone(), error.clone()),
        shared::cloud::ws::CloudRpcResult::StoreOp(r) => {
            (r.success, serde_json::to_value(r).ok(), r.error.clone())
        }
    };

    let result_json = serde_json::json!({ "success": success, "data": data, "error": error });
    let _ = commands::complete_command(&state.pool, command_id, success, &result_json, now).await;

    let detail = serde_json::json!({
        "command_type": req.command_type,
        "command_id": command_id,
    });
    let _ = crate::db::audit::log(
        &state.pool,
        &identity.tenant_id,
        if success {
            "command_completed"
        } else {
            "command_failed"
        },
        Some(&detail),
        None,
        now,
    )
    .await;

    Ok(Json(serde_json::json!({
        "command_id": command_id,
        "success": success,
        "data": data,
        "error": error,
    })))
}

/// GET /api/tenant/stores/:id/commands
#[derive(Deserialize)]
pub struct CommandsQuery {
    pub page: Option<i32>,
    pub per_page: Option<i32>,
}

pub async fn list_commands(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path(store_id): Path<i64>,
    Query(query): Query<CommandsQuery>,
) -> ApiResult<Vec<commands::CommandRecord>> {
    verify_store(&state, store_id, &identity.tenant_id).await?;

    let per_page = query.per_page.unwrap_or(20).min(100);
    let page = query.page.unwrap_or(1).max(1);
    let offset = (page - 1) * per_page;

    let commands =
        commands::get_command_history(&state.pool, store_id, &identity.tenant_id, per_page, offset)
            .await
            .map_err(|e| {
                tracing::error!("Commands query error: {e}");
                AppError::new(ErrorCode::InternalError)
            })?;

    Ok(Json(commands))
}
