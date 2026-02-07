//! Print Destination API Handlers

use axum::{
    Json,
    extract::{Extension, Path, State},
};

use crate::audit::AuditAction;
use crate::audit_log;
use crate::auth::CurrentUser;
use crate::core::ServerState;
use crate::db::repository::print_destination;
use crate::utils::{AppError, AppResult};
use shared::models::{PrintDestination, PrintDestinationCreate, PrintDestinationUpdate};

const RESOURCE: &str = "print_destination";

/// GET /api/print-destinations - 获取所有打印目的地
pub async fn list(
    State(state): State<ServerState>,
) -> AppResult<Json<Vec<PrintDestination>>> {
    let items = print_destination::find_all(&state.pool).await?;
    Ok(Json(items))
}

/// GET /api/print-destinations/:id - 获取单个打印目的地
pub async fn get_by_id(
    State(state): State<ServerState>,
    Path(id): Path<i64>,
) -> AppResult<Json<PrintDestination>> {
    let item = print_destination::find_by_id(&state.pool, id)
        .await?
        .ok_or_else(|| AppError::not_found(format!("Print destination {} not found", id)))?;
    Ok(Json(item))
}

/// POST /api/print-destinations - 创建打印目的地
pub async fn create(
    State(state): State<ServerState>,
    Extension(current_user): Extension<CurrentUser>,
    Json(payload): Json<PrintDestinationCreate>,
) -> AppResult<Json<PrintDestination>> {
    let item = print_destination::create(&state.pool, payload).await?;

    let id = item.id.to_string();

    audit_log!(
        state.audit_service,
        AuditAction::PrintDestinationCreated,
        "print_destination", &id,
        operator_id = Some(current_user.id.clone()),
        operator_name = Some(current_user.display_name.clone()),
        details = serde_json::json!({"name": &item.name})
    );

    state
        .broadcast_sync(RESOURCE, "created", &id, Some(&item))
        .await;

    Ok(Json(item))
}

/// PUT /api/print-destinations/:id - 更新打印目的地
pub async fn update(
    State(state): State<ServerState>,
    Extension(current_user): Extension<CurrentUser>,
    Path(id): Path<i64>,
    Json(payload): Json<PrintDestinationUpdate>,
) -> AppResult<Json<PrintDestination>> {
    let item = print_destination::update(&state.pool, id, payload).await?;

    let id_str = id.to_string();

    audit_log!(
        state.audit_service,
        AuditAction::PrintDestinationUpdated,
        "print_destination", &id_str,
        operator_id = Some(current_user.id.clone()),
        operator_name = Some(current_user.display_name.clone()),
        details = serde_json::json!({"name": &item.name})
    );

    state
        .broadcast_sync(RESOURCE, "updated", &id_str, Some(&item))
        .await;

    Ok(Json(item))
}

/// DELETE /api/print-destinations/:id - 删除打印目的地
pub async fn delete(
    State(state): State<ServerState>,
    Extension(current_user): Extension<CurrentUser>,
    Path(id): Path<i64>,
) -> AppResult<Json<bool>> {
    tracing::info!(id = %id, "Deleting print destination");
    let name_for_audit = print_destination::find_by_id(&state.pool, id).await.ok().flatten()
        .map(|p| p.name.clone()).unwrap_or_default();
    let result = print_destination::delete(&state.pool, id).await?;

    tracing::info!(id = %id, result = %result, "Print destination delete result");

    let id_str = id.to_string();

    if result {
        audit_log!(
            state.audit_service,
            AuditAction::PrintDestinationDeleted,
            "print_destination", &id_str,
            operator_id = Some(current_user.id.clone()),
            operator_name = Some(current_user.display_name.clone()),
            details = serde_json::json!({"name": name_for_audit})
        );

        state
            .broadcast_sync::<()>(RESOURCE, "deleted", &id_str, None)
            .await;
    }

    Ok(Json(result))
}
