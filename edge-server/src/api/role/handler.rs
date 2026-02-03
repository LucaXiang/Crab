//! Role API Handlers

use axum::Json;
use axum::extract::{Extension, Path, Query, State};
use axum::response::IntoResponse;
use serde::Deserialize;

use crate::audit::{create_diff, create_snapshot, AuditAction};
use crate::audit_log;
use crate::auth::permissions::{is_valid_permission, ALL_PERMISSIONS};
use crate::auth::CurrentUser;
use crate::core::ServerState;
use crate::db::models::{Role, RoleCreate, RoleUpdate};
use crate::db::repository::RoleRepository;
use crate::utils::{AppError, AppResult};

/// 权限天花板校验：操作者只能分配自己拥有的权限
fn validate_permission_ceiling(
    current_user: &CurrentUser,
    permissions: &[String],
) -> AppResult<()> {
    for perm in permissions {
        if !is_valid_permission(perm) {
            return Err(AppError::invalid_request(format!(
                "Invalid permission: {}",
                perm
            )));
        }
        if !current_user.has_permission(perm) {
            return Err(AppError::forbidden(format!(
                "Cannot grant permission '{}': you do not have it yourself",
                perm
            )));
        }
    }
    Ok(())
}

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
    ?;

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
        ?
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

    // 权限天花板校验
    validate_permission_ceiling(&current_user, &payload.permissions)?;

    let repo = RoleRepository::new(state.get_db());
    let role = repo
        .create(payload)
        .await
        ?;

    let id = role.id.as_ref().map(|id| id.to_string()).unwrap_or_default();
    audit_log!(
        state.audit_service,
        AuditAction::RoleCreated,
        "role", &id,
        operator_id = Some(current_user.id.clone()),
        operator_name = Some(current_user.display_name.clone()),
        details = create_snapshot(&role, "role")
    );

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

    // 权限天花板校验（仅当 payload 包含 permissions 时）
    if let Some(ref permissions) = payload.permissions {
        validate_permission_ceiling(&current_user, permissions)?;
    }

    let repo = RoleRepository::new(state.get_db());

    // 查询旧值（用于审计 diff）
    let old_role = repo
        .find_by_id(&id)
        .await?
        .ok_or_else(|| AppError::not_found(format!("Role {}", id)))?;

    let role = repo.update(&id, payload).await?;

    audit_log!(
        state.audit_service,
        AuditAction::RoleUpdated,
        "role", &id,
        operator_id = Some(current_user.id.clone()),
        operator_name = Some(current_user.display_name.clone()),
        details = create_diff(&old_role, &role, "role")
    );

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
    let name_for_audit = repo.find_by_id(&id).await.ok().flatten()
        .map(|r| r.name.clone()).unwrap_or_default();
    let result = repo
        .delete(&id)
        .await
        ?;

    if result {
        audit_log!(
            state.audit_service,
            AuditAction::RoleDeleted,
            "role", &id,
            operator_id = Some(current_user.id.clone()),
            operator_name = Some(current_user.display_name.clone()),
            details = serde_json::json!({"role_name": name_for_audit})
        );
    }

    Ok(Json(result))
}

/// GET /api/permissions - Get all available permissions
pub async fn get_all_permissions() -> AppResult<impl IntoResponse> {
    let permissions: Vec<String> = ALL_PERMISSIONS
        .iter()
        .map(|s| s.to_string())
        .collect();
    Ok(Json(permissions))
}

/// GET /api/roles/{id}/permissions - Get role permissions
pub async fn get_role_permissions(
    State(state): State<ServerState>,
    Path(id): Path<String>,
) -> AppResult<Json<Vec<String>>> {
    let repo = RoleRepository::new(state.get_db());
    let role = repo
        .find_by_id(&id)
        .await
        ?
        .ok_or_else(|| AppError::not_found(format!("Role {} not found", id)))?;

    Ok(Json(role.permissions))
}

/// PUT /api/roles/{id}/permissions - Update role permissions
pub async fn update_role_permissions(
    State(state): State<ServerState>,
    Extension(current_user): Extension<CurrentUser>,
    Path(id): Path<String>,
    Json(permissions): Json<Vec<String>>,
) -> AppResult<Json<Role>> {
    tracing::info!(
        user_id = %current_user.id,
        username = %current_user.username,
        role_id = %id,
        permissions = ?permissions,
        "Updating role permissions"
    );

    // 权限天花板校验
    validate_permission_ceiling(&current_user, &permissions)?;

    let repo = RoleRepository::new(state.get_db());
    let update = RoleUpdate {
        name: None,
        display_name: None,
        description: None,
        permissions: Some(permissions),
        is_active: None,
    };

    // 查询旧值（用于审计 diff）
    let old_role = repo
        .find_by_id(&id)
        .await?
        .ok_or_else(|| AppError::not_found(format!("Role {}", id)))?;

    let role = repo.update(&id, update).await?;

    audit_log!(
        state.audit_service,
        AuditAction::RoleUpdated,
        "role", &id,
        operator_id = Some(current_user.id.clone()),
        operator_name = Some(current_user.display_name.clone()),
        details = create_diff(&old_role, &role, "role")
    );

    Ok(Json(role))
}
