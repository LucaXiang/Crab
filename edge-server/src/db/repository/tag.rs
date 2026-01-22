//! Tag Repository

use super::{BaseRepository, RepoError, RepoResult, make_thing, strip_table_prefix};
use crate::db::models::{Tag, TagCreate, TagUpdate};
use surrealdb::Surreal;
use surrealdb::engine::local::Db;

const TABLE: &str = "tag";

#[derive(Clone)]
pub struct TagRepository {
    base: BaseRepository,
}

impl TagRepository {
    pub fn new(db: Surreal<Db>) -> Self {
        Self {
            base: BaseRepository::new(db),
        }
    }

    /// Find all active tags ordered by display_order
    pub async fn find_all(&self) -> RepoResult<Vec<Tag>> {
        let tags: Vec<Tag> = self
            .base
            .db()
            .query("SELECT * FROM tag WHERE is_active = true ORDER BY display_order")
            .await?
            .take(0)?;
        Ok(tags)
    }

    /// Find all tags (including inactive)
    pub async fn find_all_with_inactive(&self) -> RepoResult<Vec<Tag>> {
        let tags: Vec<Tag> = self
            .base
            .db()
            .query("SELECT * FROM tag ORDER BY display_order")
            .await?
            .take(0)?;
        Ok(tags)
    }

    /// Find tag by id
    pub async fn find_by_id(&self, id: &str) -> RepoResult<Option<Tag>> {
        let pure_id = strip_table_prefix(TABLE, id);
        let tag: Option<Tag> = self.base.db().select((TABLE, pure_id)).await?;
        Ok(tag)
    }

    /// Find tag by name
    pub async fn find_by_name(&self, name: &str) -> RepoResult<Option<Tag>> {
        let name_owned = name.to_string();
        let mut result = self
            .base
            .db()
            .query("SELECT * FROM tag WHERE name = $name LIMIT 1")
            .bind(("name", name_owned))
            .await?;
        let tags: Vec<Tag> = result.take(0)?;
        Ok(tags.into_iter().next())
    }

    /// Create a new tag
    pub async fn create(&self, data: TagCreate) -> RepoResult<Tag> {
        // Check duplicate name
        if self.find_by_name(&data.name).await?.is_some() {
            return Err(RepoError::Duplicate(format!(
                "Tag '{}' already exists",
                data.name
            )));
        }

        let tag = Tag {
            id: None,
            name: data.name,
            color: data.color.unwrap_or_else(|| "#3B82F6".to_string()),
            display_order: data.display_order.unwrap_or(0),
            is_active: true,
            is_system: false,
        };

        let created: Option<Tag> = self.base.db().create(TABLE).content(tag).await?;
        created.ok_or_else(|| RepoError::Database("Failed to create tag".to_string()))
    }

    /// Update a tag
    pub async fn update(&self, id: &str, data: TagUpdate) -> RepoResult<Tag> {
        let pure_id = strip_table_prefix(TABLE, id);
        let existing = self
            .find_by_id(pure_id)
            .await?
            .ok_or_else(|| RepoError::NotFound(format!("Tag {} not found", id)))?;

        // Check duplicate name if changing
        if let Some(ref new_name) = data.name
            && new_name != &existing.name
            && self.find_by_name(new_name).await?.is_some()
        {
            return Err(RepoError::Duplicate(format!(
                "Tag '{}' already exists",
                new_name
            )));
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
            .ok_or_else(|| RepoError::NotFound(format!("Tag {} not found", id)))
    }

    /// Hard delete a tag
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
