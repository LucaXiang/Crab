//! Employee API Handlers

use axum::{
    extract::{Path, State},
    Json,
};

use crate::core::ServerState;
use crate::db::models::{EmployeeCreate, EmployeeResponse, EmployeeUpdate};
use crate::db::repository::EmployeeRepository;
use crate::utils::{AppError, AppResult};

const RESOURCE: &str = "employee";

/// List all employees (excluding system users)
pub async fn list(State(state): State<ServerState>) -> AppResult<Json<Vec<EmployeeResponse>>> {
    let repo = EmployeeRepository::new(state.db.clone());
    let employees = repo
        .find_all()
        .await
        .map_err(|e| AppError::database(e.to_string()))?;
    Ok(Json(employees))
}

/// List all employees including inactive (excluding system users)
pub async fn list_with_inactive(
    State(state): State<ServerState>,
) -> AppResult<Json<Vec<EmployeeResponse>>> {
    let repo = EmployeeRepository::new(state.db.clone());
    let employees = repo
        .find_all_with_inactive()
        .await
        .map_err(|e| AppError::database(e.to_string()))?;
    Ok(Json(employees))
}

/// Get employee by id
pub async fn get_by_id(
    State(state): State<ServerState>,
    Path(id): Path<String>,
) -> AppResult<Json<EmployeeResponse>> {
    let repo = EmployeeRepository::new(state.db.clone());
    let employee = repo
        .find_by_id_safe(&id)
        .await
        .map_err(|e| AppError::database(e.to_string()))?
        .ok_or_else(|| AppError::not_found(format!("Employee {} not found", id)))?;
    Ok(Json(employee))
}

/// Create a new employee
pub async fn create(
    State(state): State<ServerState>,
    Json(payload): Json<EmployeeCreate>,
) -> AppResult<Json<EmployeeResponse>> {
    let repo = EmployeeRepository::new(state.db.clone());
    let employee = repo
        .create(payload)
        .await
        .map_err(|e| AppError::database(e.to_string()))?;

    let id = Some(employee.id.clone());
    state
        .broadcast_sync(RESOURCE, id.as_deref(), "created", Some(&employee))
        .await;

    Ok(Json(employee))
}

/// Update an employee
pub async fn update(
    State(state): State<ServerState>,
    Path(id): Path<String>,
    Json(payload): Json<EmployeeUpdate>,
) -> AppResult<Json<EmployeeResponse>> {
    let repo = EmployeeRepository::new(state.db.clone());
    let employee = repo
        .update(&id, payload)
        .await
        .map_err(|e| AppError::database(e.to_string()))?;

    state
        .broadcast_sync(RESOURCE, Some(&id), "updated", Some(&employee))
        .await;

    Ok(Json(employee))
}

/// Soft delete an employee
pub async fn delete(
    State(state): State<ServerState>,
    Path(id): Path<String>,
) -> AppResult<Json<bool>> {
    let repo = EmployeeRepository::new(state.db.clone());
    let result = repo
        .delete(&id)
        .await
        .map_err(|e| AppError::database(e.to_string()))?;

    if result {
        state
            .broadcast_sync::<()>(RESOURCE, Some(&id), "deleted", None)
            .await;
    }

    Ok(Json(result))
}
