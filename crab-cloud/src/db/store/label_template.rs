//! Label template database operations

use shared::cloud::store_op::StoreOpData;
use shared::models::label_template::{
    LabelField, LabelFieldAlignment, LabelFieldInput, LabelFieldType, LabelTemplate,
    LabelTemplateCreate,
};
use sqlx::PgPool;

use super::BoxError;

fn field_type_to_str(ft: &LabelFieldType) -> &'static str {
    match ft {
        LabelFieldType::Text => "text",
        LabelFieldType::Barcode => "barcode",
        LabelFieldType::Qrcode => "qrcode",
        LabelFieldType::Image => "image",
        LabelFieldType::Separator => "separator",
        LabelFieldType::Datetime => "datetime",
        LabelFieldType::Price => "price",
        LabelFieldType::Counter => "counter",
    }
}

fn alignment_to_str(a: &LabelFieldAlignment) -> &'static str {
    match a {
        LabelFieldAlignment::Left => "left",
        LabelFieldAlignment::Center => "center",
        LabelFieldAlignment::Right => "right",
    }
}

// ── Edge Sync ──

pub async fn upsert_label_template_from_sync(
    pool: &PgPool,
    store_id: i64,
    tenant_id: &str,
    source_id: i64,
    data: &serde_json::Value,
    now: i64,
) -> Result<(), BoxError> {
    let tmpl: LabelTemplate = serde_json::from_value(data.clone())?;
    let fields: Vec<LabelField> = tmpl.fields;

    let mut tx = pool.begin().await?;

    let row: Option<(i64,)> = sqlx::query_as(
        r#"
        INSERT INTO store_label_templates (
            store_id, source_id, tenant_id, name, description,
            width, height, padding, is_default, is_active,
            width_mm, height_mm, padding_mm_x, padding_mm_y,
            render_dpi, test_data, created_at, updated_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $17)
        ON CONFLICT (store_id, source_id)
        DO UPDATE SET
            name = EXCLUDED.name, description = EXCLUDED.description,
            width = EXCLUDED.width, height = EXCLUDED.height,
            padding = EXCLUDED.padding, is_default = EXCLUDED.is_default,
            is_active = EXCLUDED.is_active,
            width_mm = EXCLUDED.width_mm, height_mm = EXCLUDED.height_mm,
            padding_mm_x = EXCLUDED.padding_mm_x, padding_mm_y = EXCLUDED.padding_mm_y,
            render_dpi = EXCLUDED.render_dpi, test_data = EXCLUDED.test_data,
            updated_at = EXCLUDED.updated_at
        WHERE store_label_templates.updated_at <= EXCLUDED.updated_at
        RETURNING id
        "#,
    )
    .bind(store_id)
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
    .fetch_optional(&mut *tx)
    .await?;

    let Some((pg_id,)) = row else {
        tx.commit().await?;
        return Ok(());
    };

    // Replace fields
    sqlx::query("DELETE FROM store_label_fields WHERE template_id = $1")
        .bind(pg_id)
        .execute(&mut *tx)
        .await?;

    batch_insert_fields(&mut *tx, pg_id, &fields).await?;

    tx.commit().await?;
    Ok(())
}

