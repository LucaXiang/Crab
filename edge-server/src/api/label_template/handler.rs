//! Label Template API Handlers

use axum::{
    Json,
    extract::{Path, State},
};
use surrealdb::RecordId;

use crate::core::ServerState;
use crate::db::models::{LabelTemplate, LabelTemplateCreate, LabelTemplateUpdate};
use crate::db::repository::LabelTemplateRepository;
use crate::utils::{AppError, AppResult};

const TABLE: &str = "label_template";
const RESOURCE: &str = "label_template";

/// GET /api/label-templates - List all active label templates
pub async fn list(State(state): State<ServerState>) -> AppResult<Json<Vec<LabelTemplate>>> {
    let repo = LabelTemplateRepository::new(state.db.clone(), state.images_dir());
    let templates = repo
        .list()
        .await
        ?;
    Ok(Json(templates))
}

/// GET /api/label-templates/all - List all label templates (including inactive)
pub async fn list_all(State(state): State<ServerState>) -> AppResult<Json<Vec<LabelTemplate>>> {
    let repo = LabelTemplateRepository::new(state.db.clone(), state.images_dir());
    let templates = repo
        .list_all()
        .await
        ?;
    Ok(Json(templates))
}

/// GET /api/label-templates/default - Get the default label template
pub async fn get_default(State(state): State<ServerState>) -> AppResult<Json<Option<LabelTemplate>>> {
    let repo = LabelTemplateRepository::new(state.db.clone(), state.images_dir());
    let template = repo
        .get_default()
        .await
        ?;
    Ok(Json(template))
}

/// GET /api/label-templates/:id - Get a label template by ID
pub async fn get_by_id(
    State(state): State<ServerState>,
    Path(id): Path<String>,
) -> AppResult<Json<LabelTemplate>> {
    let record_id = RecordId::from_table_key(TABLE, &id);
    let repo = LabelTemplateRepository::new(state.db.clone(), state.images_dir());
    let template = repo
        .get(&record_id)
        .await
        ?
        .ok_or_else(|| AppError::not_found(format!("Label template {} not found", id)))?;
    Ok(Json(template))
}

/// POST /api/label-templates - Create a new label template
pub async fn create(
    State(state): State<ServerState>,
    Json(payload): Json<LabelTemplateCreate>,
) -> AppResult<Json<LabelTemplate>> {
    let repo = LabelTemplateRepository::new(state.db.clone(), state.images_dir());
    let template = repo
        .create(payload)
        .await
        ?;

    // Broadcast sync notification
    let id = template.id.as_ref().map(|id| id.to_string()).unwrap_or_default();
    state
        .broadcast_sync(RESOURCE, "created", &id, Some(&template))
        .await;

    Ok(Json(template))
}

/// PUT /api/label-templates/:id - Update a label template
pub async fn update(
    State(state): State<ServerState>,
    Path(id): Path<String>,
    Json(payload): Json<LabelTemplateUpdate>,
) -> AppResult<Json<LabelTemplate>> {
    let record_id = RecordId::from_table_key(TABLE, &id);
    let repo = LabelTemplateRepository::new(state.db.clone(), state.images_dir());
    let template = repo
        .update(&record_id, payload)
        .await
        ?;

    // Broadcast sync notification
    state
        .broadcast_sync(RESOURCE, "updated", &id, Some(&template))
        .await;

    Ok(Json(template))
}

/// DELETE /api/label-templates/:id - Delete a label template (soft delete)
pub async fn delete(
    State(state): State<ServerState>,
    Path(id): Path<String>,
) -> AppResult<Json<bool>> {
    let record_id = RecordId::from_table_key(TABLE, &id);
    let repo = LabelTemplateRepository::new(state.db.clone(), state.images_dir());
    let result = repo
        .delete(&record_id)
        .await
        ?;

    // Broadcast sync notification
    if result {
        state
            .broadcast_sync::<()>(RESOURCE, "deleted", &id, None)
            .await;
    }

    Ok(Json(result))
}
