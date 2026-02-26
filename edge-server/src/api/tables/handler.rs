//! Dining Table API Handlers

use axum::{
    Json,
    extract::{Extension, Path, State},
};

use crate::audit::{AuditAction, create_diff, create_snapshot};
use crate::audit_log;
use crate::auth::CurrentUser;
use crate::core::ServerState;
use crate::db::repository::dining_table;
use crate::utils::validation::{MAX_NAME_LEN, validate_required_text};
use crate::utils::{AppError, AppResult};
use shared::error::ErrorCode;
use shared::message::SyncChangeType;
use shared::models::{DiningTable, DiningTableCreate, DiningTableUpdate};

use shared::cloud::SyncResource;
const RESOURCE: SyncResource = SyncResource::DiningTable;

fn validate_create(payload: &DiningTableCreate) -> AppResult<()> {
    validate_required_text(&payload.name, "name", MAX_NAME_LEN)?;
    Ok(())
}

fn validate_update(payload: &DiningTableUpdate) -> AppResult<()> {
    if let Some(name) = &payload.name {
        validate_required_text(name, "name", MAX_NAME_LEN)?;
    }
    Ok(())
}

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
        .ok_or_else(|| {
            AppError::with_message(ErrorCode::TableNotFound, format!("Table {} not found", id))
        })?;
    Ok(Json(table))
}

/// POST /api/tables - 创建桌台
pub async fn create(
    State(state): State<ServerState>,
    Extension(current_user): Extension<CurrentUser>,
    Json(payload): Json<DiningTableCreate>,
) -> AppResult<Json<DiningTable>> {
    validate_create(&payload)?;

    let table = dining_table::create(&state.pool, None, payload).await?;

    let id = table.id.to_string();

    audit_log!(
        state.audit_service,
        AuditAction::TableCreated,
        "dining_table",
        &id,
        operator_id = Some(current_user.id),
        operator_name = Some(current_user.name.clone()),
        details = create_snapshot(&table, "dining_table")
    );

    state
        .broadcast_sync(RESOURCE, SyncChangeType::Created, &id, Some(&table), false)
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
    validate_update(&payload)?;

    // 查询旧值（用于审计 diff）
    let old_table = dining_table::find_by_id(&state.pool, id)
        .await?
        .ok_or_else(|| {
            AppError::with_message(ErrorCode::TableNotFound, format!("Table {} not found", id))
        })?;

    let table = dining_table::update(&state.pool, id, payload).await?;

    let id_str = id.to_string();

    audit_log!(
        state.audit_service,
        AuditAction::TableUpdated,
        "dining_table",
        &id_str,
        operator_id = Some(current_user.id),
        operator_name = Some(current_user.name.clone()),
        details = create_diff(&old_table, &table, "dining_table")
    );

    state
        .broadcast_sync(
            RESOURCE,
            SyncChangeType::Updated,
            &id_str,
            Some(&table),
            false,
        )
        .await;

    Ok(Json(table))
}

/// DELETE /api/tables/:id - 删除桌台 (软删除)
pub async fn delete(
    State(state): State<ServerState>,
    Extension(current_user): Extension<CurrentUser>,
    Path(id): Path<i64>,
) -> AppResult<Json<bool>> {
    // Reject if table has active orders (stored in redb, not SQLite)
    if let Ok(Some(order_id)) = state
        .orders_manager
        .storage()
        .find_active_order_for_table(id)
    {
        return Err(AppError::with_message(
            ErrorCode::TableHasOrders,
            format!("Cannot delete table: active order {} exists", order_id),
        ));
    }

    let name_for_audit = dining_table::find_by_id(&state.pool, id)
        .await
        .ok()
        .flatten()
        .map(|t| t.name.clone())
        .unwrap_or_default();
    let result = dining_table::delete(&state.pool, id).await?;

    let id_str = id.to_string();

    if result {
        audit_log!(
            state.audit_service,
            AuditAction::TableDeleted,
            "dining_table",
            &id_str,
            operator_id = Some(current_user.id),
            operator_name = Some(current_user.name.clone()),
            details = serde_json::json!({"name": name_for_audit})
        );

        state
            .broadcast_sync::<()>(RESOURCE, SyncChangeType::Deleted, &id_str, None, false)
            .await;
    }

    Ok(Json(result))
}
