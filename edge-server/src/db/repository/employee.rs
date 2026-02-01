//! Employee Repository

use super::{BaseRepository, RepoError, RepoResult};
use crate::db::models::{Employee, EmployeeCreate, EmployeeUpdate};
use surrealdb::engine::local::Db;
use surrealdb::{RecordId, Surreal};

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

    /// Find all active employees
    pub async fn find_all(&self) -> RepoResult<Vec<Employee>> {
        let employees: Vec<Employee> = self
            .base
            .db()
            .query("SELECT * FROM employee WHERE is_active = true ORDER BY username")
            .await?
            .take(0)?;
        Ok(employees)
    }

    /// Find all employees including inactive
    pub async fn find_all_with_inactive(&self) -> RepoResult<Vec<Employee>> {
        let employees: Vec<Employee> = self
            .base
            .db()
            .query("SELECT * FROM employee ORDER BY username")
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

        let mut result = self
            .base
            .db()
            .query(
                r#"CREATE employee SET
                    username = $username,
                    display_name = $display_name,
                    hash_pass = $hash_pass,
                    role = $role,
                    is_system = false,
                    is_active = true
                RETURN AFTER"#,
            )
            .bind(("username", data.username))
            .bind(("display_name", display_name))
            .bind(("hash_pass", hash_pass))
            .bind(("role", data.role))
            .await?;

        let created: Option<Employee> = result.take(0)?;
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

        // System users can only change password
        if existing.is_system
            && (data.username.is_some() || data.role.is_some() || data.is_active.is_some() || data.display_name.is_some())
        {
            return Err(RepoError::Validation(
                "System user can only change password".to_string(),
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

        let hash_pass = if let Some(ref password) = data.password {
            Some(
                Employee::hash_password(password)
                    .map_err(|e| RepoError::Database(format!("Failed to hash password: {}", e)))?,
            )
        } else {
            None
        };

        let mut result = self.base
            .db()
            .query(
                r#"UPDATE $thing SET
                    username = $username OR username,
                    display_name = $display_name OR display_name,
                    hash_pass = $hash_pass OR hash_pass,
                    role = IF $has_role THEN $role ELSE role END,
                    is_active = IF $has_is_active THEN $is_active ELSE is_active END
                RETURN AFTER"#,
            )
            .bind(("thing", thing))
            .bind(("username", data.username))
            .bind(("display_name", data.display_name))
            .bind(("hash_pass", hash_pass))
            .bind(("has_role", data.role.is_some()))
            .bind(("role", data.role))
            .bind(("has_is_active", data.is_active.is_some()))
            .bind(("is_active", data.is_active))
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