async fn batch_insert_fields(
    conn: impl sqlx::Executor<'_, Database = sqlx::Postgres>,
    template_id: i64,
    fields: &[LabelField],
) -> Result<(), BoxError> {
    if fields.is_empty() {
        return Ok(());
    }
    let tids: Vec<i64> = fields.iter().map(|_| template_id).collect();
    let field_ids: Vec<&str> = fields.iter().map(|f| f.field_id.as_str()).collect();
    let names: Vec<&str> = fields.iter().map(|f| f.name.as_str()).collect();
    let field_types: Vec<&str> = fields
        .iter()
        .map(|f| field_type_to_str(&f.field_type))
        .collect();
    let xs: Vec<f32> = fields.iter().map(|f| f.x).collect();
    let ys: Vec<f32> = fields.iter().map(|f| f.y).collect();
    let widths: Vec<f32> = fields.iter().map(|f| f.width).collect();
    let heights: Vec<f32> = fields.iter().map(|f| f.height).collect();
    let font_sizes: Vec<i32> = fields.iter().map(|f| f.font_size).collect();
    let font_weights: Vec<Option<&str>> = fields.iter().map(|f| f.font_weight.as_deref()).collect();
    let font_families: Vec<Option<&str>> =
        fields.iter().map(|f| f.font_family.as_deref()).collect();
    let colors: Vec<Option<&str>> = fields.iter().map(|f| f.color.as_deref()).collect();
    let rotates: Vec<Option<i32>> = fields.iter().map(|f| f.rotate).collect();
    let alignments: Vec<Option<&str>> = fields
        .iter()
        .map(|f| f.alignment.as_ref().map(alignment_to_str))
        .collect();
    let data_sources: Vec<&str> = fields.iter().map(|f| f.data_source.as_str()).collect();
    let formats: Vec<Option<&str>> = fields.iter().map(|f| f.format.as_deref()).collect();
    let visibles: Vec<bool> = fields.iter().map(|f| f.visible).collect();
    let labels: Vec<Option<&str>> = fields.iter().map(|f| f.label.as_deref()).collect();
    let templates: Vec<Option<&str>> = fields.iter().map(|f| f.template.as_deref()).collect();
    let data_keys: Vec<Option<&str>> = fields.iter().map(|f| f.data_key.as_deref()).collect();
    let source_types: Vec<Option<&str>> = fields.iter().map(|f| f.source_type.as_deref()).collect();
    let maintain_ratios: Vec<Option<bool>> =
        fields.iter().map(|f| f.maintain_aspect_ratio).collect();
    let styles: Vec<Option<&str>> = fields.iter().map(|f| f.style.as_deref()).collect();
    let aligns: Vec<Option<&str>> = fields.iter().map(|f| f.align.as_deref()).collect();
    let v_aligns: Vec<Option<&str>> = fields.iter().map(|f| f.vertical_align.as_deref()).collect();
    let line_styles: Vec<Option<&str>> = fields.iter().map(|f| f.line_style.as_deref()).collect();

    sqlx::query(
        r#"INSERT INTO store_label_fields (
            template_id, field_id, name, field_type,
            x, y, width, height, font_size, font_weight, font_family,
            color, rotate, alignment, data_source, format, visible,
            label, template, data_key, source_type,
            maintain_aspect_ratio, style, align, vertical_align, line_style
        ) SELECT * FROM UNNEST(
            $1::bigint[], $2::text[], $3::text[], $4::text[]::label_field_type[],
            $5::real[], $6::real[], $7::real[], $8::real[],
            $9::integer[], $10::text[], $11::text[],
            $12::text[], $13::integer[], $14::text[]::label_field_alignment[], $15::text[], $16::text[], $17::boolean[],
            $18::text[], $19::text[], $20::text[], $21::text[],
            $22::boolean[], $23::text[], $24::text[], $25::text[], $26::text[]
        )"#,
    )
    .bind(&tids)
    .bind(&field_ids)
    .bind(&names)
    .bind(&field_types)
    .bind(&xs)
    .bind(&ys)
    .bind(&widths)
    .bind(&heights)
    .bind(&font_sizes)
    .bind(&font_weights)
    .bind(&font_families)
    .bind(&colors)
    .bind(&rotates)
    .bind(&alignments)
    .bind(&data_sources)
    .bind(&formats)
    .bind(&visibles)
    .bind(&labels)
    .bind(&templates)
    .bind(&data_keys)
    .bind(&source_types)
    .bind(&maintain_ratios)
    .bind(&styles)
    .bind(&aligns)
    .bind(&v_aligns)
    .bind(&line_styles)
    .execute(conn)
    .await?;
    Ok(())
}

