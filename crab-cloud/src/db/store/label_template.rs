//! Label template database operations

use shared::cloud::store_op::StoreOpData;
use shared::models::label_template::{
    LabelField, LabelFieldInput, LabelTemplate, LabelTemplateCreate,
};
use sqlx::PgPool;

use super::BoxError;

// ── Edge Sync ──

pub async fn upsert_label_template_from_sync(
    pool: &PgPool,
    edge_server_id: i64,
    tenant_id: &str,
    source_id: i64,
    data: &serde_json::Value,
    now: i64,
) -> Result<(), BoxError> {
    let tmpl: LabelTemplate = serde_json::from_value(data.clone())?;
    let fields: Vec<LabelField> = tmpl.fields;

    let mut tx = pool.begin().await?;

    let (pg_id,): (i64,) = sqlx::query_as(
        r#"
        INSERT INTO store_label_templates (
            edge_server_id, source_id, tenant_id, name, description,
            width, height, padding, is_default, is_active,
            width_mm, height_mm, padding_mm_x, padding_mm_y,
            render_dpi, test_data, created_at, updated_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $17)
        ON CONFLICT (edge_server_id, source_id)
        DO UPDATE SET
            name = EXCLUDED.name, description = EXCLUDED.description,
            width = EXCLUDED.width, height = EXCLUDED.height,
            padding = EXCLUDED.padding, is_default = EXCLUDED.is_default,
            is_active = EXCLUDED.is_active,
            width_mm = EXCLUDED.width_mm, height_mm = EXCLUDED.height_mm,
            padding_mm_x = EXCLUDED.padding_mm_x, padding_mm_y = EXCLUDED.padding_mm_y,
            render_dpi = EXCLUDED.render_dpi, test_data = EXCLUDED.test_data,
            updated_at = EXCLUDED.updated_at
        RETURNING id
        "#,
    )
    .bind(edge_server_id)
    .bind(source_id)
    .bind(tenant_id)
    .bind(&tmpl.name)
    .bind(&tmpl.description)
    .bind(tmpl.width)
    .bind(tmpl.height)
    .bind(tmpl.padding)
    .bind(tmpl.is_default)
    .bind(tmpl.is_active)
    .bind(tmpl.width_mm)
    .bind(tmpl.height_mm)
    .bind(tmpl.padding_mm_x)
    .bind(tmpl.padding_mm_y)
    .bind(tmpl.render_dpi)
    .bind(&tmpl.test_data)
    .bind(now)
    .fetch_one(&mut *tx)
    .await?;

    // Replace fields
    sqlx::query("DELETE FROM store_label_fields WHERE template_id = $1")
        .bind(pg_id)
        .execute(&mut *tx)
        .await?;

    for f in &fields {
        insert_field(&mut *tx, pg_id, f).await?;
    }

    tx.commit().await?;
    Ok(())
}

async fn insert_field(
    conn: impl sqlx::Executor<'_, Database = sqlx::Postgres>,
    template_id: i64,
    f: &LabelField,
) -> Result<(), BoxError> {
    sqlx::query(
        r#"
        INSERT INTO store_label_fields (
            template_id, field_id, name, field_type,
            x, y, width, height, font_size, font_weight, font_family,
            color, rotate, alignment, data_source, format, visible,
            label, template, data_key, source_type,
            maintain_aspect_ratio, style, align, vertical_align, line_style
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22, $23, $24, $25, $26)
        "#,
    )
    .bind(template_id)
    .bind(&f.field_id)
    .bind(&f.name)
    .bind(&f.field_type)
    .bind(f.x)
    .bind(f.y)
    .bind(f.width)
    .bind(f.height)
    .bind(f.font_size)
    .bind(&f.font_weight)
    .bind(&f.font_family)
    .bind(&f.color)
    .bind(f.rotate)
    .bind(&f.alignment)
    .bind(&f.data_source)
    .bind(&f.format)
    .bind(f.visible)
    .bind(&f.label)
    .bind(&f.template)
    .bind(&f.data_key)
    .bind(&f.source_type)
    .bind(f.maintain_aspect_ratio)
    .bind(&f.style)
    .bind(&f.align)
    .bind(&f.vertical_align)
    .bind(&f.line_style)
    .execute(conn)
    .await?;
    Ok(())
}

