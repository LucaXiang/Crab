//! Label Template Repository

use super::{BaseRepository, ImageRefRepository, RepoError, RepoResult};
use crate::db::models::{ImageRefEntityType, LabelField, LabelTemplate, LabelTemplateCreate, LabelTemplateUpdate};
use crate::services::ImageCleanupService;
use std::collections::HashSet;
use surrealdb::engine::local::Db;
use surrealdb::{RecordId, Surreal};

#[derive(Clone)]
pub struct LabelTemplateRepository {
    base: BaseRepository,
    image_ref_repo: ImageRefRepository,
    image_cleanup: ImageCleanupService,
}

impl LabelTemplateRepository {
    pub fn new(db: Surreal<Db>, images_dir: std::path::PathBuf) -> Self {
        Self {
            image_ref_repo: ImageRefRepository::new(db.clone()),
            image_cleanup: ImageCleanupService::new(images_dir),
            base: BaseRepository::new(db),
        }
    }

    /// List all label templates
    pub async fn list(&self) -> RepoResult<Vec<LabelTemplate>> {
        let templates: Vec<LabelTemplate> = self
            .base
            .db()
            .query("SELECT * FROM label_template WHERE is_active = true ORDER BY name")
            .await?
            .take(0)?;
        Ok(templates)
    }

    /// List all label templates (including inactive)
    pub async fn list_all(&self) -> RepoResult<Vec<LabelTemplate>> {
        let templates: Vec<LabelTemplate> = self
            .base
            .db()
            .query("SELECT * FROM label_template ORDER BY name")
            .await?
            .take(0)?;
        Ok(templates)
    }

    /// Get a label template by ID
    pub async fn get(&self, id: &RecordId) -> RepoResult<Option<LabelTemplate>> {
        let template: Option<LabelTemplate> = self.base.db().select(id.clone()).await?;
        Ok(template)
    }

    /// Get the default label template
    pub async fn get_default(&self) -> RepoResult<Option<LabelTemplate>> {
        let templates: Vec<LabelTemplate> = self
            .base
            .db()
            .query("SELECT * FROM label_template WHERE is_default = true AND is_active = true LIMIT 1")
            .await?
            .take(0)?;
        Ok(templates.into_iter().next())
    }

    /// Create a new label template
    pub async fn create(&self, data: LabelTemplateCreate) -> RepoResult<LabelTemplate> {
        // If this is set as default, unset other defaults first
        if data.is_default {
            self.base
                .db()
                .query("UPDATE label_template SET is_default = false WHERE is_default = true")
                .await?;
        }

        // Create the template with timestamps - let SurrealDB generate the ID
        let template: Option<LabelTemplate> = self
            .base
            .db()
            .query(
                "CREATE label_template CONTENT {
                    name: $name,
                    description: $description,
                    width: $width,
                    height: $height,
                    fields: $fields,
                    is_default: $is_default,
                    is_active: $is_active,
                    padding_mm_x: $padding_mm_x,
                    padding_mm_y: $padding_mm_y,
                    render_dpi: $render_dpi,
                    test_data: $test_data,
                    created_at: $now,
                    updated_at: $now
                }",
            )
            .bind(("name", data.name.clone()))
            .bind(("description", data.description.clone()))
            .bind(("width", data.width))
            .bind(("height", data.height))
            .bind(("fields", data.fields.clone()))
            .bind(("is_default", data.is_default))
            .bind(("is_active", data.is_active))
            .bind(("padding_mm_x", data.padding_mm_x))
            .bind(("padding_mm_y", data.padding_mm_y))
            .bind(("render_dpi", data.render_dpi))
            .bind(("test_data", data.test_data.clone()))
            .bind(("now", shared::util::now_millis()))
            .await?
            .take(0)?;

        let template =
            template.ok_or_else(|| RepoError::Database("Failed to create label template".to_string()))?;