async fn batch_insert_field_inputs(
    conn: impl sqlx::Executor<'_, Database = sqlx::Postgres>,
    template_id: i64,
    fields: &[LabelFieldInput],
) -> Result<(), BoxError> {
    if fields.is_empty() {
        return Ok(());
    }
    let tids: Vec<i64> = fields.iter().map(|_| template_id).collect();
    let field_ids: Vec<&str> = fields.iter().map(|f| f.field_id.as_str()).collect();
    let names: Vec<&str> = fields.iter().map(|f| f.name.as_str()).collect();
    let field_types: Vec<&str> = fields
        .iter()
        .map(|f| field_type_to_str(&f.field_type))
        .collect();
    let xs: Vec<f32> = fields.iter().map(|f| f.x).collect();
    let ys: Vec<f32> = fields.iter().map(|f| f.y).collect();
    let widths: Vec<f32> = fields.iter().map(|f| f.width).collect();
    let heights: Vec<f32> = fields.iter().map(|f| f.height).collect();
    let font_sizes: Vec<i32> = fields.iter().map(|f| f.font_size).collect();
    let font_weights: Vec<Option<&str>> = fields.iter().map(|f| f.font_weight.as_deref()).collect();
    let font_families: Vec<Option<&str>> =
        fields.iter().map(|f| f.font_family.as_deref()).collect();
    let colors: Vec<Option<&str>> = fields.iter().map(|f| f.color.as_deref()).collect();
    let rotates: Vec<Option<i32>> = fields.iter().map(|f| f.rotate).collect();
    let alignments: Vec<Option<&str>> = fields
        .iter()
        .map(|f| f.alignment.as_ref().map(alignment_to_str))
        .collect();
    let data_sources: Vec<&str> = fields.iter().map(|f| f.data_source.as_str()).collect();
    let formats: Vec<Option<&str>> = fields.iter().map(|f| f.format.as_deref()).collect();
    let visibles: Vec<bool> = fields.iter().map(|f| f.visible).collect();
    let labels: Vec<Option<&str>> = fields.iter().map(|f| f.label.as_deref()).collect();
    let templates: Vec<Option<&str>> = fields.iter().map(|f| f.template.as_deref()).collect();
    let data_keys: Vec<Option<&str>> = fields.iter().map(|f| f.data_key.as_deref()).collect();
    let source_types: Vec<Option<&str>> = fields.iter().map(|f| f.source_type.as_deref()).collect();
    let maintain_ratios: Vec<Option<bool>> =
        fields.iter().map(|f| f.maintain_aspect_ratio).collect();
    let styles: Vec<Option<&str>> = fields.iter().map(|f| f.style.as_deref()).collect();
    let aligns: Vec<Option<&str>> = fields.iter().map(|f| f.align.as_deref()).collect();
    let v_aligns: Vec<Option<&str>> = fields.iter().map(|f| f.vertical_align.as_deref()).collect();
    let line_styles: Vec<Option<&str>> = fields.iter().map(|f| f.line_style.as_deref()).collect();

    sqlx::query(
        r#"INSERT INTO store_label_fields (
            template_id, field_id, name, field_type,
            x, y, width, height, font_size, font_weight, font_family,
            color, rotate, alignment, data_source, format, visible,
            label, template, data_key, source_type,
            maintain_aspect_ratio, style, align, vertical_align, line_style
        ) SELECT * FROM UNNEST(
            $1::bigint[], $2::text[], $3::text[], $4::text[]::label_field_type[],
            $5::real[], $6::real[], $7::real[], $8::real[],
            $9::integer[], $10::text[], $11::text[],
            $12::text[], $13::integer[], $14::text[]::label_field_alignment[], $15::text[], $16::text[], $17::boolean[],
            $18::text[], $19::text[], $20::text[], $21::text[],
            $22::boolean[], $23::text[], $24::text[], $25::text[], $26::text[]
        )"#,
    )
    .bind(&tids)
    .bind(&field_ids)
    .bind(&names)
    .bind(&field_types)
    .bind(&xs)
    .bind(&ys)
    .bind(&widths)
    .bind(&heights)
    .bind(&font_sizes)
    .bind(&font_weights)
    .bind(&font_families)
    .bind(&colors)
    .bind(&rotates)
    .bind(&alignments)
    .bind(&data_sources)
    .bind(&formats)
    .bind(&visibles)
    .bind(&labels)
    .bind(&templates)
    .bind(&data_keys)
    .bind(&source_types)
    .bind(&maintain_ratios)
    .bind(&styles)
    .bind(&aligns)
    .bind(&v_aligns)
    .bind(&line_styles)
    .execute(conn)
    .await?;
    Ok(())
}

// ── Console Read ──