async fn insert_field_input(
    conn: impl sqlx::Executor<'_, Database = sqlx::Postgres>,
    template_id: i64,
    f: &LabelFieldInput,
) -> Result<(), BoxError> {
    sqlx::query(
        r#"
        INSERT INTO store_label_fields (
            template_id, field_id, name, field_type,
            x, y, width, height, font_size, font_weight, font_family,
            color, rotate, alignment, data_source, format, visible,
            label, template, data_key, source_type,
            maintain_aspect_ratio, style, align, vertical_align, line_style
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22, $23, $24, $25, $26)
        "#,
    )
    .bind(template_id)
    .bind(&f.field_id)
    .bind(&f.name)
    .bind(&f.field_type)
    .bind(f.x)
    .bind(f.y)
    .bind(f.width)
    .bind(f.height)
    .bind(f.font_size)
    .bind(&f.font_weight)
    .bind(&f.font_family)
    .bind(&f.color)
    .bind(f.rotate)
    .bind(&f.alignment)
    .bind(&f.data_source)
    .bind(&f.format)
    .bind(f.visible)
    .bind(&f.label)
    .bind(&f.template)
    .bind(&f.data_key)
    .bind(&f.source_type)
    .bind(f.maintain_aspect_ratio)
    .bind(&f.style)
    .bind(&f.align)
    .bind(&f.vertical_align)
    .bind(&f.line_style)
    .execute(conn)
    .await?;
    Ok(())
}

// ── Console Read ──

pub async fn list_label_templates(
    pool: &PgPool,
    edge_server_id: i64,
) -> Result<Vec<LabelTemplate>, BoxError> {
    let templates: Vec<LabelTemplate> = sqlx::query_as(
        r#"
        SELECT id, name, description, width, height, padding,
               is_default, is_active, width_mm, height_mm,
               padding_mm_x, padding_mm_y, render_dpi, test_data,
               created_at, updated_at
        FROM store_label_templates
        WHERE edge_server_id = $1
        ORDER BY created_at
        "#,
    )
    .bind(edge_server_id)
    .fetch_all(pool)
    .await?;

    if templates.is_empty() {
        return Ok(vec![]);
    }

    let template_ids: Vec<i64> = templates.iter().map(|t| t.id).collect();
    let fields: Vec<LabelField> = sqlx::query_as(
        r#"
        SELECT id, template_id, field_id, name, field_type,
               x, y, width, height, font_size, font_weight, font_family,
               color, rotate, alignment, data_source, format, visible,
               label, template, data_key, source_type,
               maintain_aspect_ratio, style, align, vertical_align, line_style
        FROM store_label_fields
        WHERE template_id = ANY($1)
        ORDER BY id
        "#,
    )
    .bind(&template_ids)
    .fetch_all(pool)
    .await?;

    let mut field_map: std::collections::HashMap<i64, Vec<LabelField>> =
        std::collections::HashMap::new();
    for f in fields {
        field_map.entry(f.template_id).or_default().push(f);
    }

    Ok(templates
        .into_iter()
        .map(|mut t| {
            t.fields = field_map.remove(&t.id).unwrap_or_default();
            t
        })
        .collect())
}

// ── Console CRUD ──

pub async fn create_label_template_direct(
    pool: &PgPool,
    edge_server_id: i64,
    tenant_id: &str,
    data: &LabelTemplateCreate,
) -> Result<(i64, StoreOpData), BoxError> {
    let now = shared::util::now_millis();
    let mut tx = pool.begin().await?;

    let (pg_id,): (i64,) = sqlx::query_as(
        r#"
        INSERT INTO store_label_templates (
            edge_server_id, source_id, tenant_id, name, description,
            width, height, padding, is_default, is_active,
            width_mm, height_mm, padding_mm_x, padding_mm_y,
            render_dpi, test_data, created_at, updated_at
        )
        VALUES ($1, 0, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $16)
        RETURNING id
        "#,
    )
    .bind(edge_server_id)
    .bind(tenant_id)
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
    .fetch_one(&mut *tx)
    .await?;

    let source_id = super::snowflake_id();
    sqlx::query("UPDATE store_label_templates SET source_id = $1 WHERE id = $2")
        .bind(source_id)
        .bind(pg_id)
        .execute(&mut *tx)
        .await?;

    for f in &data.fields {
        insert_field_input(&mut *tx, pg_id, f).await?;
    }

    // Read back fields with IDs
    let fields: Vec<LabelField> = sqlx::query_as(
        r#"
        SELECT id, template_id, field_id, name, field_type,
               x, y, width, height, font_size, font_weight, font_family,
               color, rotate, alignment, data_source, format, visible,
               label, template, data_key, source_type,
               maintain_aspect_ratio, style, align, vertical_align, line_style
        FROM store_label_fields
        WHERE template_id = $1
        ORDER BY id
        "#,
    )
    .bind(pg_id)
    .fetch_all(&mut *tx)
    .await?;

    tx.commit().await?;

    let tmpl = LabelTemplate {
        id: source_id,
        name: data.name.clone(),
        description: data.description.clone(),
        width: data.width,
        height: data.height,
        padding: data.padding,
        is_default: data.is_default,
        is_active: data.is_active,
        width_mm: data.width_mm,
        height_mm: data.height_mm,
        padding_mm_x: data.padding_mm_x,
        padding_mm_y: data.padding_mm_y,
        render_dpi: data.render_dpi,
        test_data: data.test_data.clone(),
        created_at: Some(now),
        updated_at: Some(now),
        fields,
    };
    Ok((source_id, StoreOpData::LabelTemplate(tmpl)))
}

