//! Label Template Repository

use super::{RepoError, RepoResult};
use shared::models::{
    ImageRefEntityType, LabelField, LabelFieldInput, LabelTemplate, LabelTemplateCreate,
    LabelTemplateUpdate,
};
use sqlx::SqlitePool;
use std::collections::HashSet;

pub async fn list(pool: &SqlitePool) -> RepoResult<Vec<LabelTemplate>> {
    let mut templates = sqlx::query_as::<_, LabelTemplate>(
        "SELECT id, name, description, width, height, padding, is_default, is_active, width_mm, height_mm, padding_mm_x, padding_mm_y, render_dpi, test_data, created_at, updated_at FROM label_template WHERE is_active = 1 ORDER BY name",
    )
    .fetch_all(pool)
    .await?;

    for t in &mut templates {
        t.fields = find_fields(pool, t.id).await?;
    }
    Ok(templates)
}

pub async fn list_all(pool: &SqlitePool) -> RepoResult<Vec<LabelTemplate>> {
    let mut templates = sqlx::query_as::<_, LabelTemplate>(
        "SELECT id, name, description, width, height, padding, is_default, is_active, width_mm, height_mm, padding_mm_x, padding_mm_y, render_dpi, test_data, created_at, updated_at FROM label_template ORDER BY name",
    )
    .fetch_all(pool)
    .await?;

    for t in &mut templates {
        t.fields = find_fields(pool, t.id).await?;
    }
    Ok(templates)
}

pub async fn get(pool: &SqlitePool, id: i64) -> RepoResult<Option<LabelTemplate>> {
    let mut template = sqlx::query_as::<_, LabelTemplate>(
        "SELECT id, name, description, width, height, padding, is_default, is_active, width_mm, height_mm, padding_mm_x, padding_mm_y, render_dpi, test_data, created_at, updated_at FROM label_template WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    if let Some(ref mut t) = template {
        t.fields = find_fields(pool, t.id).await?;
    }
    Ok(template)
}

pub async fn get_default(pool: &SqlitePool) -> RepoResult<Option<LabelTemplate>> {
    let mut template = sqlx::query_as::<_, LabelTemplate>(
        "SELECT id, name, description, width, height, padding, is_default, is_active, width_mm, height_mm, padding_mm_x, padding_mm_y, render_dpi, test_data, created_at, updated_at FROM label_template WHERE is_default = 1 AND is_active = 1 LIMIT 1",
    )
    .fetch_optional(pool)
    .await?;

    if let Some(ref mut t) = template {
        t.fields = find_fields(pool, t.id).await?;
    }
    Ok(template)
}

pub async fn create(pool: &SqlitePool, data: LabelTemplateCreate) -> RepoResult<LabelTemplate> {
    // If this is set as default, unset other defaults first
    if data.is_default {
        sqlx::query("UPDATE label_template SET is_default = 0 WHERE is_default = 1")
            .execute(pool)
            .await?;
    }

    let now = shared::util::now_millis();
    let id = sqlx::query_scalar::<_, i64>(
        "INSERT INTO label_template (name, description, width, height, padding, is_default, is_active, width_mm, height_mm, padding_mm_x, padding_mm_y, render_dpi, test_data, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?14) RETURNING id",
    )
    .bind(&data.name)
    .bind(&data.description)
    .bind(data.width)
    .bind(data.height)
    .bind(data.padding)
    .bind(data.is_default)
    .bind(data.is_active)
    .bind(data.width_mm)
    .bind(data.height_mm)
    .bind(data.padding_mm_x)
    .bind(data.padding_mm_y)
    .bind(data.render_dpi)
    .bind(&data.test_data)
    .bind(now)
    .fetch_one(pool)
    .await?;

    // Create fields
    for field in &data.fields {
        create_field(pool, id, field).await?;
    }

    // Sync image refs
    let template = get(pool, id)
        .await?
        .ok_or_else(|| RepoError::Database("Failed to create label template".into()))?;

    let image_hashes = extract_image_hashes(&template.fields);
    if !image_hashes.is_empty() {
        let _ = super::image_ref::sync_refs(
            pool,
            ImageRefEntityType::LabelTemplate,
            &id.to_string(),
            image_hashes,
        )
        .await;
    }

    Ok(template)
}