pub async fn list_label_templates(
    pool: &PgPool,
    store_id: i64,
) -> Result<Vec<LabelTemplate>, BoxError> {
    // Use internal struct to hold PG id for field join, expose source_id as id
    #[derive(sqlx::FromRow)]
    struct TmplRow {
        pg_id: i64,
        id: i64,
        name: String,
        description: Option<String>,
        width: f32,
        height: f32,
        padding: f32,
        is_default: bool,
        is_active: bool,
        width_mm: Option<f32>,
        height_mm: Option<f32>,
        padding_mm_x: Option<f32>,
        padding_mm_y: Option<f32>,
        render_dpi: Option<i32>,
        test_data: Option<String>,
        created_at: Option<i64>,
        updated_at: Option<i64>,
    }

    let rows: Vec<TmplRow> = sqlx::query_as(
        r#"
        SELECT id AS pg_id, source_id AS id, name, description, width, height, padding,
               is_default, is_active, width_mm, height_mm,
               padding_mm_x, padding_mm_y, render_dpi, test_data,
               created_at, updated_at
        FROM store_label_templates
        WHERE store_id = $1
        ORDER BY created_at
        "#,
    )
    .bind(store_id)
    .fetch_all(pool)
    .await?;

    if rows.is_empty() {
        return Ok(vec![]);
    }

    let pg_ids: Vec<i64> = rows.iter().map(|r| r.pg_id).collect();
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
    .bind(&pg_ids)
    .fetch_all(pool)
    .await?;

    let mut field_map: std::collections::HashMap<i64, Vec<LabelField>> =
        std::collections::HashMap::new();
    for f in fields {
        field_map.entry(f.template_id).or_default().push(f);
    }

    Ok(rows
        .into_iter()
        .map(|r| LabelTemplate {
            id: r.id,
            name: r.name,
            description: r.description,
            width: r.width,
            height: r.height,
            padding: r.padding,
            is_default: r.is_default,
            is_active: r.is_active,
            width_mm: r.width_mm,
            height_mm: r.height_mm,
            padding_mm_x: r.padding_mm_x,
            padding_mm_y: r.padding_mm_y,
            render_dpi: r.render_dpi,
            test_data: r.test_data,
            created_at: r.created_at,
            updated_at: r.updated_at,
            fields: field_map.remove(&r.pg_id).unwrap_or_default(),
        })
        .collect())
}

// ── Console CRUD ──

pub async fn create_label_template_direct(
    pool: &PgPool,
    store_id: i64,
    tenant_id: &str,
    data: &LabelTemplateCreate,
) -> Result<(i64, StoreOpData), BoxError> {
    let now = shared::util::now_millis();
    let mut tx = pool.begin().await?;

    let source_id = super::snowflake_id();

    let (pg_id,): (i64,) = sqlx::query_as(
        r#"
        INSERT INTO store_label_templates (
            store_id, source_id, tenant_id, name, description,
            width, height, padding, is_default, is_active,
            width_mm, height_mm, padding_mm_x, padding_mm_y,
            render_dpi, test_data, created_at, updated_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $17)
        RETURNING id
        "#,
    )
    .bind(store_id)
    .bind(source_id)
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

    batch_insert_field_inputs(&mut *tx, pg_id, &data.fields).await?;

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
    store_id: i64,
    source_id: i64,
    data: &shared::models::label_template::LabelTemplateUpdate,
) -> Result<StoreOpData, BoxError> {
    let now = shared::util::now_millis();
    let mut tx = pool.begin().await?;

    // Resolve PG id from source_id
    let pg_id: i64 = sqlx::query_scalar(
        "SELECT id FROM store_label_templates WHERE store_id = $1 AND source_id = $2",
    )
    .bind(store_id)
    .bind(source_id)
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
    .bind(pg_id)
    .execute(&mut *tx)
    .await?;

    if let Some(ref fields) = data.fields {
        sqlx::query("DELETE FROM store_label_fields WHERE template_id = $1")
            .bind(pg_id)
            .execute(&mut *tx)
            .await?;
        batch_insert_field_inputs(&mut *tx, pg_id, fields).await?;
    }

    // Read back full template (expose source_id as id)
    let tmpl: LabelTemplate = sqlx::query_as(
        r#"
        SELECT source_id AS id, name, description, width, height, padding,
               is_default, is_active, width_mm, height_mm,
               padding_mm_x, padding_mm_y, render_dpi, test_data,
               created_at, updated_at
        FROM store_label_templates
        WHERE id = $1
        "#,
    )
    .bind(pg_id)
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
    .bind(pg_id)
    .fetch_all(&mut *tx)
    .await?;

    tx.commit().await?;

    let mut result = tmpl;
    result.fields = fields;
    Ok(StoreOpData::LabelTemplate(result))
}

pub async fn delete_label_template_direct(
    pool: &PgPool,
    store_id: i64,
    source_id: i64,
) -> Result<(), BoxError> {
    let rows =
        sqlx::query("DELETE FROM store_label_templates WHERE store_id = $1 AND source_id = $2")
            .bind(store_id)
            .bind(source_id)
            .execute(pool)
            .await?;
    if rows.rows_affected() == 0 {
        return Err("Label template not found".into());
    }
    Ok(())
}
