//! Employee API Handlers

use axum::{
    Json,
    extract::{Extension, Path, State},
};

use crate::audit::AuditAction;
use crate::audit_log;
use crate::auth::CurrentUser;
use crate::core::ServerState;
use crate::db::models::{Employee, EmployeeCreate, EmployeeUpdate};
use crate::db::repository::EmployeeRepository;
use crate::utils::{AppError, AppResult};

const RESOURCE: &str = "employee";

/// List all employees (excluding system users)
pub async fn list(State(state): State<ServerState>) -> AppResult<Json<Vec<Employee>>> {
    let repo = EmployeeRepository::new(state.db.clone());
    let employees = repo
        .find_all()
        .await
        ?;
    Ok(Json(employees))
}

/// List all employees including inactive (excluding system users)
pub async fn list_with_inactive(
    State(state): State<ServerState>,
) -> AppResult<Json<Vec<Employee>>> {
    let repo = EmployeeRepository::new(state.db.clone());
    let employees = repo
        .find_all_with_inactive()
        .await
        ?;
    Ok(Json(employees))
}

/// Get employee by id
pub async fn get_by_id(
    State(state): State<ServerState>,
    Path(id): Path<String>,
) -> AppResult<Json<Employee>> {
    let repo = EmployeeRepository::new(state.db.clone());
    let employee = repo
        .find_by_id_safe(&id)
        .await
        ?
        .ok_or_else(|| AppError::not_found(format!("Employee {} not found", id)))?;
    Ok(Json(employee))
}

/// Create a new employee
pub async fn create(
    State(state): State<ServerState>,
    Extension(current_user): Extension<CurrentUser>,
    Json(payload): Json<EmployeeCreate>,
) -> AppResult<Json<Employee>> {
    let repo = EmployeeRepository::new(state.db.clone());
    let employee = repo
        .create(payload)
        .await
        ?;

    let id = employee.id.as_ref().map(|id| id.to_string()).unwrap_or_default();

    audit_log!(
        state.audit_service,
        AuditAction::EmployeeCreated,
        "employee", &id,
        operator_id = Some(current_user.id.clone()),
        operator_name = Some(current_user.display_name.clone()),
        details = serde_json::json!({"username": &employee.username, "role": employee.role.to_string()})
    );

    state
        .broadcast_sync(RESOURCE, "created", &id, Some(&employee))
        .await;

    Ok(Json(employee))
}

/// Update an employee
pub async fn update(
    State(state): State<ServerState>,
    Extension(current_user): Extension<CurrentUser>,
    Path(id): Path<String>,
    Json(payload): Json<EmployeeUpdate>,
) -> AppResult<Json<Employee>> {
    let repo = EmployeeRepository::new(state.db.clone());
    let employee = repo
        .update(&id, payload)
        .await
        ?;

    audit_log!(
        state.audit_service,
        AuditAction::EmployeeUpdated,
        "employee", &id,
        operator_id = Some(current_user.id.clone()),
        operator_name = Some(current_user.display_name.clone()),
        details = serde_json::json!({"username": &employee.username, "role": employee.role.to_string(), "is_active": employee.is_active})
    );

    state
        .broadcast_sync(RESOURCE, "updated", &id, Some(&employee))
        .await;

    Ok(Json(employee))
}

/// Soft delete an employee
pub async fn delete(
    State(state): State<ServerState>,
    Extension(current_user): Extension<CurrentUser>,
    Path(id): Path<String>,
) -> AppResult<Json<bool>> {
    let repo = EmployeeRepository::new(state.db.clone());
    // 删除前查名称用于审计
    let name_for_audit = repo.find_by_id(&id).await.ok().flatten()
        .map(|e| e.username.clone()).unwrap_or_default();
    let result = repo
        .delete(&id)
        .await
        ?;

    if result {
        audit_log!(
            state.audit_service,
            AuditAction::EmployeeDeleted,
            "employee", &id,
            operator_id = Some(current_user.id.clone()),
            operator_name = Some(current_user.display_name.clone()),
            details = serde_json::json!({"username": name_for_audit})
        );

        state
            .broadcast_sync::<()>(RESOURCE, "deleted", &id, None)
            .await;
    }

    Ok(Json(result))
}