pub async fn update_label_template_direct(
    pool: &PgPool,
    edge_server_id: i64,
    template_id: i64,
    data: &shared::models::label_template::LabelTemplateUpdate,
) -> Result<StoreOpData, BoxError> {
    let now = shared::util::now_millis();
    let mut tx = pool.begin().await?;

    // Verify ownership
    let _: i64 = sqlx::query_scalar(
        "SELECT id FROM store_label_templates WHERE id = $1 AND edge_server_id = $2",
    )
    .bind(template_id)
    .bind(edge_server_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or("Label template not found")?;

    sqlx::query(
        r#"
        UPDATE store_label_templates SET
            name = COALESCE($1, name),
            description = COALESCE($2, description),
            width = COALESCE($3, width),
            height = COALESCE($4, height),
            padding = COALESCE($5, padding),
            is_default = COALESCE($6, is_default),
            is_active = COALESCE($7, is_active),
            width_mm = COALESCE($8, width_mm),
            height_mm = COALESCE($9, height_mm),
            padding_mm_x = COALESCE($10, padding_mm_x),
            padding_mm_y = COALESCE($11, padding_mm_y),
            render_dpi = COALESCE($12, render_dpi),
            test_data = COALESCE($13, test_data),
            updated_at = $14
        WHERE id = $15
        "#,
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
    .bind(template_id)
    .execute(&mut *tx)
    .await?;

    if let Some(ref fields) = data.fields {
        sqlx::query("DELETE FROM store_label_fields WHERE template_id = $1")
            .bind(template_id)
            .execute(&mut *tx)
            .await?;
        for f in fields {
            insert_field_input(&mut *tx, template_id, f).await?;
        }
    }

    // Read back full template
    let tmpl: LabelTemplate = sqlx::query_as(
        r#"
        SELECT id, name, description, width, height, padding,
               is_default, is_active, width_mm, height_mm,
               padding_mm_x, padding_mm_y, render_dpi, test_data,
               created_at, updated_at
        FROM store_label_templates
        WHERE id = $1
        "#,
    )
    .bind(template_id)
    .fetch_one(&mut *tx)
    .await?;

    let fields: Vec<LabelField> = sqlx::query_as(
        r#"
        SELECT id, template_id, field_id, name, field_type,
               x, y, width, height, font_size, font_weight, font_family,
               color, rotate, alignment, data_source, format, visible,
               label, template, data_key, source_type,
               maintain_aspect_ratio, style, align, vertical_align, line_style
        FROM store_label_fields
        WHERE template_id = $1
        ORDER BY id
        "#,
    )
    .bind(template_id)
    .fetch_all(&mut *tx)
    .await?;

    tx.commit().await?;

    let mut result = tmpl;
    result.fields = fields;
    Ok(StoreOpData::LabelTemplate(result))
}

pub async fn delete_label_template_direct(
    pool: &PgPool,
    edge_server_id: i64,
    template_id: i64,
) -> Result<(), BoxError> {
    let rows =
        sqlx::query("DELETE FROM store_label_templates WHERE id = $1 AND edge_server_id = $2")
            .bind(template_id)
            .bind(edge_server_id)
            .execute(pool)
            .await?;
    if rows.rows_affected() == 0 {
        return Err("Label template not found".into());
    }
    Ok(())
}
