//! Print Destination API Handlers

use axum::{
    Json,
    extract::{Extension, Path, State},
};

use crate::audit::{create_diff, create_snapshot, AuditAction};
use crate::audit_log;
use crate::auth::CurrentUser;
use crate::core::ServerState;
use crate::db::repository::print_destination;
use crate::utils::{AppError, AppResult};
use crate::utils::validation::{validate_required_text, validate_optional_text, MAX_NAME_LEN, MAX_NOTE_LEN, MAX_SHORT_TEXT_LEN};
use shared::models::{PrintDestination, PrintDestinationCreate, PrintDestinationUpdate};

const RESOURCE: &str = "print_destination";

fn validate_create(payload: &PrintDestinationCreate) -> AppResult<()> {
    validate_required_text(&payload.name, "name", MAX_NAME_LEN)?;
    validate_optional_text(&payload.description, "description", MAX_NOTE_LEN)?;
    validate_required_text(&payload.purpose, "purpose", MAX_SHORT_TEXT_LEN)?;
    Ok(())
}

fn validate_update(payload: &PrintDestinationUpdate) -> AppResult<()> {
    if let Some(name) = &payload.name {
        validate_required_text(name, "name", MAX_NAME_LEN)?;
    }
    validate_optional_text(&payload.description, "description", MAX_NOTE_LEN)?;
    if let Some(purpose) = &payload.purpose {
        validate_required_text(purpose, "purpose", MAX_SHORT_TEXT_LEN)?;
    }
    Ok(())
}

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
    validate_create(&payload)?;

    let item = print_destination::create(&state.pool, payload).await?;

    let id = item.id.to_string();

    audit_log!(
        state.audit_service,
        AuditAction::PrintDestinationCreated,
        "print_destination", &id,
        operator_id = Some(current_user.id),
        operator_name = Some(current_user.display_name.clone()),
        details = create_snapshot(&item, "print_destination")
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
    validate_update(&payload)?;

    let old_item = print_destination::find_by_id(&state.pool, id)
        .await?
        .ok_or_else(|| AppError::not_found(format!("Print destination {} not found", id)))?;

    let item = print_destination::update(&state.pool, id, payload).await?;

    let id_str = id.to_string();

    audit_log!(
        state.audit_service,
        AuditAction::PrintDestinationUpdated,
        "print_destination", &id_str,
        operator_id = Some(current_user.id),
        operator_name = Some(current_user.display_name.clone()),
        details = create_diff(&old_item, &item, "print_destination")
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
    // 检查是否有分类正在使用此打印目标
    let total_refs = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM category_print_dest WHERE print_destination_id = ?",
        id
    )
    .fetch_one(&state.pool)
    .await
    .map_err(|e| AppError::internal(e.to_string()))?;
    if total_refs > 0 {
        return Err(AppError::validation(format!(
            "Cannot delete print destination: {} category reference(s) exist",
            total_refs
        )));
    }

    let name_for_audit = print_destination::find_by_id(&state.pool, id).await.ok().flatten()
        .map(|p| p.name.clone()).unwrap_or_default();
    let result = print_destination::delete(&state.pool, id).await?;

    let id_str = id.to_string();

    if result {
        audit_log!(
            state.audit_service,
            AuditAction::PrintDestinationDeleted,
            "print_destination", &id_str,
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
