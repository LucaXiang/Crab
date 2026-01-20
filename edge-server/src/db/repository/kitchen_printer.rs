//! Kitchen Printer Repository

use super::{BaseRepository, RepoError, RepoResult};
use crate::db::models::{KitchenPrinter, KitchenPrinterCreate, KitchenPrinterUpdate};
use surrealdb::Surreal;
use surrealdb::engine::local::Db;

const TABLE: &str = "kitchen_printer";

#[derive(Clone)]
pub struct KitchenPrinterRepository {
    base: BaseRepository,
}

impl KitchenPrinterRepository {
    pub fn new(db: Surreal<Db>) -> Self {
        Self {
            base: BaseRepository::new(db),
        }
    }

    /// Find all active kitchen printers
    pub async fn find_all(&self) -> RepoResult<Vec<KitchenPrinter>> {
        let printers: Vec<KitchenPrinter> = self
            .base
            .db()
            .query("SELECT * FROM kitchen_printer WHERE is_active = true ORDER BY name")
            .await?
            .take(0)?;
        Ok(printers)
    }

    /// Find printer by id
    pub async fn find_by_id(&self, id: &str) -> RepoResult<Option<KitchenPrinter>> {
        let printer: Option<KitchenPrinter> = self.base.db().select((TABLE, id)).await?;
        Ok(printer)
    }

    /// Find printer by name
    pub async fn find_by_name(&self, name: &str) -> RepoResult<Option<KitchenPrinter>> {
        let name_owned = name.to_string();
        let mut result = self
            .base
            .db()
            .query("SELECT * FROM kitchen_printer WHERE name = $name LIMIT 1")
            .bind(("name", name_owned))
            .await?;
        let printers: Vec<KitchenPrinter> = result.take(0)?;
        Ok(printers.into_iter().next())
    }

    /// Create a new kitchen printer
    pub async fn create(&self, data: KitchenPrinterCreate) -> RepoResult<KitchenPrinter> {
        // Check duplicate name
        if self.find_by_name(&data.name).await?.is_some() {
            return Err(RepoError::Duplicate(format!(
                "Kitchen printer '{}' already exists",
                data.name
            )));
        }

        let printer = KitchenPrinter {
            id: None,
            name: data.name,
            printer_name: data.printer_name,
            description: data.description,
            is_active: true,
        };

        let created: Option<KitchenPrinter> = self.base.db().create(TABLE).content(printer).await?;
        created.ok_or_else(|| RepoError::Database("Failed to create kitchen printer".to_string()))
    }

    /// Update a kitchen printer
    pub async fn update(&self, id: &str, data: KitchenPrinterUpdate) -> RepoResult<KitchenPrinter> {
        let existing = self
            .find_by_id(id)
            .await?
            .ok_or_else(|| RepoError::NotFound(format!("Kitchen printer {} not found", id)))?;

        // Check duplicate name if changing
        if let Some(ref new_name) = data.name
            && new_name != &existing.name
            && self.find_by_name(new_name).await?.is_some()
        {
            return Err(RepoError::Duplicate(format!(
                "Kitchen printer '{}' already exists",
                new_name
            )));
        }

        let updated: Option<KitchenPrinter> =
            self.base.db().update((TABLE, id)).merge(data).await?;
        updated.ok_or_else(|| RepoError::NotFound(format!("Kitchen printer {} not found", id)))
    }

    /// Soft delete a kitchen printer
    pub async fn delete(&self, id: &str) -> RepoResult<bool> {
        let result: Option<KitchenPrinter> = self
            .base
            .db()
            .update((TABLE, id))
            .merge(KitchenPrinterUpdate {
                name: None,
                printer_name: None,
                description: None,
                is_active: Some(false),
            })
            .await?;
        Ok(result.is_some())
    }
}
