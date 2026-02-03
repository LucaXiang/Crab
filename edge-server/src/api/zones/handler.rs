//! Zone API Handlers

use axum::{
    Json,
    extract::{Extension, Path, State},
};

use crate::audit::{create_diff, create_snapshot, AuditAction};
use crate::audit_log;
use crate::auth::CurrentUser;
use crate::core::ServerState;
use crate::db::models::{DiningTable, Zone, ZoneCreate, ZoneUpdate};
use crate::db::repository::{DiningTableRepository, ZoneRepository};
use crate::utils::{AppError, AppResult};

const RESOURCE: &str = "zone";

/// GET /api/zones - 获取所有区域
pub async fn list(State(state): State<ServerState>) -> AppResult<Json<Vec<Zone>>> {
    let repo = ZoneRepository::new(state.db.clone());
    let zones = repo
        .find_all()
        .await
        ?;
    Ok(Json(zones))
}

/// GET /api/zones/:id - 获取单个区域
pub async fn get_by_id(
    State(state): State<ServerState>,
    Path(id): Path<String>,
) -> AppResult<Json<Zone>> {
    let repo = ZoneRepository::new(state.db.clone());
    let zone = repo
        .find_by_id(&id)
        .await
        ?
        .ok_or_else(|| AppError::not_found(format!("Zone {} not found", id)))?;
    Ok(Json(zone))
}

/// POST /api/zones - 创建区域
pub async fn create(
    State(state): State<ServerState>,
    Extension(current_user): Extension<CurrentUser>,
    Json(payload): Json<ZoneCreate>,
) -> AppResult<Json<Zone>> {
    let repo = ZoneRepository::new(state.db.clone());
    let zone = repo
        .create(payload)
        .await
        ?;

    let id = zone.id.as_ref().map(|id| id.to_string()).unwrap_or_default();

    audit_log!(
        state.audit_service,
        AuditAction::ZoneCreated,
        "zone", &id,
        operator_id = Some(current_user.id.clone()),
        operator_name = Some(current_user.display_name.clone()),
        details = create_snapshot(&zone, "zone")
    );

    state
        .broadcast_sync(RESOURCE, "created", &id, Some(&zone))
        .await;

    Ok(Json(zone))
}

/// PUT /api/zones/:id - 更新区域
pub async fn update(
    State(state): State<ServerState>,
    Extension(current_user): Extension<CurrentUser>,
    Path(id): Path<String>,
    Json(payload): Json<ZoneUpdate>,
) -> AppResult<Json<Zone>> {
    let repo = ZoneRepository::new(state.db.clone());

    // 查询旧值（用于审计 diff）
    let old_zone = repo
        .find_by_id(&id)
        .await?
        .ok_or_else(|| AppError::not_found(format!("Zone {}", id)))?;

    let zone = repo.update(&id, payload).await?;

    audit_log!(
        state.audit_service,
        AuditAction::ZoneUpdated,
        "zone", &id,
        operator_id = Some(current_user.id.clone()),
        operator_name = Some(current_user.display_name.clone()),
        details = create_diff(&old_zone, &zone, "zone")
    );

    state
        .broadcast_sync(RESOURCE, "updated", &id, Some(&zone))
        .await;

    Ok(Json(zone))
}

/// DELETE /api/zones/:id - 删除区域 (软删除)
pub async fn delete(
    State(state): State<ServerState>,
    Extension(current_user): Extension<CurrentUser>,
    Path(id): Path<String>,
) -> AppResult<Json<bool>> {
    let repo = ZoneRepository::new(state.db.clone());
    let name_for_audit = repo.find_by_id(&id).await.ok().flatten()
        .map(|z| z.name.clone()).unwrap_or_default();
    let result = repo
        .delete(&id)
        .await
        ?;

    if result {
        audit_log!(
            state.audit_service,
            AuditAction::ZoneDeleted,
            "zone", &id,
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

/// GET /api/zones/:id/tables - 获取区域内的所有桌台
pub async fn list_tables(
    State(state): State<ServerState>,
    Path(zone_id): Path<String>,
) -> AppResult<Json<Vec<DiningTable>>> {
    let repo = DiningTableRepository::new(state.db.clone());
    let tables = repo
        .find_by_zone(&zone_id)
        .await
        ?;
    Ok(Json(tables))
}
