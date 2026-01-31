//! Dining Table API Handlers

use axum::{
    Json,
    extract::{Extension, Path, State},
};

use crate::audit::AuditAction;
use crate::audit_log;
use crate::auth::CurrentUser;
use crate::core::ServerState;
use crate::db::models::{DiningTable, DiningTableCreate, DiningTableUpdate};
use crate::db::repository::DiningTableRepository;
use crate::utils::{AppError, AppResult};

const RESOURCE: &str = "dining_table";

/// GET /api/tables - 获取所有桌台
pub async fn list(State(state): State<ServerState>) -> AppResult<Json<Vec<DiningTable>>> {
    let repo = DiningTableRepository::new(state.db.clone());
    let tables = repo
        .find_all()
        .await
        ?;
    Ok(Json(tables))
}

/// GET /api/tables/:id - 获取单个桌台
pub async fn get_by_id(
    State(state): State<ServerState>,
    Path(id): Path<String>,
) -> AppResult<Json<DiningTable>> {
    let repo = DiningTableRepository::new(state.db.clone());
    let table = repo
        .find_by_id_with_zone(&id)
        .await
        ?
        .ok_or_else(|| AppError::not_found(format!("Table {} not found", id)))?;
    Ok(Json(table))
}

/// POST /api/tables - 创建桌台
pub async fn create(
    State(state): State<ServerState>,
    Extension(current_user): Extension<CurrentUser>,
    Json(payload): Json<DiningTableCreate>,
) -> AppResult<Json<DiningTable>> {
    let repo = DiningTableRepository::new(state.db.clone());
    let table = repo
        .create(payload)
        .await
        ?;

    let id = table.id.as_ref().map(|id| id.to_string()).unwrap_or_default();

    audit_log!(
        state.audit_service,
        AuditAction::TableCreated,
        "dining_table", &id,
        operator_id = Some(current_user.id.clone()),
        operator_name = Some(current_user.display_name.clone()),
        details = serde_json::json!({"name": &table.name})
    );

    state
        .broadcast_sync(RESOURCE, "created", &id, Some(&table))
        .await;

    Ok(Json(table))
}

/// PUT /api/tables/:id - 更新桌台
pub async fn update(
    State(state): State<ServerState>,
    Extension(current_user): Extension<CurrentUser>,
    Path(id): Path<String>,
    Json(payload): Json<DiningTableUpdate>,
) -> AppResult<Json<DiningTable>> {
    let repo = DiningTableRepository::new(state.db.clone());
    let table = repo
        .update(&id, payload)
        .await
        ?;

    audit_log!(
        state.audit_service,
        AuditAction::TableUpdated,
        "dining_table", &id,
        operator_id = Some(current_user.id.clone()),
        operator_name = Some(current_user.display_name.clone()),
        details = serde_json::json!({"name": &table.name})
    );

    state
        .broadcast_sync(RESOURCE, "updated", &id, Some(&table))
        .await;

    Ok(Json(table))
}

/// DELETE /api/tables/:id - 删除桌台 (软删除)
pub async fn delete(
    State(state): State<ServerState>,
    Extension(current_user): Extension<CurrentUser>,
    Path(id): Path<String>,
) -> AppResult<Json<bool>> {
    let repo = DiningTableRepository::new(state.db.clone());
    let result = repo
        .delete(&id)
        .await
        ?;

    if result {
        audit_log!(
            state.audit_service,
            AuditAction::TableDeleted,
            "dining_table", &id,
            operator_id = Some(current_user.id.clone()),
            operator_name = Some(current_user.display_name.clone()),
            details = serde_json::json!({})
        );

        state
            .broadcast_sync::<()>(RESOURCE, "deleted", &id, None)
            .await;
    }

    Ok(Json(result))
}
