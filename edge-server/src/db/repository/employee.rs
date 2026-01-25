//! Employee Repository

use super::{BaseRepository, RepoError, RepoResult};
use crate::db::models::{Employee, EmployeeCreate, EmployeeUpdate};
use surrealdb::engine::local::Db;
use surrealdb::{RecordId, Surreal};

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
    pub async fn find_all(&self) -> RepoResult<Vec<Employee>> {
        let employees: Vec<Employee> = self
            .base
            .db()
            .query("SELECT * FROM employee WHERE is_active = true AND is_system = false ORDER BY username")
            .await?
            .take(0)?;
        Ok(employees)
    }

    /// Find all employees including inactive (excluding system users)
    pub async fn find_all_with_inactive(&self) -> RepoResult<Vec<Employee>> {
        let employees: Vec<Employee> = self
            .base
            .db()
            .query("SELECT * FROM employee WHERE is_system = false ORDER BY username")
            .await?
            .take(0)?;
        Ok(employees)
    }

    /// Find employee by id
    pub async fn find_by_id(&self, id: &str) -> RepoResult<Option<Employee>> {
        let thing: RecordId = id
            .parse()
            .map_err(|_| RepoError::Validation(format!("Invalid ID: {}", id)))?;
        let emp: Option<Employee> = self.base.db().select(thing).await?;
        Ok(emp)
    }

    /// Find employee by id (returns Employee without password)
    pub async fn find_by_id_safe(&self, id: &str) -> RepoResult<Option<Employee>> {
        let thing: RecordId = id
            .parse()
            .map_err(|_| RepoError::Validation(format!("Invalid ID: {}", id)))?;
        let emp: Option<Employee> = self.base.db().select(thing).await?;
        Ok(emp)
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
    pub async fn create(&self, data: EmployeeCreate) -> RepoResult<Employee> {
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

        let display_name = data.display_name.unwrap_or_else(|| data.username.clone());

        // Internal struct without serde_helpers to preserve native RecordId for SurrealDB
        #[derive(serde::Serialize)]
        struct InternalEmployee {
            username: String,
            #[serde(rename = "employee_name")]
            display_name: String,
            hash_pass: String,
            role: RecordId,
            is_system: bool,
            is_active: bool,
        }

        let employee = InternalEmployee {
            username: data.username,
            display_name,
            hash_pass,
            role: data.role,
            is_system: false,
            is_active: true,
        };

        let created: Option<Employee> = self.base.db().create(TABLE).content(employee).await?;
        created.ok_or_else(|| RepoError::Database("Failed to create employee".to_string()))
    }

    /// Update an employee
    pub async fn update(&self, id: &str, data: EmployeeUpdate) -> RepoResult<Employee> {
        let thing: RecordId = id
            .parse()
            .map_err(|_| RepoError::Validation(format!("Invalid ID: {}", id)))?;
        let existing = self
            .find_by_id(id)
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
            #[serde(skip_serializing_if = "Option::is_none", rename = "employee_name")]
            display_name: Option<String>,
            #[serde(skip_serializing_if = "Option::is_none")]
            hash_pass: Option<String>,
            #[serde(skip_serializing_if = "Option::is_none")]
            role: Option<RecordId>,
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
            display_name: data.display_name,
            hash_pass,
            role: data.role,
            is_active: data.is_active,
        };

        let mut result = self.base
            .db()
            .query("UPDATE $thing MERGE $data RETURN AFTER")
            .bind(("thing", thing))
            .bind(("data", update_doc))
            .await?;

        result.take::<Option<Employee>>(0)?
            .ok_or_else(|| RepoError::NotFound(format!("Employee {} not found", id)))
    }

    /// Hard delete an employee
    pub async fn delete(&self, id: &str) -> RepoResult<bool> {
        let thing: RecordId = id
            .parse()
            .map_err(|_| RepoError::Validation(format!("Invalid ID: {}", id)))?;
        let existing = self
            .find_by_id(id)
            .await?
            .ok_or_else(|| RepoError::NotFound(format!("Employee {} not found", id)))?;

        // Prevent deleting system users
        if existing.is_system {
            return Err(RepoError::Validation(
                "Cannot delete system user".to_string(),
            ));
        }

        self.base
            .db()
            .query("DELETE $thing")
            .bind(("thing", thing))
            .await?;
        Ok(true)
    }
}