        // Sync image references
        if let Some(id) = &template.id {
            let image_hashes = Self::extract_image_hashes(&template.fields);
            let _ = self
                .image_ref_repo
                .sync_refs(ImageRefEntityType::LabelTemplate, &id.to_string(), image_hashes)
                .await;
        }

        Ok(template)
    }

    /// Update a label template
    pub async fn update(&self, id: &RecordId, data: LabelTemplateUpdate) -> RepoResult<LabelTemplate> {
        // If setting as default, unset other defaults first
        if data.is_default == Some(true) {
            self.base
                .db()
                .query("UPDATE label_template SET is_default = false WHERE is_default = true AND id != $id")
                .bind(("id", id.clone()))
                .await?;
        }

        // Update timestamp
        let _ = self
            .base
            .db()
            .query("UPDATE $id SET updated_at = $now")
            .bind(("id", id.clone()))
            .bind(("now", shared::util::now_millis()))
            .await?;

        // Merge update data
        let updated: Option<LabelTemplate> = self.base.db().update(id.clone()).merge(data).await?;
        let updated =
            updated.ok_or_else(|| RepoError::NotFound(format!("Label template {} not found", id)))?;

        // Sync image references and cleanup orphans
        let image_hashes = Self::extract_image_hashes(&updated.fields);
        let removed_hashes = self
            .image_ref_repo
            .sync_refs(ImageRefEntityType::LabelTemplate, &id.to_string(), image_hashes)
            .await
            .unwrap_or_default();

        // Cleanup orphan images
        if !removed_hashes.is_empty() {
            let orphans = self
                .image_ref_repo
                .find_orphan_hashes(&removed_hashes)
                .await
                .unwrap_or_default();
            self.image_cleanup.cleanup_orphan_images(&orphans).await;
        }

        Ok(updated)
    }

    /// Delete a label template (soft delete by setting is_active = false)
    pub async fn delete(&self, id: &RecordId) -> RepoResult<bool> {
        // Get image references before soft delete
        let image_hashes = self
            .image_ref_repo
            .delete_entity_refs(ImageRefEntityType::LabelTemplate, &id.to_string())
            .await
            .unwrap_or_default();

        let _: Option<LabelTemplate> = self
            .base
            .db()
            .update(id.clone())
            .merge(LabelTemplateUpdate {
                is_active: Some(false),
                ..Default::default()
            })
            .await?;

        // Cleanup orphan images
        if !image_hashes.is_empty() {
            let orphans = self
                .image_ref_repo
                .find_orphan_hashes(&image_hashes)
                .await
                .unwrap_or_default();
            self.image_cleanup.cleanup_orphan_images(&orphans).await;
        }

        Ok(true)
    }

    /// Hard delete a label template
    pub async fn hard_delete(&self, id: &RecordId) -> RepoResult<bool> {
        // Get image references before deleting
        let image_hashes = self
            .image_ref_repo
            .delete_entity_refs(ImageRefEntityType::LabelTemplate, &id.to_string())
            .await
            .unwrap_or_default();

        let deleted: Option<LabelTemplate> = self.base.db().delete(id.clone()).await?;

        // Cleanup orphan images
        if !image_hashes.is_empty() {
            let orphans = self
                .image_ref_repo
                .find_orphan_hashes(&image_hashes)
                .await
                .unwrap_or_default();
            self.image_cleanup.cleanup_orphan_images(&orphans).await;
        }

        Ok(deleted.is_some())
    }

    /// Extract image hashes from label template fields
    ///
    /// Returns hashes from fields where source_type == "image" and template is not empty
    fn extract_image_hashes(fields: &[LabelField]) -> HashSet<String> {
        fields
            .iter()
            .filter(|f| f.source_type.as_deref() == Some("image"))
            .filter_map(|f| f.template.as_ref())
            .filter(|t| !t.is_empty())
            .cloned()
            .collect()
    }
}
