//! Zone API Handlers

use axum::{
    Json,
    extract::{Extension, Path, State},
};

use crate::audit::{create_diff, create_snapshot, AuditAction};
use crate::audit_log;
use crate::auth::CurrentUser;
use crate::core::ServerState;
use crate::db::repository::{dining_table, zone};
use crate::utils::{AppError, AppResult};
use crate::utils::validation::{validate_required_text, validate_optional_text, MAX_NAME_LEN, MAX_NOTE_LEN};
use shared::models::{DiningTable, Zone, ZoneCreate, ZoneUpdate};

const RESOURCE: &str = "zone";

fn validate_create(payload: &ZoneCreate) -> AppResult<()> {
    validate_required_text(&payload.name, "name", MAX_NAME_LEN)?;
    validate_optional_text(&payload.description, "description", MAX_NOTE_LEN)?;
    Ok(())
}

fn validate_update(payload: &ZoneUpdate) -> AppResult<()> {
    if let Some(name) = &payload.name {
        validate_required_text(name, "name", MAX_NAME_LEN)?;
    }
    validate_optional_text(&payload.description, "description", MAX_NOTE_LEN)?;
    Ok(())
}

/// GET /api/zones - 获取所有区域
pub async fn list(State(state): State<ServerState>) -> AppResult<Json<Vec<Zone>>> {
    let zones = zone::find_all(&state.pool).await?;
    Ok(Json(zones))
}

/// GET /api/zones/:id - 获取单个区域
pub async fn get_by_id(
    State(state): State<ServerState>,
    Path(id): Path<i64>,
) -> AppResult<Json<Zone>> {
    let z = zone::find_by_id(&state.pool, id)
        .await?
        .ok_or_else(|| AppError::not_found(format!("Zone {} not found", id)))?;
    Ok(Json(z))
}

/// POST /api/zones - 创建区域
pub async fn create(
    State(state): State<ServerState>,
    Extension(current_user): Extension<CurrentUser>,
    Json(payload): Json<ZoneCreate>,
) -> AppResult<Json<Zone>> {
    validate_create(&payload)?;

    let z = zone::create(&state.pool, payload).await?;

    let id = z.id.to_string();

    audit_log!(
        state.audit_service,
        AuditAction::ZoneCreated,
        "zone", &id,
        operator_id = Some(current_user.id),
        operator_name = Some(current_user.display_name.clone()),
        details = create_snapshot(&z, "zone")
    );

    state
        .broadcast_sync(RESOURCE, "created", &id, Some(&z))
        .await;

    Ok(Json(z))
}

/// PUT /api/zones/:id - 更新区域
pub async fn update(
    State(state): State<ServerState>,
    Extension(current_user): Extension<CurrentUser>,
    Path(id): Path<i64>,
    Json(payload): Json<ZoneUpdate>,
) -> AppResult<Json<Zone>> {
    validate_update(&payload)?;

    // 查询旧值（用于审计 diff）
    let old_zone = zone::find_by_id(&state.pool, id)
        .await?
        .ok_or_else(|| AppError::not_found(format!("Zone {}", id)))?;

    let z = zone::update(&state.pool, id, payload).await?;

    let id_str = id.to_string();
    audit_log!(
        state.audit_service,
        AuditAction::ZoneUpdated,
        "zone", &id_str,
        operator_id = Some(current_user.id),
        operator_name = Some(current_user.display_name.clone()),
        details = create_diff(&old_zone, &z, "zone")
    );

    state
        .broadcast_sync(RESOURCE, "updated", &id_str, Some(&z))
        .await;

    Ok(Json(z))
}

/// DELETE /api/zones/:id - 删除区域 (软删除)
pub async fn delete(
    State(state): State<ServerState>,
    Extension(current_user): Extension<CurrentUser>,
    Path(id): Path<i64>,
) -> AppResult<Json<bool>> {
    let name_for_audit = zone::find_by_id(&state.pool, id).await.ok().flatten()
        .map(|z| z.name.clone()).unwrap_or_default();
    let result = zone::delete(&state.pool, id).await?;

    if result {
        let id_str = id.to_string();
        audit_log!(
            state.audit_service,
            AuditAction::ZoneDeleted,
            "zone", &id_str,
            operator_id = Some(current_user.id),
            operator_name = Some(current_user.display_name.clone()),
            details = serde_json::json!({"name": name_for_audit})
        );

        state
            .broadcast_sync::<()>(RESOURCE, "deleted", &id_str, None)
            .await;
    }

    Ok(Json(result))
}

/// GET /api/zones/:id/tables - 获取区域内的所有桌台
pub async fn list_tables(
    State(state): State<ServerState>,
    Path(zone_id): Path<i64>,
) -> AppResult<Json<Vec<DiningTable>>> {
    let tables = dining_table::find_by_zone(&state.pool, zone_id).await?;
    Ok(Json(tables))
}
