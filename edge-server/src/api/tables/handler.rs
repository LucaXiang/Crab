//! Dining Table API Handlers

use axum::{
    Json,
    extract::{Extension, Path, State},
};

use crate::audit::{create_diff, create_snapshot, AuditAction};
use crate::audit_log;
use crate::auth::CurrentUser;
use crate::core::ServerState;
use crate::db::repository::dining_table;
use crate::utils::{AppError, AppResult};
use shared::models::{DiningTable, DiningTableCreate, DiningTableUpdate};

const RESOURCE: &str = "dining_table";

/// GET /api/tables - 获取所有桌台
pub async fn list(State(state): State<ServerState>) -> AppResult<Json<Vec<DiningTable>>> {
    let tables = dining_table::find_all(&state.pool).await?;
    Ok(Json(tables))
}

/// GET /api/tables/:id - 获取单个桌台
pub async fn get_by_id(
    State(state): State<ServerState>,
    Path(id): Path<i64>,
) -> AppResult<Json<DiningTable>> {
    let table = dining_table::find_by_id(&state.pool, id)
        .await?
        .ok_or_else(|| AppError::not_found(format!("Table {} not found", id)))?;
    Ok(Json(table))
}

/// POST /api/tables - 创建桌台
pub async fn create(
    State(state): State<ServerState>,
    Extension(current_user): Extension<CurrentUser>,
    Json(payload): Json<DiningTableCreate>,
) -> AppResult<Json<DiningTable>> {
    let table = dining_table::create(&state.pool, payload).await?;

    let id = table.id.to_string();

    audit_log!(
        state.audit_service,
        AuditAction::TableCreated,
        "dining_table", &id,
        operator_id = Some(current_user.id.clone()),
        operator_name = Some(current_user.display_name.clone()),
        details = create_snapshot(&table, "dining_table")
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
    Path(id): Path<i64>,
    Json(payload): Json<DiningTableUpdate>,
) -> AppResult<Json<DiningTable>> {
    // 查询旧值（用于审计 diff）
    let old_table = dining_table::find_by_id(&state.pool, id)
        .await?
        .ok_or_else(|| AppError::not_found(format!("Table {}", id)))?;

    let table = dining_table::update(&state.pool, id, payload).await?;

    let id_str = id.to_string();

    audit_log!(
        state.audit_service,
        AuditAction::TableUpdated,
        "dining_table", &id_str,
        operator_id = Some(current_user.id.clone()),
        operator_name = Some(current_user.display_name.clone()),
        details = create_diff(&old_table, &table, "dining_table")
    );

    state
        .broadcast_sync(RESOURCE, "updated", &id_str, Some(&table))
        .await;

    Ok(Json(table))
}

/// DELETE /api/tables/:id - 删除桌台 (软删除)
pub async fn delete(
    State(state): State<ServerState>,
    Extension(current_user): Extension<CurrentUser>,
    Path(id): Path<i64>,
) -> AppResult<Json<bool>> {
    let name_for_audit = dining_table::find_by_id(&state.pool, id).await.ok().flatten()
        .map(|t| t.name.clone()).unwrap_or_default();
    let result = dining_table::delete(&state.pool, id).await?;

    let id_str = id.to_string();

    if result {
        audit_log!(
            state.audit_service,
            AuditAction::TableDeleted,
            "dining_table", &id_str,
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
