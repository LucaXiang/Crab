//! Dining Table Repository

use super::{make_thing, strip_table_prefix, BaseRepository, RepoError, RepoResult};
use crate::db::models::{DiningTable, DiningTableCreate, DiningTableUpdate};
use surrealdb::engine::local::Db;
use surrealdb::Surreal;

const TABLE: &str = "dining_table";

#[derive(Clone)]
pub struct DiningTableRepository {
    base: BaseRepository,
}

impl DiningTableRepository {
    pub fn new(db: Surreal<Db>) -> Self {
        Self {
            base: BaseRepository::new(db),
        }
    }

    /// Find all active dining tables
    pub async fn find_all(&self) -> RepoResult<Vec<DiningTable>> {
        let tables: Vec<DiningTable> = self
            .base
            .db()
            .query("SELECT * FROM dining_table WHERE is_active = true ORDER BY name FETCH zone")
            .await?
            .take(0)?;
        Ok(tables)
    }

    /// Find all tables in a zone
    pub async fn find_by_zone(&self, zone_id: &str) -> RepoResult<Vec<DiningTable>> {
        let zone_thing = make_thing("zone", zone_id);
        let tables: Vec<DiningTable> = self
            .base
            .db()
            .query("SELECT * FROM dining_table WHERE zone = $zone AND is_active = true ORDER BY name")
            .bind(("zone", zone_thing))
            .await?
            .take(0)?;
        Ok(tables)
    }

    /// Find table by id
    pub async fn find_by_id(&self, id: &str) -> RepoResult<Option<DiningTable>> {
        let pure_id = strip_table_prefix(TABLE, id);
        let table: Option<DiningTable> = self.base.db().select((TABLE, pure_id)).await?;
        Ok(table)
    }

    /// Find table by id with zone fetched
    pub async fn find_by_id_with_zone(&self, id: &str) -> RepoResult<Option<DiningTable>> {
        let table_thing = make_thing(TABLE, id);
        let mut result = self
            .base
            .db()
            .query("SELECT * FROM dining_table WHERE id = $id FETCH zone")
            .bind(("id", table_thing))
            .await?;
        let tables: Vec<DiningTable> = result.take(0)?;
        Ok(tables.into_iter().next())
    }

    /// Find table by name in zone
    pub async fn find_by_name_in_zone(&self, zone_id: &str, name: &str) -> RepoResult<Option<DiningTable>> {
        let zone_thing = make_thing("zone", zone_id);
        let name_owned = name.to_string();
        let mut result = self
            .base
            .db()
            .query("SELECT * FROM dining_table WHERE zone = $zone AND name = $name LIMIT 1")
            .bind(("zone", zone_thing))
            .bind(("name", name_owned))
            .await?;
        let tables: Vec<DiningTable> = result.take(0)?;
        Ok(tables.into_iter().next())
    }

    /// Create a new dining table
    pub async fn create(&self, data: DiningTableCreate) -> RepoResult<DiningTable> {
        // Check duplicate name in same zone
        let zone_id = data.zone.id.to_string();
        if self.find_by_name_in_zone(&zone_id, &data.name).await?.is_some() {
            return Err(RepoError::Duplicate(format!(
                "Table '{}' already exists in this zone",
                data.name
            )));
        }

        let table = DiningTable {
            id: None,
            name: data.name,
            zone: data.zone,
            capacity: data.capacity.unwrap_or(4),
            is_active: true,
        };

        let created: Option<DiningTable> = self.base.db().create(TABLE).content(table).await?;
        created.ok_or_else(|| RepoError::Database("Failed to create dining table".to_string()))
    }

    /// Update a dining table
    pub async fn update(&self, id: &str, data: DiningTableUpdate) -> RepoResult<DiningTable> {
        let pure_id = strip_table_prefix(TABLE, id);
        let existing = self
            .find_by_id(pure_id)
            .await?
            .ok_or_else(|| RepoError::NotFound(format!("Dining table {} not found", id)))?;

        // Check duplicate name in zone if changing name or zone
        let check_zone = data.zone.as_ref().unwrap_or(&existing.zone);
        let check_name = data.name.as_ref().unwrap_or(&existing.name);

        if data.name.is_some() || data.zone.is_some() {
            let zone_id = check_zone.id.to_string();
            if let Some(found) = self.find_by_name_in_zone(&zone_id, check_name).await?
                && found.id != existing.id
            {
                return Err(RepoError::Duplicate(format!(
                    "Table '{}' already exists in this zone",
                    check_name
                )));
            }
        }

        let thing = make_thing(TABLE, pure_id);
        self.base
            .db()
            .query("UPDATE $thing MERGE $data")
            .bind(("thing", thing))
            .bind(("data", data))
            .await?;

        self.find_by_id(pure_id)
            .await?
            .ok_or_else(|| RepoError::NotFound(format!("Dining table {} not found", id)))
    }

    /// Hard delete a dining table
    pub async fn delete(&self, id: &str) -> RepoResult<bool> {
        let pure_id = strip_table_prefix(TABLE, id);
        let thing = make_thing(TABLE, pure_id);
        self.base
            .db()
            .query("DELETE $thing")
            .bind(("thing", thing))
            .await?;
        Ok(true)
    }
}
