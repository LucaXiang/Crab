use axum::Json;
use axum::extract::{Extension, Query, State};
use axum::response::IntoResponse;
use serde::Deserialize;

use crate::AppError;
use crate::auth::CurrentUser;
use crate::core::ServerState;
use crate::db::models::role::Role;
use crate::utils::AppResult;

/// Query filter for role listing
#[derive(Debug, Deserialize)]
pub struct RoleQuery {
    /// If true, return all roles (including inactive)
    /// If false or not specified, return only active roles
    all: Option<bool>,
}

/// Get all roles or active roles only
pub async fn get(
    State(state): State<ServerState>,
    Extension(current_user): Extension<CurrentUser>,
    Query(query): Query<RoleQuery>,
) -> AppResult<impl IntoResponse> {
    let db = state.get_db();

    // Log the request
    tracing::info!(
        user_id = %current_user.id,
        username = %current_user.username,
        "Fetching roles"
    );

    // Build SQL query based on filter
    let sql = if query.all.unwrap_or(false) {
        "SELECT id.id() as id, * FROM role"
    } else {
        "SELECT id.id() as id, * FROM role WHERE is_active = true"
    };

    // Execute query and return results
    let result: Vec<Role> = db
        .query(sql)
        .await
        .map_err(|e| AppError::database(e.to_string()))?
        .take(0)
        .map_err(|e| AppError::database(e.to_string()))?;

    Ok(Json(result))
}
