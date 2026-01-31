//! Print Destination API Handlers

use axum::{
    Json,
    extract::{Extension, Path, State},
};

use crate::audit::AuditAction;
use crate::audit_log;
use crate::auth::CurrentUser;
use crate::core::ServerState;
use crate::db::models::{PrintDestination, PrintDestinationCreate, PrintDestinationUpdate};
use crate::db::repository::PrintDestinationRepository;
use crate::utils::{AppError, AppResult};

const RESOURCE: &str = "print_destination";

/// GET /api/print-destinations - 获取所有打印目的地
pub async fn list(
    State(state): State<ServerState>,
) -> AppResult<Json<Vec<PrintDestination>>> {
    let repo = PrintDestinationRepository::new(state.db.clone());
    let items = repo
        .find_all()
        .await
        ?;
    Ok(Json(items))
}

/// GET /api/print-destinations/:id - 获取单个打印目的地
pub async fn get_by_id(
    State(state): State<ServerState>,
    Path(id): Path<String>,
) -> AppResult<Json<PrintDestination>> {
    let repo = PrintDestinationRepository::new(state.db.clone());
    let item = repo
        .find_by_id(&id)
        .await
        ?
        .ok_or_else(|| AppError::not_found(format!("Print destination {} not found", id)))?;
    Ok(Json(item))
}

/// POST /api/print-destinations - 创建打印目的地
pub async fn create(
    State(state): State<ServerState>,
    Extension(current_user): Extension<CurrentUser>,
    Json(payload): Json<PrintDestinationCreate>,
) -> AppResult<Json<PrintDestination>> {
    let repo = PrintDestinationRepository::new(state.db.clone());
    let item = repo
        .create(payload)
        .await
        ?;

    let id = item.id.as_ref().map(|id| id.to_string()).unwrap_or_default();

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
    Path(id): Path<String>,
    Json(payload): Json<PrintDestinationUpdate>,
) -> AppResult<Json<PrintDestination>> {
    let repo = PrintDestinationRepository::new(state.db.clone());
    let item = repo
        .update(&id, payload)
        .await
        ?;

    audit_log!(
        state.audit_service,
        AuditAction::PrintDestinationUpdated,
        "print_destination", &id,
        operator_id = Some(current_user.id.clone()),
        operator_name = Some(current_user.display_name.clone()),
        details = serde_json::json!({"name": &item.name})
    );

    state
        .broadcast_sync(RESOURCE, "updated", &id, Some(&item))
        .await;

    Ok(Json(item))
}

/// DELETE /api/print-destinations/:id - 删除打印目的地
pub async fn delete(
    State(state): State<ServerState>,
    Extension(current_user): Extension<CurrentUser>,
    Path(id): Path<String>,
) -> AppResult<Json<bool>> {
    tracing::info!(id = %id, "Deleting print destination");
    let repo = PrintDestinationRepository::new(state.db.clone());
    let name_for_audit = repo.find_by_id(&id).await.ok().flatten()
        .map(|p| p.name.clone()).unwrap_or_default();
    let result = repo
        .delete(&id)
        .await
        ?;

    tracing::info!(id = %id, result = %result, "Print destination delete result");

    if result {
        audit_log!(
            state.audit_service,
            AuditAction::PrintDestinationDeleted,
            "print_destination", &id,
            operator_id = Some(current_user.id.clone()),
            operator_name = Some(current_user.display_name.clone()),
            details = serde_json::json!({"name": name_for_audit})
        );

        state
            .broadcast_sync::<()>(RESOURCE, "deleted", &id, None)
            .await;
    }

    Ok(Json(result))
}