pub async fn update(
    pool: &SqlitePool,
    id: i64,
    data: LabelTemplateUpdate,
) -> RepoResult<LabelTemplate> {
    // If setting as default, unset other defaults first
    if data.is_default == Some(true) {
        sqlx::query("UPDATE label_template SET is_default = 0 WHERE is_default = 1 AND id != ?")
            .bind(id)
            .execute(pool)
            .await?;
    }

    let now = shared::util::now_millis();
    let rows = sqlx::query(
        "UPDATE label_template SET name = COALESCE(?1, name), description = COALESCE(?2, description), width = COALESCE(?3, width), height = COALESCE(?4, height), padding = COALESCE(?5, padding), is_default = COALESCE(?6, is_default), is_active = COALESCE(?7, is_active), width_mm = COALESCE(?8, width_mm), height_mm = COALESCE(?9, height_mm), padding_mm_x = COALESCE(?10, padding_mm_x), padding_mm_y = COALESCE(?11, padding_mm_y), render_dpi = COALESCE(?12, render_dpi), test_data = COALESCE(?13, test_data), updated_at = ?14 WHERE id = ?15",
    )
    .bind(&data.name)
    .bind(&data.description)
    .bind(data.width)
    .bind(data.height)
    .bind(data.padding)
    .bind(data.is_default)
    .bind(data.is_active)
    .bind(data.width_mm)
    .bind(data.height_mm)
    .bind(data.padding_mm_x)
    .bind(data.padding_mm_y)
    .bind(data.render_dpi)
    .bind(&data.test_data)
    .bind(now)
    .bind(id)
    .execute(pool)
    .await?;

    if rows.rows_affected() == 0 {
        return Err(RepoError::NotFound(format!(
            "Label template {id} not found"
        )));
    }

    // Replace fields if provided
    if let Some(fields) = &data.fields {
        sqlx::query("DELETE FROM label_field WHERE template_id = ?")
            .bind(id)
            .execute(pool)
            .await?;
        for field in fields {
            create_field(pool, id, field).await?;
        }
    }

    let updated = get(pool, id)
        .await?
        .ok_or_else(|| RepoError::NotFound(format!("Label template {id} not found")))?;

    // Sync image refs
    let image_hashes = extract_image_hashes(&updated.fields);
    let _ = super::image_ref::sync_refs(
        pool,
        ImageRefEntityType::LabelTemplate,
        &id.to_string(),
        image_hashes,
    )
    .await;

    Ok(updated)
}

pub async fn delete(pool: &SqlitePool, id: i64) -> RepoResult<bool> {
    // Clean up image refs
    let _ = super::image_ref::delete_entity_refs(
        pool,
        ImageRefEntityType::LabelTemplate,
        &id.to_string(),
    )
    .await;

    // Soft delete
    sqlx::query("UPDATE label_template SET is_active = 0 WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(true)
}

pub async fn hard_delete(pool: &SqlitePool, id: i64) -> RepoResult<bool> {
    let _ = super::image_ref::delete_entity_refs(
        pool,
        ImageRefEntityType::LabelTemplate,
        &id.to_string(),
    )
    .await;

    // Fields cascade via FK
    let result = sqlx::query("DELETE FROM label_template WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}

// ── Fields ──────────────────────────────────────────────────────────────

async fn find_fields(pool: &SqlitePool, template_id: i64) -> RepoResult<Vec<LabelField>> {
    let fields = sqlx::query_as::<_, LabelField>(
        "SELECT id, template_id, field_id, name, field_type, x, y, width, height, font_size, font_weight, font_family, color, rotate, alignment, data_source, format, visible, label, template, data_key, source_type, maintain_aspect_ratio, style, align, vertical_align, line_style FROM label_field WHERE template_id = ?",
    )
    .bind(template_id)
    .fetch_all(pool)
    .await?;
    Ok(fields)
}

async fn create_field(pool: &SqlitePool, template_id: i64, input: &LabelFieldInput) -> RepoResult<i64> {
    let id = sqlx::query_scalar::<_, i64>(
        "INSERT INTO label_field (template_id, field_id, name, field_type, x, y, width, height, font_size, font_weight, font_family, color, rotate, alignment, data_source, format, visible, label, template, data_key, source_type, maintain_aspect_ratio, style, align, vertical_align, line_style) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26) RETURNING id",
    )
    .bind(template_id)
    .bind(&input.field_id)
    .bind(&input.name)
    .bind(&input.field_type)
    .bind(input.x)
    .bind(input.y)
    .bind(input.width)
    .bind(input.height)
    .bind(input.font_size)
    .bind(&input.font_weight)
    .bind(&input.font_family)
    .bind(&input.color)
    .bind(input.rotate)
    .bind(&input.alignment)
    .bind(&input.data_source)
    .bind(&input.format)
    .bind(input.visible)
    .bind(&input.label)
    .bind(&input.template)
    .bind(&input.data_key)
    .bind(&input.source_type)
    .bind(input.maintain_aspect_ratio)
    .bind(&input.style)
    .bind(&input.align)
    .bind(&input.vertical_align)
    .bind(&input.line_style)
    .fetch_one(pool)
    .await?;
    Ok(id)
}

/// Extract image hashes from label fields
fn extract_image_hashes(fields: &[LabelField]) -> HashSet<String> {
    fields
        .iter()
        .filter(|f| f.source_type.as_deref() == Some("image"))
        .filter_map(|f| f.template.as_ref())
        .filter(|t| !t.is_empty())
        .cloned()
        .collect()
}
