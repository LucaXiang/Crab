//! Category Repository

use super::{BaseRepository, RepoError, RepoResult, make_thing, strip_table_prefix};
use crate::db::models::{Category, CategoryCreate, CategoryUpdate};
use serde::Serialize;
use surrealdb::Surreal;
use surrealdb::engine::local::Db;
use surrealdb::sql::Thing;

const TABLE: &str = "category";

#[derive(Clone)]
pub struct CategoryRepository {
    base: BaseRepository,
}

impl CategoryRepository {
    pub fn new(db: Surreal<Db>) -> Self {
        Self {
            base: BaseRepository::new(db),
        }
    }

    /// Find all active categories ordered by sort_order
    pub async fn find_all(&self) -> RepoResult<Vec<Category>> {
        let categories: Vec<Category> = self
            .base
            .db()
            .query("SELECT * FROM category WHERE is_active = true ORDER BY sort_order")
            .await?
            .take(0)?;
        Ok(categories)
    }

    /// Find all categories with print destinations fetched
    pub async fn find_all_with_destinations(&self) -> RepoResult<Vec<Category>> {
        let categories: Vec<Category> = self
            .base
            .db()
            .query("SELECT * FROM category WHERE is_active = true ORDER BY sort_order FETCH kitchen_print_destinations, label_print_destinations")
            .await?
            .take(0)?;
        Ok(categories)
    }

    /// Find category by id
    pub async fn find_by_id(&self, id: &str) -> RepoResult<Option<Category>> {
        // Extract pure id if it contains table prefix (e.g., "category:xxx" -> "xxx")
        let pure_id = strip_table_prefix(TABLE, id);
        let category: Option<Category> = self.base.db().select((TABLE, pure_id)).await?;
        Ok(category)
    }

    /// Find category by name
    pub async fn find_by_name(&self, name: &str) -> RepoResult<Option<Category>> {
        let name_owned = name.to_string();
        let mut result = self
            .base
            .db()
            .query("SELECT * FROM category WHERE name = $name LIMIT 1")
            .bind(("name", name_owned))
            .await?;
        let categories: Vec<Category> = result.take(0)?;
        Ok(categories.into_iter().next())
    }

    /// Create a new category
    pub async fn create(&self, data: CategoryCreate) -> RepoResult<Category> {
        // Check duplicate name
        if self.find_by_name(&data.name).await?.is_some() {
            return Err(RepoError::Duplicate(format!(
                "Category '{}' already exists",
                data.name
            )));
        }

        let kitchen_print_destinations: Vec<Thing> = data
            .kitchen_print_destinations
            .iter()
            .map(|id| make_thing("print_destination", id))
            .collect();

        let label_print_destinations: Vec<Thing> = data
            .label_print_destinations
            .iter()
            .map(|id| make_thing("print_destination", id))
            .collect();

        let tag_ids: Vec<Thing> = data
            .tag_ids
            .iter()
            .map(|id| make_thing("tag", id))
            .collect();

        let category = Category {
            id: None,
            name: data.name,
            sort_order: data.sort_order.unwrap_or(0),
            kitchen_print_destinations,
            label_print_destinations,
            is_kitchen_print_enabled: data.is_kitchen_print_enabled.unwrap_or(true),
            is_label_print_enabled: data.is_label_print_enabled.unwrap_or(true),
            is_active: true,
            is_virtual: data.is_virtual.unwrap_or(false),
            tag_ids,
            match_mode: data.match_mode.unwrap_or_else(|| "any".to_string()),
        };

        let created: Option<Category> = self.base.db().create(TABLE).content(category).await?;
        created.ok_or_else(|| RepoError::Database("Failed to create category".to_string()))
    }

