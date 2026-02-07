//! Tag API Handlers

use axum::{
    Json,
    extract::{Extension, Path, State},
};

use crate::audit::{create_diff, create_snapshot, AuditAction};
use crate::audit_log;
use crate::auth::CurrentUser;
use crate::core::ServerState;
use crate::db::repository::tag;
use crate::utils::{AppError, AppResult};
use shared::models::{Tag, TagCreate, TagUpdate};

const RESOURCE: &str = "tag";

/// GET /api/tags - 获取所有标签
pub async fn list(State(state): State<ServerState>) -> AppResult<Json<Vec<Tag>>> {
    let tags = tag::find_all(&state.pool).await?;
    Ok(Json(tags))
}

/// GET /api/tags/:id - 获取单个标签
pub async fn get_by_id(
    State(state): State<ServerState>,
    Path(id): Path<i64>,
) -> AppResult<Json<Tag>> {
    let t = tag::find_by_id(&state.pool, id)
        .await?
        .ok_or_else(|| AppError::not_found(format!("Tag {} not found", id)))?;
    Ok(Json(t))
}

/// POST /api/tags - 创建标签
pub async fn create(
    State(state): State<ServerState>,
    Extension(current_user): Extension<CurrentUser>,
    Json(payload): Json<TagCreate>,
) -> AppResult<Json<Tag>> {
    let t = tag::create(&state.pool, payload).await?;

    let id = t.id.to_string();

    audit_log!(
        state.audit_service,
        AuditAction::TagCreated,
        "tag", &id,
        operator_id = Some(current_user.id.clone()),
        operator_name = Some(current_user.display_name.clone()),
        details = create_snapshot(&t, "tag")
    );

    state
        .broadcast_sync(RESOURCE, "created", &id, Some(&t))
        .await;

    Ok(Json(t))
}

/// PUT /api/tags/:id - 更新标签
pub async fn update(
    State(state): State<ServerState>,
    Extension(current_user): Extension<CurrentUser>,
    Path(id): Path<i64>,
    Json(payload): Json<TagUpdate>,
) -> AppResult<Json<Tag>> {
    // 查询旧值（用于审计 diff）
    let old_tag = tag::find_by_id(&state.pool, id)
        .await?
        .ok_or_else(|| AppError::not_found(format!("Tag {}", id)))?;

    let t = tag::update(&state.pool, id, payload).await?;

    let id_str = id.to_string();
    audit_log!(
        state.audit_service,
        AuditAction::TagUpdated,
        "tag", &id_str,
        operator_id = Some(current_user.id.clone()),
        operator_name = Some(current_user.display_name.clone()),
        details = create_diff(&old_tag, &t, "tag")
    );

    state
        .broadcast_sync(RESOURCE, "updated", &id_str, Some(&t))
        .await;

    Ok(Json(t))
}

/// DELETE /api/tags/:id - 删除标签 (软删除)
pub async fn delete(
    State(state): State<ServerState>,
    Extension(current_user): Extension<CurrentUser>,
    Path(id): Path<i64>,
) -> AppResult<Json<bool>> {
    let name_for_audit = tag::find_by_id(&state.pool, id).await.ok().flatten()
        .map(|t| t.name.clone()).unwrap_or_default();
    let result = tag::delete(&state.pool, id).await?;

    if result {
        let id_str = id.to_string();
        audit_log!(
            state.audit_service,
            AuditAction::TagDeleted,
            "tag", &id_str,
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
