//! Role API Handlers

use axum::Json;
use axum::extract::{Extension, Path, Query, State};
use axum::response::IntoResponse;
use serde::Deserialize;

use crate::auth::CurrentUser;
use crate::core::ServerState;
use crate::db::models::{Role, RoleCreate, RoleUpdate};
use crate::db::repository::RoleRepository;
use crate::utils::{AppError, AppResult};

/// Query filter for role listing
#[derive(Debug, Deserialize)]
pub struct RoleQuery {
    /// If true, return all roles (including inactive)
    /// If false or not specified, return only active roles
    all: Option<bool>,
}

/// GET /api/roles - Get all roles
pub async fn list(
    State(state): State<ServerState>,
    Extension(current_user): Extension<CurrentUser>,
    Query(query): Query<RoleQuery>,
) -> AppResult<impl IntoResponse> {
    tracing::info!(
        user_id = %current_user.id,
        username = %current_user.username,
        "Fetching roles"
    );

    let repo = RoleRepository::new(state.get_db());
    let roles = if query.all.unwrap_or(false) {
        repo.find_all_with_inactive().await
    } else {
        repo.find_all().await
    }
    .map_err(|e| AppError::database(e.to_string()))?;

    Ok(Json(roles))
}

/// GET /api/roles/{id} - Get role by ID
pub async fn get_by_id(
    State(state): State<ServerState>,
    Path(id): Path<String>,
) -> AppResult<Json<Role>> {
    let repo = RoleRepository::new(state.get_db());
    let role = repo
        .find_by_id(&id)
        .await
        .map_err(|e| AppError::database(e.to_string()))?
        .ok_or_else(|| AppError::not_found(format!("Role {} not found", id)))?;

    Ok(Json(role))
}

/// POST /api/roles - Create a new role
pub async fn create(
    State(state): State<ServerState>,
    Extension(current_user): Extension<CurrentUser>,
    Json(payload): Json<RoleCreate>,
) -> AppResult<Json<Role>> {
    tracing::info!(
        user_id = %current_user.id,
        username = %current_user.username,
        role_name = %payload.name,
        "Creating role"
    );

    let repo = RoleRepository::new(state.get_db());
    let role = repo
        .create(payload)
        .await
        .map_err(|e| AppError::database(e.to_string()))?;

    Ok(Json(role))
}

/// PUT /api/roles/{id} - Update a role
pub async fn update(
    State(state): State<ServerState>,
    Extension(current_user): Extension<CurrentUser>,
    Path(id): Path<String>,
    Json(payload): Json<RoleUpdate>,
) -> AppResult<Json<Role>> {
    tracing::info!(
        user_id = %current_user.id,
        username = %current_user.username,
        role_id = %id,
        "Updating role"
    );

    let repo = RoleRepository::new(state.get_db());
    let role = repo
        .update(&id, payload)
        .await
        .map_err(|e| AppError::database(e.to_string()))?;

    Ok(Json(role))
}

/// DELETE /api/roles/{id} - Delete a role
pub async fn delete(
    State(state): State<ServerState>,
    Extension(current_user): Extension<CurrentUser>,
    Path(id): Path<String>,
) -> AppResult<Json<bool>> {
    tracing::info!(
        user_id = %current_user.id,
        username = %current_user.username,
        role_id = %id,
        "Deleting role"
    );

    let repo = RoleRepository::new(state.get_db());
    let result = repo
        .delete(&id)
        .await
        .map_err(|e| AppError::database(e.to_string()))?;

    Ok(Json(result))
}