    /// Update a category
    pub async fn update(&self, id: &str, data: CategoryUpdate) -> RepoResult<Category> {
        let existing = self
            .find_by_id(id)
            .await?
            .ok_or_else(|| RepoError::NotFound(format!("Category {} not found", id)))?;

        // Check duplicate name if changing
        if let Some(ref new_name) = data.name
            && new_name != &existing.name
            && self.find_by_name(new_name).await?.is_some()
        {
            return Err(RepoError::Duplicate(format!(
                "Category '{}' already exists",
                new_name
            )));
        }

        #[derive(Serialize)]
        struct CategoryUpdateDb {
            #[serde(skip_serializing_if = "Option::is_none")]
            name: Option<String>,
            #[serde(skip_serializing_if = "Option::is_none")]
            sort_order: Option<i32>,
            #[serde(skip_serializing_if = "Option::is_none")]
            kitchen_print_destinations: Option<Vec<Thing>>,
            #[serde(skip_serializing_if = "Option::is_none")]
            label_print_destinations: Option<Vec<Thing>>,
            #[serde(skip_serializing_if = "Option::is_none")]
            is_kitchen_print_enabled: Option<bool>,
            #[serde(skip_serializing_if = "Option::is_none")]
            is_label_print_enabled: Option<bool>,
            #[serde(skip_serializing_if = "Option::is_none")]
            is_active: Option<bool>,
            #[serde(skip_serializing_if = "Option::is_none")]
            is_virtual: Option<bool>,
            #[serde(skip_serializing_if = "Option::is_none")]
            tag_ids: Option<Vec<Thing>>,
            #[serde(skip_serializing_if = "Option::is_none")]
            match_mode: Option<String>,
        }

        let update_data = CategoryUpdateDb {
            name: data.name,
            sort_order: data.sort_order,
            kitchen_print_destinations: data.kitchen_print_destinations.map(|ids| {
                ids.iter()
                    .map(|id| make_thing("print_destination", id))
                    .collect()
            }),
            label_print_destinations: data.label_print_destinations.map(|ids| {
                ids.iter()
                    .map(|id| make_thing("print_destination", id))
                    .collect()
            }),
            is_kitchen_print_enabled: data.is_kitchen_print_enabled,
            is_label_print_enabled: data.is_label_print_enabled,
            is_active: data.is_active,
            is_virtual: data.is_virtual,
            tag_ids: data
                .tag_ids
                .map(|ids| ids.iter().map(|id| make_thing("tag", id)).collect()),
            match_mode: data.match_mode,
        };

        // Extract pure id if it contains table prefix
        let pure_id = strip_table_prefix(TABLE, id);

        // Update using raw query to avoid deserialization issues with null fields
        let thing = make_thing(TABLE, pure_id);
        self.base
            .db()
            .query("UPDATE $thing MERGE $data")
            .bind(("thing", thing.clone()))
            .bind(("data", update_data))
            .await?;

        // Fetch the updated record
        self.find_by_id(pure_id)
            .await?
            .ok_or_else(|| RepoError::NotFound(format!("Category {} not found", id)))
    }

    /// Hard delete a category
    pub async fn delete(&self, id: &str) -> RepoResult<bool> {
        // Extract pure id if it contains table prefix (e.g., "category:xxx" -> "xxx")
        let pure_id = strip_table_prefix(TABLE, id);

        // Check if category has products
        let cat_thing = make_thing(TABLE, pure_id);
        let mut result = self
            .base
            .db()
            .query(
                "SELECT count() FROM product WHERE category = $cat AND is_active = true GROUP ALL",
            )
            .bind(("cat", cat_thing))
            .await?;
        let count: Option<i64> = result.take((0, "count"))?;

        if count.unwrap_or(0) > 0 {
            return Err(RepoError::Validation(
                "Cannot delete category with active products".to_string(),
            ));
        }

        // Clean up has_attribute edges first
        let thing = make_thing(TABLE, pure_id);
        self.base
            .db()
            .query("DELETE has_attribute WHERE in = $category")
            .bind(("category", thing.clone()))
            .await?;

        // Hard delete
        self.base
            .db()
            .query("DELETE $thing")
            .bind(("thing", thing))
            .await?;

        Ok(true)
    }
}
