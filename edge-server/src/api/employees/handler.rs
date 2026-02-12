//! Employee API Handlers

use axum::{
    Json,
    extract::{Extension, Path, State},
};

use crate::audit::{create_diff, create_snapshot, AuditAction};
use crate::audit_log;
use crate::auth::CurrentUser;
use crate::core::ServerState;
use crate::db::repository::employee;
use crate::utils::{AppError, AppResult};
use shared::models::{Employee, EmployeeCreate, EmployeeUpdate};

const RESOURCE: &str = "employee";

/// List all employees (excluding system users)
pub async fn list(State(state): State<ServerState>) -> AppResult<Json<Vec<Employee>>> {
    let employees = employee::find_all(&state.pool).await?;
    Ok(Json(employees))
}

/// List all employees including inactive (excluding system users)
pub async fn list_with_inactive(
    State(state): State<ServerState>,
) -> AppResult<Json<Vec<Employee>>> {
    let employees = employee::find_all_with_inactive(&state.pool).await?;
    Ok(Json(employees))
}

/// Get employee by id
pub async fn get_by_id(
    State(state): State<ServerState>,
    Path(id): Path<i64>,
) -> AppResult<Json<Employee>> {
    let employee = employee::find_by_id(&state.pool, id)
        .await?
        .ok_or_else(|| AppError::not_found(format!("Employee {} not found", id)))?;
    Ok(Json(employee))
}

/// Create a new employee
pub async fn create(
    State(state): State<ServerState>,
    Extension(current_user): Extension<CurrentUser>,
    Json(payload): Json<EmployeeCreate>,
) -> AppResult<Json<Employee>> {
    let emp = employee::create(&state.pool, payload).await?;

    let id = emp.id.to_string();

    audit_log!(
        state.audit_service,
        AuditAction::EmployeeCreated,
        "employee", &id,
        operator_id = Some(current_user.id),
        operator_name = Some(current_user.display_name.clone()),
        details = create_snapshot(&emp, "employee")
    );

    state
        .broadcast_sync(RESOURCE, "created", &id, Some(&emp))
        .await;

    Ok(Json(emp))
}

/// Update an employee
pub async fn update(
    State(state): State<ServerState>,
    Extension(current_user): Extension<CurrentUser>,
    Path(id): Path<i64>,
    Json(payload): Json<EmployeeUpdate>,
) -> AppResult<Json<Employee>> {
    // 查询旧值（用于审计 diff）
    let old_employee = employee::find_by_id(&state.pool, id)
        .await?
        .ok_or_else(|| AppError::not_found(format!("Employee {}", id)))?;

    let emp = employee::update(&state.pool, id, payload).await?;

    let id_str = id.to_string();
    audit_log!(
        state.audit_service,
        AuditAction::EmployeeUpdated,
        "employee", &id_str,
        operator_id = Some(current_user.id),
        operator_name = Some(current_user.display_name.clone()),
        details = create_diff(&old_employee, &emp, "employee")
    );

    state
        .broadcast_sync(RESOURCE, "updated", &id_str, Some(&emp))
        .await;

    Ok(Json(emp))
}

/// Soft delete an employee
pub async fn delete(
    State(state): State<ServerState>,
    Extension(current_user): Extension<CurrentUser>,
    Path(id): Path<i64>,
) -> AppResult<Json<bool>> {
    // 删除前查信息用于审计
    let emp_for_audit = employee::find_by_id(&state.pool, id).await.ok().flatten();
    let result = employee::delete(&state.pool, id).await?;

    if result {
        let id_str = id.to_string();
        let (name, username) = emp_for_audit
            .map(|e| (e.display_name, e.username))
            .unwrap_or_default();
        audit_log!(
            state.audit_service,
            AuditAction::EmployeeDeleted,
            "employee", &id_str,
            operator_id = Some(current_user.id),
            operator_name = Some(current_user.display_name.clone()),
            details = serde_json::json!({"name": name, "username": username})
        );

        state
            .broadcast_sync::<()>(RESOURCE, "deleted", &id_str, None)
            .await;
    }

    Ok(Json(result))
}
