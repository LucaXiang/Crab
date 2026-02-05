//! Print Destination Repository

use super::{BaseRepository, RepoError, RepoResult};
use crate::db::models::{PrintDestination, PrintDestinationCreate, PrintDestinationUpdate};
use surrealdb::engine::local::Db;
use surrealdb::{RecordId, Surreal};

const TABLE: &str = "print_destination";

#[derive(Clone)]
pub struct PrintDestinationRepository {
    base: BaseRepository,
}

impl PrintDestinationRepository {
    pub fn new(db: Surreal<Db>) -> Self {
        Self {
            base: BaseRepository::new(db),
        }
    }

    /// Find all active print destinations
    pub async fn find_all(&self) -> RepoResult<Vec<PrintDestination>> {
        let items: Vec<PrintDestination> = self
            .base
            .db()
            .query("SELECT * FROM print_destination WHERE is_active = true ORDER BY name")
            .await?
            .take(0)?;
        Ok(items)
    }

    /// Find all print destinations (including inactive)
    pub async fn find_all_with_inactive(&self) -> RepoResult<Vec<PrintDestination>> {
        let items: Vec<PrintDestination> = self
            .base
            .db()
            .query("SELECT * FROM print_destination ORDER BY name")
            .await?
            .take(0)?;
        Ok(items)
    }

    /// Find print destination by id
    pub async fn find_by_id(&self, id: &str) -> RepoResult<Option<PrintDestination>> {
        let thing: RecordId = id
            .parse()
            .map_err(|_| RepoError::Validation(format!("Invalid ID: {}", id)))?;
        let item: Option<PrintDestination> = self.base.db().select(thing).await?;
        Ok(item)
    }

    /// Find print destination by name
    pub async fn find_by_name(&self, name: &str) -> RepoResult<Option<PrintDestination>> {
        let name_owned = name.to_string();
        let mut result = self
            .base
            .db()
            .query("SELECT * FROM print_destination WHERE name = $name LIMIT 1")
            .bind(("name", name_owned))
            .await?;
        let items: Vec<PrintDestination> = result.take(0)?;
        Ok(items.into_iter().next())
    }

    /// Create a new print destination
    pub async fn create(&self, data: PrintDestinationCreate) -> RepoResult<PrintDestination> {
        let item = PrintDestination {
            id: None,
            name: data.name,
            description: data.description,
            printers: data.printers,
            is_active: data.is_active.unwrap_or(true),
        };

        let created: Option<PrintDestination> = self.base.db().create(TABLE).content(item).await?;
        created.ok_or_else(|| RepoError::Database("Failed to create print destination".to_string()))
    }

    /// Update a print destination
    pub async fn update(
        &self,
        id: &str,
        data: PrintDestinationUpdate,
    ) -> RepoResult<PrintDestination> {
        let thing: RecordId = id
            .parse()
            .map_err(|_| RepoError::Validation(format!("Invalid ID: {}", id)))?;

        let mut result = self.base
            .db()
            .query("UPDATE $thing MERGE $data RETURN AFTER")
            .bind(("thing", thing))
            .bind(("data", data))
            .await?;

        result.take::<Option<PrintDestination>>(0)?
            .ok_or_else(|| RepoError::NotFound(format!("Print destination {} not found", id)))
    }

    /// Hard delete a print destination
    pub async fn delete(&self, id: &str) -> RepoResult<bool> {
        let thing: RecordId = id
            .parse()
            .map_err(|_| RepoError::Validation(format!("Invalid ID: {}", id)))?;
        self.base
            .db()
            .query("DELETE $thing")
            .bind(("thing", thing))
            .await?;
        Ok(true)
    }
}
