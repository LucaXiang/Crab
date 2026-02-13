//! Label Template API Handlers

use axum::{
    Json,
    extract::{Extension, Path, State},
};

use crate::audit::{create_diff, create_snapshot, AuditAction};
use crate::audit_log;
use crate::auth::CurrentUser;
use crate::core::ServerState;
use crate::db::repository::label_template;
use crate::utils::{AppError, AppResult};
use crate::utils::validation::{validate_required_text, validate_optional_text, MAX_NAME_LEN, MAX_NOTE_LEN};
use shared::models::{LabelTemplate, LabelTemplateCreate, LabelTemplateUpdate};

const RESOURCE: &str = "label_template";

fn validate_create(payload: &LabelTemplateCreate) -> AppResult<()> {
    validate_required_text(&payload.name, "name", MAX_NAME_LEN)?;
    validate_optional_text(&payload.description, "description", MAX_NOTE_LEN)?;
    // test_data can be large (JSON), use a generous limit
    Ok(())
}

fn validate_update(payload: &LabelTemplateUpdate) -> AppResult<()> {
    if let Some(name) = &payload.name {
        validate_required_text(name, "name", MAX_NAME_LEN)?;
    }
    validate_optional_text(&payload.description, "description", MAX_NOTE_LEN)?;
    Ok(())
}

/// GET /api/label-templates - List all active label templates
pub async fn list(State(state): State<ServerState>) -> AppResult<Json<Vec<LabelTemplate>>> {
    let templates = label_template::list(&state.pool).await?;
    Ok(Json(templates))
}

/// GET /api/label-templates/all - List all label templates (including inactive)
pub async fn list_all(State(state): State<ServerState>) -> AppResult<Json<Vec<LabelTemplate>>> {
    let templates = label_template::list_all(&state.pool).await?;
    Ok(Json(templates))
}

/// GET /api/label-templates/default - Get the default label template
pub async fn get_default(State(state): State<ServerState>) -> AppResult<Json<Option<LabelTemplate>>> {
    let template = label_template::get_default(&state.pool).await?;
    Ok(Json(template))
}

/// GET /api/label-templates/:id - Get a label template by ID
pub async fn get_by_id(
    State(state): State<ServerState>,
    Path(id): Path<i64>,
) -> AppResult<Json<LabelTemplate>> {
    let template = label_template::get(&state.pool, id)
        .await?
        .ok_or_else(|| AppError::not_found(format!("Label template {} not found", id)))?;
    Ok(Json(template))
}

/// POST /api/label-templates - Create a new label template
pub async fn create(
    State(state): State<ServerState>,
    Extension(current_user): Extension<CurrentUser>,
    Json(payload): Json<LabelTemplateCreate>,
) -> AppResult<Json<LabelTemplate>> {
    validate_create(&payload)?;

    let template = label_template::create(&state.pool, payload).await?;

    let id = template.id.to_string();

    audit_log!(
        state.audit_service,
        AuditAction::LabelTemplateCreated,
        "label_template", &id,
        operator_id = Some(current_user.id),
        operator_name = Some(current_user.display_name.clone()),
        details = create_snapshot(&template, "label_template")
    );

    state
        .broadcast_sync(RESOURCE, "created", &id, Some(&template))
        .await;

    Ok(Json(template))
}

/// PUT /api/label-templates/:id - Update a label template
pub async fn update(
    State(state): State<ServerState>,
    Extension(current_user): Extension<CurrentUser>,
    Path(id): Path<i64>,
    Json(payload): Json<LabelTemplateUpdate>,
) -> AppResult<Json<LabelTemplate>> {
    validate_update(&payload)?;

    let old_template = label_template::get(&state.pool, id)
        .await?
        .ok_or_else(|| AppError::not_found(format!("Label template {} not found", id)))?;

    let template = label_template::update(&state.pool, id, payload).await?;

    let id_str = id.to_string();

    audit_log!(
        state.audit_service,
        AuditAction::LabelTemplateUpdated,
        "label_template", &id_str,
        operator_id = Some(current_user.id),
        operator_name = Some(current_user.display_name.clone()),
        details = create_diff(&old_template, &template, "label_template")
    );

    state
        .broadcast_sync(RESOURCE, "updated", &id_str, Some(&template))
        .await;

    Ok(Json(template))
}

/// DELETE /api/label-templates/:id - Delete a label template (soft delete)
pub async fn delete(
    State(state): State<ServerState>,
    Extension(current_user): Extension<CurrentUser>,
    Path(id): Path<i64>,
) -> AppResult<Json<bool>> {
    let name_for_audit = label_template::get(&state.pool, id).await.ok().flatten()
        .map(|t| t.name.clone()).unwrap_or_default();
    let result = label_template::delete(&state.pool, id).await?;

    let id_str = id.to_string();

    if result {
        audit_log!(
            state.audit_service,
            AuditAction::LabelTemplateDeleted,
            "label_template", &id_str,
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
