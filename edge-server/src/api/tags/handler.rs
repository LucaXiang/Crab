//! Tag API Handlers

use axum::{
    Json,
    extract::{Extension, Path, State},
};

use crate::audit::{AuditAction, create_diff, create_snapshot};
use crate::audit_log;
use crate::auth::CurrentUser;
use crate::core::ServerState;
use crate::db::repository::tag;
use crate::utils::validation::{
    MAX_NAME_LEN, MAX_SHORT_TEXT_LEN, validate_optional_text, validate_required_text,
};
use crate::utils::{AppError, AppResult};
use shared::error::ErrorCode;
use shared::message::SyncChangeType;
use shared::models::{Tag, TagCreate, TagUpdate};

use shared::cloud::SyncResource;
const RESOURCE: SyncResource = SyncResource::Tag;

fn validate_create(payload: &TagCreate) -> AppResult<()> {
    validate_required_text(&payload.name, "name", MAX_NAME_LEN)?;
    validate_optional_text(&payload.color, "color", MAX_SHORT_TEXT_LEN)?;
    Ok(())
}

fn validate_update(payload: &TagUpdate) -> AppResult<()> {
    if let Some(name) = &payload.name {
        validate_required_text(name, "name", MAX_NAME_LEN)?;
    }
    validate_optional_text(&payload.color, "color", MAX_SHORT_TEXT_LEN)?;
    Ok(())
}

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
    let t = tag::find_by_id(&state.pool, id).await?.ok_or_else(|| {
        AppError::with_message(ErrorCode::TagNotFound, format!("Tag {} not found", id))
    })?;
    Ok(Json(t))
}

/// POST /api/tags - 创建标签
pub async fn create(
    State(state): State<ServerState>,
    Extension(current_user): Extension<CurrentUser>,
    Json(payload): Json<TagCreate>,
) -> AppResult<Json<Tag>> {
    validate_create(&payload)?;

    let t = tag::create(&state.pool, None, payload).await?;

    let id = t.id.to_string();

    audit_log!(
        state.audit_service,
        AuditAction::TagCreated,
        "tag",
        &id,
        operator_id = Some(current_user.id),
        operator_name = Some(current_user.display_name.clone()),
        details = create_snapshot(&t, "tag")
    );

    state
        .broadcast_sync(RESOURCE, SyncChangeType::Created, &id, Some(&t), false)
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
    validate_update(&payload)?;

    // 查询旧值（用于审计 diff）
    let old_tag = tag::find_by_id(&state.pool, id).await?.ok_or_else(|| {
        AppError::with_message(ErrorCode::TagNotFound, format!("Tag {} not found", id))
    })?;

    let t = tag::update(&state.pool, id, payload).await?;

    let id_str = id.to_string();
    audit_log!(
        state.audit_service,
        AuditAction::TagUpdated,
        "tag",
        &id_str,
        operator_id = Some(current_user.id),
        operator_name = Some(current_user.display_name.clone()),
        details = create_diff(&old_tag, &t, "tag")
    );

    state
        .broadcast_sync(RESOURCE, SyncChangeType::Updated, &id_str, Some(&t), false)
        .await;

    Ok(Json(t))
}

/// DELETE /api/tags/:id - 删除标签 (软删除)
pub async fn delete(
    State(state): State<ServerState>,
    Extension(current_user): Extension<CurrentUser>,
    Path(id): Path<i64>,
) -> AppResult<Json<bool>> {
    // 检查是否有商品正在使用此标签
    let product_count =
        sqlx::query_scalar!("SELECT COUNT(*) FROM product_tag WHERE tag_id = ?", id)
            .fetch_one(&state.pool)
            .await
            .unwrap_or(0);

    if product_count > 0 {
        return Err(AppError::with_message(
            ErrorCode::TagInUse,
            format!(
                "Cannot delete tag: {} product(s) are using it",
                product_count
            ),
        ));
    }

    let name_for_audit = tag::find_by_id(&state.pool, id)
        .await
        .ok()
        .flatten()
        .map(|t| t.name.clone())
        .unwrap_or_default();
    let result = tag::delete(&state.pool, id).await?;

    if result {
        let id_str = id.to_string();
        audit_log!(
            state.audit_service,
            AuditAction::TagDeleted,
            "tag",
            &id_str,
            operator_id = Some(current_user.id),
            operator_name = Some(current_user.display_name.clone()),
            details = serde_json::json!({"name": name_for_audit})
        );

        state
            .broadcast_sync::<()>(RESOURCE, SyncChangeType::Deleted, &id_str, None, false)
            .await;
    }

    Ok(Json(result))
}
