//! Role Repository

use super::{BaseRepository, RepoError, RepoResult};
use crate::db::models::{Role, RoleCreate, RoleUpdate};
use surrealdb::engine::local::Db;
use surrealdb::{RecordId, Surreal};

const TABLE: &str = "role";

#[derive(Clone)]
pub struct RoleRepository {
    base: BaseRepository,
}

impl RoleRepository {
    pub fn new(db: Surreal<Db>) -> Self {
        Self {
            base: BaseRepository::new(db),
        }
    }

    /// Find all active roles
    pub async fn find_all(&self) -> RepoResult<Vec<Role>> {
        let roles: Vec<Role> = self
            .base
            .db()
            .query("SELECT * FROM role WHERE is_active = true ORDER BY name")
            .await?
            .take(0)?;
        Ok(roles)
    }

    /// Find all roles including inactive
    pub async fn find_all_with_inactive(&self) -> RepoResult<Vec<Role>> {
        let roles: Vec<Role> = self
            .base
            .db()
            .query("SELECT * FROM role ORDER BY name")
            .await?
            .take(0)?;
        Ok(roles)
    }

    /// Find role by id
    pub async fn find_by_id(&self, id: &str) -> RepoResult<Option<Role>> {
        let thing: RecordId = id
            .parse()
            .map_err(|_| RepoError::Validation(format!("Invalid ID: {}", id)))?;
        let role: Option<Role> = self.base.db().select(thing).await?;
        Ok(role)
    }

    /// Find role by name
    pub async fn find_by_name(&self, name: &str) -> RepoResult<Option<Role>> {
        let name_owned = name.to_string();
        let mut result = self
            .base
            .db()
            .query("SELECT * FROM role WHERE name = $name LIMIT 1")
            .bind(("name", name_owned))
            .await?;
        let roles: Vec<Role> = result.take(0)?;
        Ok(roles.into_iter().next())
    }

    /// Create a new role
    pub async fn create(&self, data: RoleCreate) -> RepoResult<Role> {
        // Check duplicate name
        if self.find_by_name(&data.name).await?.is_some() {
            return Err(RepoError::Duplicate(format!(
                "Role '{}' already exists",
                data.name
            )));
        }

        let mut role = Role::new(data.name, data.permissions);
        if let Some(display_name) = data.display_name {
            role.display_name = display_name;
        }
        role.description = data.description;
        let created: Option<Role> = self.base.db().create(TABLE).content(role).await?;
        created.ok_or_else(|| RepoError::Database("Failed to create role".to_string()))
    }

    /// Update a role
    pub async fn update(&self, id: &str, data: RoleUpdate) -> RepoResult<Role> {
        let thing: RecordId = id
            .parse()
            .map_err(|_| RepoError::Validation(format!("Invalid ID: {}", id)))?;
        let existing = self
            .find_by_id(id)
            .await?
            .ok_or_else(|| RepoError::NotFound(format!("Role {} not found", id)))?;

        // Prevent modifying system roles
        if existing.is_system {
            return Err(RepoError::Validation(
                "Cannot modify system role".to_string(),
            ));
        }

        // Check duplicate name if changing
        if let Some(ref new_name) = data.name
            && new_name != &existing.name
            && self.find_by_name(new_name).await?.is_some()
        {
            return Err(RepoError::Duplicate(format!(
                "Role '{}' already exists",
                new_name
            )));
        }

        let mut result = self.base
            .db()
            .query("UPDATE $thing MERGE $data RETURN AFTER")
            .bind(("thing", thing))
            .bind(("data", data))
            .await?;

        result.take::<Option<Role>>(0)?
            .ok_or_else(|| RepoError::NotFound(format!("Role {} not found", id)))
    }

    /// Hard delete a role
    pub async fn delete(&self, id: &str) -> RepoResult<bool> {
        let thing: RecordId = id
            .parse()
            .map_err(|_| RepoError::Validation(format!("Invalid ID: {}", id)))?;
        let existing = self
            .find_by_id(id)
            .await?
            .ok_or_else(|| RepoError::NotFound(format!("Role {} not found", id)))?;

        // Prevent deleting system roles
        if existing.is_system {
            return Err(RepoError::Validation(
                "Cannot delete system role".to_string(),
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
