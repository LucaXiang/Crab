//! Label Template Repository

use super::{RepoError, RepoResult};
use shared::models::{
    ImageRefEntityType, LabelField, LabelTemplate, LabelTemplateCreate, LabelTemplateUpdate,
};
use sqlx::SqlitePool;
use std::collections::HashSet;

pub async fn list(pool: &SqlitePool) -> RepoResult<Vec<LabelTemplate>> {
    let mut templates = sqlx::query_as::<_, LabelTemplate>(
        "SELECT id, name, description, width, height, padding, is_default, is_active, width_mm, height_mm, padding_mm_x, padding_mm_y, render_dpi, test_data, created_at, updated_at FROM label_template WHERE is_active = 1 ORDER BY name",
    )
    .fetch_all(pool)
    .await?;

    batch_load_fields(pool, &mut templates).await?;
    Ok(templates)
}

pub async fn list_all(pool: &SqlitePool) -> RepoResult<Vec<LabelTemplate>> {
    let mut templates = sqlx::query_as::<_, LabelTemplate>(
        "SELECT id, name, description, width, height, padding, is_default, is_active, width_mm, height_mm, padding_mm_x, padding_mm_y, render_dpi, test_data, created_at, updated_at FROM label_template ORDER BY name",
    )
    .fetch_all(pool)
    .await?;

    batch_load_fields(pool, &mut templates).await?;
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
        sqlx::query!("UPDATE label_template SET is_default = 0 WHERE is_default = 1")
            .execute(pool)
            .await?;
    }

    let now = shared::util::now_millis();
    let mut tx = pool.begin().await?;

    let id = sqlx::query_scalar!(
        r#"INSERT INTO label_template (name, description, width, height, padding, is_default, is_active, width_mm, height_mm, padding_mm_x, padding_mm_y, render_dpi, test_data, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?) RETURNING id as "id!""#,
        data.name,
        data.description,
        data.width,
        data.height,
        data.padding,
        data.is_default,
        data.is_active,
        data.width_mm,
        data.height_mm,
        data.padding_mm_x,
        data.padding_mm_y,
        data.render_dpi,
        data.test_data,
        now,
        now,
    )
    .fetch_one(&mut *tx)
    .await?;

    // Create fields
    for field in &data.fields {
        sqlx::query!(
            "INSERT INTO label_field (template_id, field_id, name, field_type, x, y, width, height, font_size, font_weight, font_family, color, rotate, alignment, data_source, format, visible, label, template, data_key, source_type, maintain_aspect_ratio, style, align, vertical_align, line_style) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            id,
            field.field_id,
            field.name,
            field.field_type,
            field.x,
            field.y,
            field.width,
            field.height,
            field.font_size,
            field.font_weight,
            field.font_family,
            field.color,
            field.rotate,
            field.alignment,
            field.data_source,
            field.format,
            field.visible,
            field.label,
            field.template,
            field.data_key,
            field.source_type,
            field.maintain_aspect_ratio,
            field.style,
            field.align,
            field.vertical_align,
            field.line_style,
        )
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;

    // Sync image refs (non-critical, outside transaction)
    let template = get(pool, id)
        .await?
        .ok_or_else(|| RepoError::Database("Failed to create label template".into()))?;

    let image_hashes = extract_image_hashes(&template.fields);
    if !image_hashes.is_empty() {
        let _ =
            super::image_ref::sync_refs(pool, ImageRefEntityType::LabelTemplate, id, image_hashes)
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
        sqlx::query!(
            "UPDATE label_template SET is_default = 0 WHERE is_default = 1 AND id != ?",
            id
        )
        .execute(pool)
        .await?;
    }

    let now = shared::util::now_millis();
    let rows = sqlx::query!(
        "UPDATE label_template SET name = COALESCE(?, name), description = COALESCE(?, description), width = COALESCE(?, width), height = COALESCE(?, height), padding = COALESCE(?, padding), is_default = COALESCE(?, is_default), is_active = COALESCE(?, is_active), width_mm = COALESCE(?, width_mm), height_mm = COALESCE(?, height_mm), padding_mm_x = COALESCE(?, padding_mm_x), padding_mm_y = COALESCE(?, padding_mm_y), render_dpi = COALESCE(?, render_dpi), test_data = COALESCE(?, test_data), updated_at = ? WHERE id = ?",
        data.name,
        data.description,
        data.width,
        data.height,
        data.padding,
        data.is_default,
        data.is_active,
        data.width_mm,
        data.height_mm,
        data.padding_mm_x,
        data.padding_mm_y,
        data.render_dpi,
        data.test_data,
        now,
        id,
    )
    .execute(pool)
    .await?;

    if rows.rows_affected() == 0 {
        return Err(RepoError::NotFound(format!(
            "Label template {id} not found"
        )));
    }

    // Replace fields if provided (atomic: delete + re-create in transaction)
    if let Some(fields) = &data.fields {
        let mut tx = pool.begin().await?;
        sqlx::query!("DELETE FROM label_field WHERE template_id = ?", id)
            .execute(&mut *tx)
            .await?;
        for field in fields {
            sqlx::query!(
                "INSERT INTO label_field (template_id, field_id, name, field_type, x, y, width, height, font_size, font_weight, font_family, color, rotate, alignment, data_source, format, visible, label, template, data_key, source_type, maintain_aspect_ratio, style, align, vertical_align, line_style) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                id,
                field.field_id,
                field.name,
                field.field_type,
                field.x,
                field.y,
                field.width,
                field.height,
                field.font_size,
                field.font_weight,
                field.font_family,
                field.color,
                field.rotate,
                field.alignment,
                field.data_source,
                field.format,
                field.visible,
                field.label,
                field.template,
                field.data_key,
                field.source_type,
                field.maintain_aspect_ratio,
                field.style,
                field.align,
                field.vertical_align,
                field.line_style,
            )
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
    }

    let updated = get(pool, id)
        .await?
        .ok_or_else(|| RepoError::NotFound(format!("Label template {id} not found")))?;

    // Sync image refs
    let image_hashes = extract_image_hashes(&updated.fields);
    let _ = super::image_ref::sync_refs(pool, ImageRefEntityType::LabelTemplate, id, image_hashes)
        .await;

    Ok(updated)
}

pub async fn delete(pool: &SqlitePool, id: i64) -> RepoResult<bool> {
    // Clean up image refs
    let _ = super::image_ref::delete_entity_refs(pool, ImageRefEntityType::LabelTemplate, id).await;

    // Soft delete
    sqlx::query!("UPDATE label_template SET is_active = 0 WHERE id = ?", id)
        .execute(pool)
        .await?;
    Ok(true)
}

pub async fn hard_delete(pool: &SqlitePool, id: i64) -> RepoResult<bool> {
    let _ = super::image_ref::delete_entity_refs(pool, ImageRefEntityType::LabelTemplate, id).await;

    // Fields cascade via FK
    let result = sqlx::query!("DELETE FROM label_template WHERE id = ?", id)
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

/// Batch load fields for multiple templates (eliminates N+1)
async fn batch_load_fields(pool: &SqlitePool, templates: &mut [LabelTemplate]) -> RepoResult<()> {
    if templates.is_empty() {
        return Ok(());
    }
    let ids: Vec<i64> = templates.iter().map(|t| t.id).collect();
    let placeholders = ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    let sql = format!(
        "SELECT id, template_id, field_id, name, field_type, x, y, width, height, font_size, font_weight, font_family, color, rotate, alignment, data_source, format, visible, label, template, data_key, source_type, maintain_aspect_ratio, style, align, vertical_align, line_style FROM label_field WHERE template_id IN ({placeholders})"
    );
    let mut query = sqlx::query_as::<_, LabelField>(&sql);
    for id in &ids {
        query = query.bind(id);
    }
    let all_fields = query.fetch_all(pool).await?;

    let mut map: std::collections::HashMap<i64, Vec<LabelField>> = std::collections::HashMap::new();
    for f in all_fields {
        map.entry(f.template_id).or_default().push(f);
    }
    for t in templates.iter_mut() {
        t.fields = map.remove(&t.id).unwrap_or_default();
    }
    Ok(())
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
