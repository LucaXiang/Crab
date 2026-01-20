//! Category Repository

use super::{BaseRepository, RepoError, RepoResult, make_thing};
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

    /// Find all categories with kitchen_printer fetched
    pub async fn find_all_with_printer(&self) -> RepoResult<Vec<Category>> {
        let categories: Vec<Category> = self
            .base
            .db()
            .query("SELECT * FROM category WHERE is_active = true ORDER BY sort_order FETCH kitchen_printer")
            .await?
            .take(0)?;
        Ok(categories)
    }

    /// Find category by id
    pub async fn find_by_id(&self, id: &str) -> RepoResult<Option<Category>> {
        let category: Option<Category> = self.base.db().select((TABLE, id)).await?;
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

        let kitchen_printer = data
            .kitchen_printer
            .map(|id| make_thing("kitchen_printer", &id));

        let category = Category {
            id: None,
            name: data.name,
            sort_order: data.sort_order.unwrap_or(0),
            kitchen_printer,
            is_kitchen_print_enabled: data.is_kitchen_print_enabled.unwrap_or(true),
            is_label_print_enabled: data.is_label_print_enabled.unwrap_or(true),
            is_active: true,
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
            kitchen_printer: Option<Thing>,
            #[serde(skip_serializing_if = "Option::is_none")]
            is_kitchen_print_enabled: Option<bool>,
            #[serde(skip_serializing_if = "Option::is_none")]
            is_label_print_enabled: Option<bool>,
            #[serde(skip_serializing_if = "Option::is_none")]
            is_active: Option<bool>,
        }

        let update_data = CategoryUpdateDb {
            name: data.name,
            sort_order: data.sort_order,
            kitchen_printer: data
                .kitchen_printer
                .map(|id| make_thing("kitchen_printer", &id)),
            is_kitchen_print_enabled: data.is_kitchen_print_enabled,
            is_label_print_enabled: data.is_label_print_enabled,
            is_active: data.is_active,
        };

        let updated: Option<Category> = self
            .base
            .db()
            .update((TABLE, id))
            .merge(update_data)
            .await?;

        updated.ok_or_else(|| RepoError::NotFound(format!("Category {} not found", id)))
    }

    /// Soft delete a category
    pub async fn delete(&self, id: &str) -> RepoResult<bool> {
        // Check if category has products
        let cat_thing = make_thing(TABLE, id);
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

        let result: Option<Category> = self
            .base
            .db()
            .update((TABLE, id))
            .merge(CategoryUpdate {
                name: None,
                sort_order: None,
                kitchen_printer: None,
                is_kitchen_print_enabled: None,
                is_label_print_enabled: None,
                is_active: Some(false),
            })
            .await?;
        Ok(result.is_some())
    }
}
