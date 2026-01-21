//! Employee Repository

use super::{BaseRepository, RepoError, RepoResult, make_thing, strip_table_prefix};
use crate::db::models::{Employee, EmployeeCreate, EmployeeResponse, EmployeeUpdate};
use surrealdb::Surreal;
use surrealdb::engine::local::Db;

const TABLE: &str = "employee";

#[derive(Clone)]
pub struct EmployeeRepository {
    base: BaseRepository,
}

impl EmployeeRepository {
    pub fn new(db: Surreal<Db>) -> Self {
        Self {
            base: BaseRepository::new(db),
        }
    }

    /// Find all active employees (excluding system users)
    pub async fn find_all(&self) -> RepoResult<Vec<EmployeeResponse>> {
        let employees: Vec<Employee> = self
            .base
            .db()
            .query("SELECT * FROM employee WHERE is_active = true AND is_system = false ORDER BY username")
            .await?
            .take(0)?;
        Ok(employees.into_iter().map(|e| e.into()).collect())
    }

    /// Find all employees including inactive (excluding system users)
    pub async fn find_all_with_inactive(&self) -> RepoResult<Vec<EmployeeResponse>> {
        let employees: Vec<Employee> = self
            .base
            .db()
            .query("SELECT * FROM employee WHERE is_system = false ORDER BY username")
            .await?
            .take(0)?;
        Ok(employees.into_iter().map(|e| e.into()).collect())
    }

    /// Find employee by id
    pub async fn find_by_id(&self, id: &str) -> RepoResult<Option<Employee>> {
        let pure_id = strip_table_prefix(TABLE, id);
        let emp: Option<Employee> = self.base.db().select((TABLE, pure_id)).await?;
        Ok(emp)
    }

    /// Find employee by id (returns EmployeeResponse without password)
    pub async fn find_by_id_safe(&self, id: &str) -> RepoResult<Option<EmployeeResponse>> {
        let pure_id = strip_table_prefix(TABLE, id);
        let emp: Option<Employee> = self.base.db().select((TABLE, pure_id)).await?;
        Ok(emp.map(|e| e.into()))
    }

    /// Find employee by username
    pub async fn find_by_username(&self, username: &str) -> RepoResult<Option<Employee>> {
        let username_owned = username.to_string();
        let mut result = self
            .base
            .db()
            .query("SELECT * FROM employee WHERE username = $username LIMIT 1")
            .bind(("username", username_owned))
            .await?;
        let employees: Vec<Employee> = result.take(0)?;
        Ok(employees.into_iter().next())
    }

    /// Create a new employee
    pub async fn create(&self, data: EmployeeCreate) -> RepoResult<EmployeeResponse> {
        // Check duplicate username
        if self.find_by_username(&data.username).await?.is_some() {
            return Err(RepoError::Duplicate(format!(
                "Username '{}' already exists",
                data.username
            )));
        }

        // Hash password
        let hash_pass = Employee::hash_password(&data.password)
            .map_err(|e| RepoError::Database(format!("Failed to hash password: {}", e)))?;

        let employee = Employee {
            id: None,
            username: data.username,
            hash_pass,
            role: data.role,
            is_system: false,
            is_active: true,
        };

        let created: Option<Employee> = self.base.db().create(TABLE).content(employee).await?;
        created
            .map(|e| e.into())
            .ok_or_else(|| RepoError::Database("Failed to create employee".to_string()))
    }

    /// Update an employee
    pub async fn update(&self, id: &str, data: EmployeeUpdate) -> RepoResult<EmployeeResponse> {
        let pure_id = strip_table_prefix(TABLE, id);
        let existing = self
            .find_by_id(pure_id)
            .await?
            .ok_or_else(|| RepoError::NotFound(format!("Employee {} not found", id)))?;

        // Prevent modifying system users
        if existing.is_system {
            return Err(RepoError::Validation(
                "Cannot modify system user".to_string(),
            ));
        }

        // Check duplicate username if changing
        if let Some(ref new_username) = data.username
            && new_username != &existing.username
            && self.find_by_username(new_username).await?.is_some()
        {
            return Err(RepoError::Duplicate(format!(
                "Username '{}' already exists",
                new_username
            )));
        }

        // Build update document
        #[derive(serde::Serialize)]
        struct UpdateDoc {
            #[serde(skip_serializing_if = "Option::is_none")]
            username: Option<String>,
            #[serde(skip_serializing_if = "Option::is_none")]
            hash_pass: Option<String>,
            #[serde(skip_serializing_if = "Option::is_none")]
            role: Option<surrealdb::sql::Thing>,
            #[serde(skip_serializing_if = "Option::is_none")]
            is_active: Option<bool>,
        }

        let hash_pass = if let Some(ref password) = data.password {
            Some(
                Employee::hash_password(password)
                    .map_err(|e| RepoError::Database(format!("Failed to hash password: {}", e)))?,
            )
        } else {
            None
        };

        let update_doc = UpdateDoc {
            username: data.username,
            hash_pass,
            role: data.role,
            is_active: data.is_active,
        };

        let thing = make_thing(TABLE, pure_id);
        self.base
            .db()
            .query("UPDATE $thing MERGE $data")
            .bind(("thing", thing))
            .bind(("data", update_doc))
            .await?;

        self.find_by_id(pure_id)
            .await?
            .map(|e| e.into())
            .ok_or_else(|| RepoError::NotFound(format!("Employee {} not found", id)))
    }

    /// Hard delete an employee
    pub async fn delete(&self, id: &str) -> RepoResult<bool> {
        let pure_id = strip_table_prefix(TABLE, id);
        let existing = self
            .find_by_id(pure_id)
            .await?
            .ok_or_else(|| RepoError::NotFound(format!("Employee {} not found", id)))?;

        // Prevent deleting system users
        if existing.is_system {
            return Err(RepoError::Validation(
                "Cannot delete system user".to_string(),
            ));
        }

        let thing = make_thing(TABLE, pure_id);
        self.base
            .db()
            .query("DELETE $thing")
            .bind(("thing", thing))
            .await?;
        Ok(true)
    }
}
