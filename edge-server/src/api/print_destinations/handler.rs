//! Print Destination API Handlers

use axum::{
    Json,
    extract::{Extension, Path, State},
};

use crate::audit::{AuditAction, create_diff, create_snapshot};
use crate::audit_log;
use crate::auth::CurrentUser;
use crate::core::ServerState;
use crate::db::repository::{print_config, print_destination};
use crate::utils::validation::{
    MAX_NAME_LEN, MAX_NOTE_LEN, MAX_SHORT_TEXT_LEN, validate_optional_text, validate_required_text,
};
use crate::utils::{AppError, AppResult};
use shared::error::ErrorCode;
use shared::models::{PrintDestination, PrintDestinationCreate, PrintDestinationUpdate};

use shared::cloud::SyncResource;
const RESOURCE: SyncResource = SyncResource::PrintDestination;

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
pub async fn list(State(state): State<ServerState>) -> AppResult<Json<Vec<PrintDestination>>> {
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
        .ok_or_else(|| {
            AppError::with_message(
                ErrorCode::PrintDestinationNotFound,
                format!("Print destination {} not found", id),
            )
        })?;
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
        "print_destination",
        &id,
        operator_id = Some(current_user.id),
        operator_name = Some(current_user.display_name.clone()),
        details = create_snapshot(&item, "print_destination")
    );

    state
        .broadcast_sync(RESOURCE, "created", &id, Some(&item))
        .await;

    // Auto-set as global default if none exists for this purpose
    auto_set_default_if_missing(&state, &item).await;

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
        .ok_or_else(|| {
            AppError::with_message(
                ErrorCode::PrintDestinationNotFound,
                format!("Print destination {} not found", id),
            )
        })?;

    let item = print_destination::update(&state.pool, id, payload).await?;

    let id_str = id.to_string();

    audit_log!(
        state.audit_service,
        AuditAction::PrintDestinationUpdated,
        "print_destination",
        &id_str,
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
        return Err(AppError::with_message(
            ErrorCode::PrintDestinationInUse,
            format!(
                "Cannot delete print destination: {} category reference(s) exist",
                total_refs
            ),
        ));
    }

    let name_for_audit = print_destination::find_by_id(&state.pool, id)
        .await
        .ok()
        .flatten()
        .map(|p| p.name.clone())
        .unwrap_or_default();
    let result = print_destination::delete(&state.pool, id).await?;

    let id_str = id.to_string();

    if result {
        audit_log!(
            state.audit_service,
            AuditAction::PrintDestinationDeleted,
            "print_destination",
            &id_str,
            operator_id = Some(current_user.id),
            operator_name = Some(current_user.display_name.clone()),
            details = serde_json::json!({"name": name_for_audit})
        );

        state
            .broadcast_sync::<()>(RESOURCE, "deleted", &id_str, None)
            .await;

        // Clean up global default if it referenced the deleted destination
        clear_default_if_deleted(&state, id).await;
    }

    Ok(Json(result))
}

// =============================================================================
// Print Config Auto-Sync Helpers
// =============================================================================

/// After creating a PrintDestination, auto-set it as the global default
/// if no default exists for this purpose.
async fn auto_set_default_if_missing(state: &ServerState, dest: &PrintDestination) {
    let defaults = state.catalog_service.get_print_defaults();
    let id_str = dest.id.to_string();

    let (kitchen, label) = match dest.purpose.as_str() {
        "kitchen" if defaults.kitchen_destination.is_none() => {
            (Some(id_str), defaults.label_destination)
        }
        "label" if defaults.label_destination.is_none() => {
            (defaults.kitchen_destination, Some(id_str))
        }
        _ => return,
    };

    if let Err(e) = print_config::update(&state.pool, kitchen.as_deref(), label.as_deref()).await {
        tracing::error!(error = ?e, "Failed to auto-set print_config default");
        return;
    }

    state
        .catalog_service
        .set_print_defaults(kitchen.clone(), label.clone());

    tracing::info!(
        kitchen = ?kitchen,
        label = ?label,
        "Auto-set print_config default for new destination"
    );

    broadcast_print_config(state, kitchen, label).await;
}

/// After deleting a PrintDestination, clear or fall back the global default
/// if it referenced the deleted ID.
async fn clear_default_if_deleted(state: &ServerState, deleted_id: i64) {
    let defaults = state.catalog_service.get_print_defaults();
    let deleted_str = deleted_id.to_string();

    let mut kitchen = defaults.kitchen_destination;
    let mut label = defaults.label_destination;
    let mut changed = false;

    if kitchen.as_deref() == Some(&deleted_str) {
        kitchen = find_next_active_destination(&state.pool, "kitchen", deleted_id).await;
        changed = true;
    }
    if label.as_deref() == Some(&deleted_str) {
        label = find_next_active_destination(&state.pool, "label", deleted_id).await;
        changed = true;
    }

    if !changed {
        return;
    }

    if let Err(e) = print_config::update(&state.pool, kitchen.as_deref(), label.as_deref()).await {
        tracing::error!(error = ?e, "Failed to update print_config after deletion");
        return;
    }

    state
        .catalog_service
        .set_print_defaults(kitchen.clone(), label.clone());

    tracing::info!(
        kitchen = ?kitchen,
        label = ?label,
        "Updated print_config default after destination deletion"
    );

    broadcast_print_config(state, kitchen, label).await;
}

/// Find the next active PrintDestination with the given purpose, excluding `excluded_id`.
async fn find_next_active_destination(
    pool: &sqlx::SqlitePool,
    purpose: &str,
    excluded_id: i64,
) -> Option<String> {
    sqlx::query_scalar::<_, i64>(
        "SELECT id FROM print_destination WHERE purpose = ? AND is_active = 1 AND id != ? LIMIT 1",
    )
    .bind(purpose)
    .bind(excluded_id)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()
    .map(|id| id.to_string())
}

/// Broadcast a print_config update to connected clients.
async fn broadcast_print_config(
    state: &ServerState,
    kitchen: Option<String>,
    label: Option<String>,
) {
    let config = serde_json::json!({
        "default_kitchen_printer": kitchen,
        "default_label_printer": label,
    });
    state
        .broadcast_sync(
            SyncResource::PrintConfig,
            "updated",
            "default",
            Some(&config),
        )
        .await;
}
