//! Zone Repository

use super::{BaseRepository, RepoError, RepoResult};
use crate::db::models::{Zone, ZoneCreate, ZoneUpdate};
use surrealdb::engine::local::Db;
use surrealdb::{RecordId, Surreal};

const TABLE: &str = "zone";

#[derive(Clone)]
pub struct ZoneRepository {
    base: BaseRepository,
}

impl ZoneRepository {
    pub fn new(db: Surreal<Db>) -> Self {
        Self {
            base: BaseRepository::new(db),
        }
    }

    /// Find all active zones
    pub async fn find_all(&self) -> RepoResult<Vec<Zone>> {
        let zones: Vec<Zone> = self
            .base
            .db()
            .query("SELECT * FROM zone WHERE is_active = true ORDER BY name")
            .await?
            .take(0)?;
        Ok(zones)
    }

    /// Find zone by id
    pub async fn find_by_id(&self, id: &str) -> RepoResult<Option<Zone>> {
        let thing: RecordId = id
            .parse()
            .map_err(|_| RepoError::Validation(format!("Invalid ID: {}", id)))?;
        let zone: Option<Zone> = self.base.db().select(thing).await?;
        Ok(zone)
    }

    /// Find zone by name
    pub async fn find_by_name(&self, name: &str) -> RepoResult<Option<Zone>> {
        let name_owned = name.to_string();
        let mut result = self
            .base
            .db()
            .query("SELECT * FROM zone WHERE name = $name LIMIT 1")
            .bind(("name", name_owned))
            .await?;
        let zones: Vec<Zone> = result.take(0)?;
        Ok(zones.into_iter().next())
    }

    /// Create a new zone
    pub async fn create(&self, data: ZoneCreate) -> RepoResult<Zone> {
        // Check duplicate name
        if self.find_by_name(&data.name).await?.is_some() {
            return Err(RepoError::Duplicate(format!(
                "Zone '{}' already exists",
                data.name
            )));
        }

        let zone = Zone {
            id: None,
            name: data.name,
            description: data.description,
            is_active: true,
        };

        let created: Option<Zone> = self.base.db().create(TABLE).content(zone).await?;
        created.ok_or_else(|| RepoError::Database("Failed to create zone".to_string()))
    }

    /// Update a zone
    pub async fn update(&self, id: &str, data: ZoneUpdate) -> RepoResult<Zone> {
        let thing: RecordId = id
            .parse()
            .map_err(|_| RepoError::Validation(format!("Invalid ID: {}", id)))?;
        let existing = self
            .find_by_id(id)
            .await?
            .ok_or_else(|| RepoError::NotFound(format!("Zone {} not found", id)))?;

        // Check duplicate name if changing
        if let Some(ref new_name) = data.name
            && new_name != &existing.name
            && self.find_by_name(new_name).await?.is_some()
        {
            return Err(RepoError::Duplicate(format!(
                "Zone '{}' already exists",
                new_name
            )));
        }

        let mut result = self.base
            .db()
            .query("UPDATE $thing MERGE $data RETURN AFTER")
            .bind(("thing", thing))
            .bind(("data", data))
            .await?;

        result.take::<Option<Zone>>(0)?
            .ok_or_else(|| RepoError::NotFound(format!("Zone {} not found", id)))
    }

    /// Hard delete a zone (check for tables first)
    pub async fn delete(&self, id: &str) -> RepoResult<bool> {
        let thing: RecordId = id
            .parse()
            .map_err(|_| RepoError::Validation(format!("Invalid ID: {}", id)))?;

        // Check if zone has dining tables
        let mut result = self
            .base
            .db()
            .query("SELECT count() FROM dining_table WHERE zone = $zone AND is_active = true GROUP ALL")
            .bind(("zone", thing.clone()))
            .await?;
        let count: Option<i64> = result.take((0, "count"))?;

        if count.unwrap_or(0) > 0 {
            return Err(RepoError::Validation(
                "Cannot delete zone with active tables".to_string(),
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
