//! Attribute API Handlers

use axum::{
    Json,
    extract::{Extension, Path, State},
};

use crate::audit::AuditAction;
use crate::audit_log;
use crate::auth::CurrentUser;
use crate::core::ServerState;
use crate::db::models::{Attribute, AttributeCreate, AttributeOption, AttributeUpdate};
use crate::db::repository::AttributeRepository;
use crate::utils::{AppError, AppResult};

const RESOURCE: &str = "attribute";

/// GET /api/attributes - 获取所有属性
pub async fn list(State(state): State<ServerState>) -> AppResult<Json<Vec<Attribute>>> {
    let repo = AttributeRepository::new(state.db.clone());
    let attrs = repo
        .find_all()
        .await
        ?;
    Ok(Json(attrs))
}

/// GET /api/attributes/:id - 获取单个属性
pub async fn get_by_id(
    State(state): State<ServerState>,
    Path(id): Path<String>,
) -> AppResult<Json<Attribute>> {
    let repo = AttributeRepository::new(state.db.clone());
    let attr = repo
        .find_by_id(&id)
        .await
        ?
        .ok_or_else(|| AppError::not_found(format!("Attribute {} not found", id)))?;
    Ok(Json(attr))
}

/// POST /api/attributes - 创建属性
pub async fn create(
    State(state): State<ServerState>,
    Extension(current_user): Extension<CurrentUser>,
    Json(payload): Json<AttributeCreate>,
) -> AppResult<Json<Attribute>> {
    let repo = AttributeRepository::new(state.db.clone());
    let attr = repo
        .create(payload)
        .await
        ?;

    let id = attr.id.as_ref().map(|id| id.to_string()).unwrap_or_default();

    audit_log!(
        state.audit_service,
        AuditAction::AttributeCreated,
        "attribute", &id,
        operator_id = Some(current_user.id.clone()),
        operator_name = Some(current_user.display_name.clone()),
        details = serde_json::json!({"name": &attr.name})
    );

    state
        .broadcast_sync(RESOURCE, "created", &id, Some(&attr))
        .await;

    Ok(Json(attr))
}

/// PUT /api/attributes/:id - 更新属性
pub async fn update(
    State(state): State<ServerState>,
    Extension(current_user): Extension<CurrentUser>,
    Path(id): Path<String>,
    Json(payload): Json<AttributeUpdate>,
) -> AppResult<Json<Attribute>> {
    let repo = AttributeRepository::new(state.db.clone());
    let attr = repo
        .update(&id, payload)
        .await
        ?;

    audit_log!(
        state.audit_service,
        AuditAction::AttributeUpdated,
        "attribute", &id,
        operator_id = Some(current_user.id.clone()),
        operator_name = Some(current_user.display_name.clone()),
        details = serde_json::json!({"name": &attr.name})
    );

    state
        .broadcast_sync(RESOURCE, "updated", &id, Some(&attr))
        .await;

    Ok(Json(attr))
}

/// DELETE /api/attributes/:id - 删除属性 (软删除)
pub async fn delete(
    State(state): State<ServerState>,
    Extension(current_user): Extension<CurrentUser>,
    Path(id): Path<String>,
) -> AppResult<Json<bool>> {
    let repo = AttributeRepository::new(state.db.clone());
    let name_for_audit = repo.find_by_id(&id).await.ok().flatten()
        .map(|a| a.name.clone()).unwrap_or_default();
    let result = repo
        .delete(&id)
        .await
        ?;

    if result {
        audit_log!(
            state.audit_service,
            AuditAction::AttributeDeleted,
            "attribute", &id,
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

/// POST /api/attributes/:id/options - 添加选项
pub async fn add_option(
    State(state): State<ServerState>,
    Path(id): Path<String>,
    Json(option): Json<AttributeOption>,
) -> AppResult<Json<Attribute>> {
    let repo = AttributeRepository::new(state.db.clone());
    let attr = repo
        .add_option(&id, option)
        .await
        ?;

    // 广播同步通知
    state
        .broadcast_sync(RESOURCE, "updated", &id, Some(&attr))
        .await;

    Ok(Json(attr))
}

/// PUT /api/attributes/:id/options/:idx - 更新选项
pub async fn update_option(
    State(state): State<ServerState>,
    Path((id, idx)): Path<(String, usize)>,
    Json(option): Json<AttributeOption>,
) -> AppResult<Json<Attribute>> {
    let repo = AttributeRepository::new(state.db.clone());
    let attr = repo
        .update_option(&id, idx, option)
        .await
        ?;

    // 广播同步通知
    state
        .broadcast_sync(RESOURCE, "updated", &id, Some(&attr))
        .await;

    Ok(Json(attr))
}

/// DELETE /api/attributes/:id/options/:idx - 删除选项
pub async fn remove_option(
    State(state): State<ServerState>,
    Path((id, idx)): Path<(String, usize)>,
) -> AppResult<Json<Attribute>> {
    let repo = AttributeRepository::new(state.db.clone());
    let attr = repo
        .remove_option(&id, idx)
        .await
        ?;

    // 广播同步通知
    state
        .broadcast_sync(RESOURCE, "updated", &id, Some(&attr))
        .await;

    Ok(Json(attr))
}
