//! Attribute + binding database operations

use serde::{Deserialize, Serialize};
use shared::cloud::store_op::StoreOpData;
use shared::models::attribute::{Attribute, AttributeBinding};
use sqlx::PgPool;

use super::BoxError;

// ── Edge Sync ──

pub async fn upsert_attribute_from_sync(
    pool: &PgPool,
    edge_server_id: i64,
    source_id: i64,
    data: &serde_json::Value,
    now: i64,
) -> Result<(), BoxError> {
    let attr: Attribute = serde_json::from_value(data.clone())?;
    let default_ids_json = attr
        .default_option_ids
        .as_ref()
        .map(serde_json::to_value)
        .transpose()?;

    let mut tx = pool.begin().await?;

    let row: Option<(i64,)> = sqlx::query_as(
        r#"
        INSERT INTO store_attributes (
            edge_server_id, source_id, name, is_multi_select, max_selections,
            default_option_ids, display_order, is_active,
            show_on_receipt, receipt_name, show_on_kitchen_print, kitchen_print_name,
            updated_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
        ON CONFLICT (edge_server_id, source_id)
        DO UPDATE SET
            name = EXCLUDED.name, is_multi_select = EXCLUDED.is_multi_select,
            max_selections = EXCLUDED.max_selections, default_option_ids = EXCLUDED.default_option_ids,
            display_order = EXCLUDED.display_order, is_active = EXCLUDED.is_active,
            show_on_receipt = EXCLUDED.show_on_receipt, receipt_name = EXCLUDED.receipt_name,
            show_on_kitchen_print = EXCLUDED.show_on_kitchen_print,
            kitchen_print_name = EXCLUDED.kitchen_print_name,
            updated_at = EXCLUDED.updated_at
        WHERE store_attributes.updated_at <= EXCLUDED.updated_at
        RETURNING id
        "#,
    )
    .bind(edge_server_id)
    .bind(source_id)
    .bind(&attr.name)
    .bind(attr.is_multi_select)
    .bind(attr.max_selections)
    .bind(&default_ids_json)
    .bind(attr.display_order)
    .bind(attr.is_active)
    .bind(attr.show_on_receipt)
    .bind(&attr.receipt_name)
    .bind(attr.show_on_kitchen_print)
    .bind(&attr.kitchen_print_name)
    .bind(now)
    .fetch_optional(&mut *tx)
    .await?;

    let Some((pg_id,)) = row else {
        tx.commit().await?;
        return Ok(());
    };

    // Replace options
    sqlx::query("DELETE FROM store_attribute_options WHERE attribute_id = $1")
        .bind(pg_id)
        .execute(&mut *tx)
        .await?;

    for opt in &attr.options {
        sqlx::query(
            r#"
            INSERT INTO store_attribute_options (
                attribute_id, source_id, name, price_modifier, display_order,
                is_active, receipt_name, kitchen_print_name, enable_quantity, max_quantity
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            "#,
        )
        .bind(pg_id)
        .bind(opt.id)
        .bind(&opt.name)
        .bind(opt.price_modifier)
        .bind(opt.display_order)
        .bind(opt.is_active)
        .bind(&opt.receipt_name)
        .bind(&opt.kitchen_print_name)
        .bind(opt.enable_quantity)
        .bind(opt.max_quantity)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    Ok(())
}

pub async fn upsert_binding_from_sync(
    pool: &PgPool,
    edge_server_id: i64,
    source_id: i64,
    data: &serde_json::Value,
    _now: i64,
) -> Result<(), BoxError> {
    let binding: AttributeBinding = serde_json::from_value(data.clone())?;
    let default_ids_json = binding
        .default_option_ids
        .as_ref()
        .map(serde_json::to_value)
        .transpose()?;

    sqlx::query(
        r#"
        INSERT INTO store_attribute_bindings (
            edge_server_id, source_id, owner_type, owner_source_id,
            attribute_source_id, is_required, display_order, default_option_ids
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        ON CONFLICT (edge_server_id, source_id)
        DO UPDATE SET
            owner_type = EXCLUDED.owner_type, owner_source_id = EXCLUDED.owner_source_id,
            attribute_source_id = EXCLUDED.attribute_source_id,
            is_required = EXCLUDED.is_required, display_order = EXCLUDED.display_order,
            default_option_ids = EXCLUDED.default_option_ids
        "#,
    )
    .bind(edge_server_id)
    .bind(source_id)
    .bind(&binding.owner_type)
    .bind(binding.owner_id)
    .bind(binding.attribute_id)
    .bind(binding.is_required)
    .bind(binding.display_order)
    .bind(&default_ids_json)
    .execute(pool)
    .await?;
    Ok(())
}

// ── Console Read Types ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreAttribute {
    pub source_id: i64,
    pub name: String,
    pub is_multi_select: bool,
    pub max_selections: Option<i32>,
    pub default_option_ids: Option<Vec<i32>>,
    pub display_order: i32,
    pub is_active: bool,
    pub show_on_receipt: bool,
    pub receipt_name: Option<String>,
    pub show_on_kitchen_print: bool,
    pub kitchen_print_name: Option<String>,
    pub options: Vec<StoreAttributeOption>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreAttributeOption {
    pub source_id: i64,
    pub name: String,
    pub price_modifier: f64,
    pub display_order: i32,
    pub is_active: bool,
    pub receipt_name: Option<String>,
    pub kitchen_print_name: Option<String>,
    pub enable_quantity: bool,
    pub max_quantity: Option<i32>,
}

// ── Console Read ──

pub async fn list_attributes(
    pool: &PgPool,
    edge_server_id: i64,
) -> Result<Vec<StoreAttribute>, BoxError> {
    #[derive(sqlx::FromRow)]
    struct AttrRow {
        id: i64,
        source_id: i64,
        name: String,
        is_multi_select: bool,
        max_selections: Option<i32>,
        default_option_ids: Option<serde_json::Value>,
        display_order: i32,
        is_active: bool,
        show_on_receipt: bool,
        receipt_name: Option<String>,
        show_on_kitchen_print: bool,
        kitchen_print_name: Option<String>,
    }

    #[derive(sqlx::FromRow)]
    struct OptRow {
        attribute_id: i64,
        source_id: i64,
        name: String,
        price_modifier: f64,
        display_order: i32,
        is_active: bool,
        receipt_name: Option<String>,
        kitchen_print_name: Option<String>,
        enable_quantity: bool,
        max_quantity: Option<i32>,
    }

    let rows: Vec<AttrRow> = sqlx::query_as(
        r#"
        SELECT id, source_id, name, is_multi_select, max_selections,
               default_option_ids, display_order, is_active,
               show_on_receipt, receipt_name, show_on_kitchen_print, kitchen_print_name
        FROM store_attributes
        WHERE edge_server_id = $1
        ORDER BY display_order, source_id
        "#,
    )
    .bind(edge_server_id)
    .fetch_all(pool)
    .await?;

    let pg_ids: Vec<i64> = rows.iter().map(|r| r.id).collect();
    if pg_ids.is_empty() {
        return Ok(vec![]);
    }

    let opts: Vec<OptRow> = sqlx::query_as(
        r#"
        SELECT attribute_id, source_id, name, price_modifier, display_order,
               is_active, receipt_name, kitchen_print_name, enable_quantity, max_quantity
        FROM store_attribute_options
        WHERE attribute_id = ANY($1)
        ORDER BY display_order
        "#,
    )
    .bind(&pg_ids)
    .fetch_all(pool)
    .await?;

    let mut opt_map: std::collections::HashMap<i64, Vec<StoreAttributeOption>> =
        std::collections::HashMap::new();
    for o in opts {
        opt_map
            .entry(o.attribute_id)
            .or_default()
            .push(StoreAttributeOption {
                source_id: o.source_id,
                name: o.name,
                price_modifier: o.price_modifier,
                display_order: o.display_order,
                is_active: o.is_active,
                receipt_name: o.receipt_name,
                kitchen_print_name: o.kitchen_print_name,
                enable_quantity: o.enable_quantity,
                max_quantity: o.max_quantity,
            });
    }

    Ok(rows
        .into_iter()
        .map(|r| {
            let default_ids: Option<Vec<i32>> = r
                .default_option_ids
                .and_then(|v| serde_json::from_value(v).ok());
            StoreAttribute {
                source_id: r.source_id,
                name: r.name,
                is_multi_select: r.is_multi_select,
                max_selections: r.max_selections,
                default_option_ids: default_ids,
                display_order: r.display_order,
                is_active: r.is_active,
                show_on_receipt: r.show_on_receipt,
                receipt_name: r.receipt_name,
                show_on_kitchen_print: r.show_on_kitchen_print,
                kitchen_print_name: r.kitchen_print_name,
                options: opt_map.remove(&r.id).unwrap_or_default(),
            }
        })
        .collect())
}

// ── Console CRUD ──

pub async fn create_attribute_direct(
    pool: &PgPool,
    edge_server_id: i64,
    data: &shared::models::attribute::AttributeCreate,
) -> Result<(i64, StoreOpData), BoxError> {
    use shared::models::attribute::AttributeOption;

    let now = shared::util::now_millis();
    let is_multi_select = data.is_multi_select.unwrap_or(false);
    let display_order = data.display_order.unwrap_or(0);
    let show_on_receipt = data.show_on_receipt.unwrap_or(false);
    let show_on_kitchen_print = data.show_on_kitchen_print.unwrap_or(false);
    let default_ids_json = data
        .default_option_ids
        .as_ref()
        .map(serde_json::to_value)
        .transpose()?;
    let mut tx = pool.begin().await?;

    let (pg_id,): (i64,) = sqlx::query_as(
        r#"INSERT INTO store_attributes (edge_server_id, source_id, name, is_multi_select, max_selections, default_option_ids, display_order, is_active, show_on_receipt, receipt_name, show_on_kitchen_print, kitchen_print_name, updated_at) VALUES ($1, 0, $2, $3, $4, $5, $6, TRUE, $7, $8, $9, $10, $11) RETURNING id"#,
    )
    .bind(edge_server_id).bind(&data.name).bind(is_multi_select).bind(data.max_selections).bind(&default_ids_json).bind(display_order).bind(show_on_receipt).bind(&data.receipt_name).bind(show_on_kitchen_print).bind(&data.kitchen_print_name).bind(now)
    .fetch_one(&mut *tx).await?;

    let source_id = super::snowflake_id();
    sqlx::query("UPDATE store_attributes SET source_id = $1 WHERE id = $2")
        .bind(source_id)
        .bind(pg_id)
        .execute(&mut *tx)
        .await?;

    if let Some(ref options) = data.options {
        for opt in options {
            sqlx::query("INSERT INTO store_attribute_options (attribute_id, source_id, name, price_modifier, display_order, is_active, receipt_name, kitchen_print_name, enable_quantity, max_quantity) VALUES ($1, 0, $2, $3, $4, TRUE, $5, $6, $7, $8)")
                .bind(pg_id).bind(&opt.name).bind(opt.price_modifier).bind(opt.display_order).bind(&opt.receipt_name).bind(&opt.kitchen_print_name).bind(opt.enable_quantity).bind(opt.max_quantity)
                .execute(&mut *tx).await?;
        }
        for row in sqlx::query_as::<_, (i64,)>(
            "SELECT id FROM store_attribute_options WHERE attribute_id = $1",
        )
        .bind(pg_id)
        .fetch_all(&mut *tx)
        .await?
        {
            sqlx::query("UPDATE store_attribute_options SET source_id = $1 WHERE id = $2")
                .bind(super::snowflake_id())
                .bind(row.0)
                .execute(&mut *tx)
                .await?;
        }
    }

    #[derive(sqlx::FromRow)]
    struct OptRow {
        id: i64,
        attribute_id: i64,
        name: String,
        price_modifier: f64,
        display_order: i32,
        is_active: bool,
        receipt_name: Option<String>,
        kitchen_print_name: Option<String>,
        enable_quantity: bool,
        max_quantity: Option<i32>,
    }
    let opt_rows: Vec<OptRow> = sqlx::query_as(
        "SELECT id, attribute_id, name, price_modifier, display_order, is_active, receipt_name, kitchen_print_name, enable_quantity, max_quantity FROM store_attribute_options WHERE attribute_id = $1 ORDER BY display_order",
    )
    .bind(pg_id)
    .fetch_all(&mut *tx)
    .await?;

    tx.commit().await?;

    let options: Vec<AttributeOption> = opt_rows
        .into_iter()
        .map(|r| AttributeOption {
            id: r.id,
            attribute_id: r.attribute_id,
            name: r.name,
            price_modifier: r.price_modifier,
            display_order: r.display_order,
            is_active: r.is_active,
            receipt_name: r.receipt_name,
            kitchen_print_name: r.kitchen_print_name,
            enable_quantity: r.enable_quantity,
            max_quantity: r.max_quantity,
        })
        .collect();

    let attr = Attribute {
        id: source_id,
        name: data.name.clone(),
        is_multi_select,
        max_selections: data.max_selections,
        default_option_ids: data.default_option_ids.clone(),
        display_order,
        is_active: true,
        show_on_receipt,
        receipt_name: data.receipt_name.clone(),
        show_on_kitchen_print,
        kitchen_print_name: data.kitchen_print_name.clone(),
        options,
    };
    Ok((source_id, StoreOpData::Attribute(attr)))
}

pub async fn update_attribute_direct(
    pool: &PgPool,
    edge_server_id: i64,
    source_id: i64,
    data: &shared::models::attribute::AttributeUpdate,
) -> Result<(), BoxError> {
    let now = shared::util::now_millis();
    let default_ids_json = data
        .default_option_ids
        .as_ref()
        .map(serde_json::to_value)
        .transpose()?;
    let mut tx = pool.begin().await?;

    let pg_id: i64 = sqlx::query_scalar(
        "SELECT id FROM store_attributes WHERE edge_server_id = $1 AND source_id = $2",
    )
    .bind(edge_server_id)
    .bind(source_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or("Attribute not found")?;

    sqlx::query("UPDATE store_attributes SET name = COALESCE($1, name), is_multi_select = COALESCE($2, is_multi_select), max_selections = COALESCE($3, max_selections), default_option_ids = COALESCE($4, default_option_ids), display_order = COALESCE($5, display_order), show_on_receipt = COALESCE($6, show_on_receipt), receipt_name = COALESCE($7, receipt_name), show_on_kitchen_print = COALESCE($8, show_on_kitchen_print), kitchen_print_name = COALESCE($9, kitchen_print_name), is_active = COALESCE($10, is_active), updated_at = $11 WHERE id = $12")
        .bind(&data.name).bind(data.is_multi_select).bind(data.max_selections).bind(&default_ids_json).bind(data.display_order).bind(data.show_on_receipt).bind(&data.receipt_name).bind(data.show_on_kitchen_print).bind(&data.kitchen_print_name).bind(data.is_active).bind(now).bind(pg_id)
        .execute(&mut *tx).await?;

    if let Some(ref options) = data.options {
        sqlx::query("DELETE FROM store_attribute_options WHERE attribute_id = $1")
            .bind(pg_id)
            .execute(&mut *tx)
            .await?;
        for opt in options {
            sqlx::query("INSERT INTO store_attribute_options (attribute_id, source_id, name, price_modifier, display_order, is_active, receipt_name, kitchen_print_name, enable_quantity, max_quantity) VALUES ($1, 0, $2, $3, $4, TRUE, $5, $6, $7, $8)")
                .bind(pg_id).bind(&opt.name).bind(opt.price_modifier).bind(opt.display_order).bind(&opt.receipt_name).bind(&opt.kitchen_print_name).bind(opt.enable_quantity).bind(opt.max_quantity)
                .execute(&mut *tx).await?;
        }
        for row in sqlx::query_as::<_, (i64,)>(
            "SELECT id FROM store_attribute_options WHERE attribute_id = $1",
        )
        .bind(pg_id)
        .fetch_all(&mut *tx)
        .await?
        {
            sqlx::query("UPDATE store_attribute_options SET source_id = $1 WHERE id = $2")
                .bind(super::snowflake_id())
                .bind(row.0)
                .execute(&mut *tx)
                .await?;
        }
    }

    tx.commit().await?;
    Ok(())
}

pub async fn delete_attribute_direct(
    pool: &PgPool,
    edge_server_id: i64,
    source_id: i64,
) -> Result<(), BoxError> {
    let rows =
        sqlx::query("DELETE FROM store_attributes WHERE edge_server_id = $1 AND source_id = $2")
            .bind(edge_server_id)
            .bind(source_id)
            .execute(pool)
            .await?;
    if rows.rows_affected() == 0 {
        return Err("Attribute not found".into());
    }
    Ok(())
}

// ── Option Independent CRUD ──

pub async fn create_option_direct(
    pool: &PgPool,
    edge_server_id: i64,
    attribute_source_id: i64,
    data: &shared::models::attribute::AttributeOptionCreate,
) -> Result<i64, BoxError> {
    // Find PG attribute_id from source_id
    let pg_attr_id: i64 = sqlx::query_scalar(
        "SELECT id FROM store_attributes WHERE edge_server_id = $1 AND source_id = $2",
    )
    .bind(edge_server_id)
    .bind(attribute_source_id)
    .fetch_optional(pool)
    .await?
    .ok_or("Attribute not found")?;

    let mut tx = pool.begin().await?;

    let (pg_id,): (i64,) = sqlx::query_as(
        r#"INSERT INTO store_attribute_options (attribute_id, source_id, name, price_modifier, display_order, is_active, receipt_name, kitchen_print_name, enable_quantity, max_quantity) VALUES ($1, 0, $2, $3, $4, TRUE, $5, $6, $7, $8) RETURNING id"#,
    )
    .bind(pg_attr_id)
    .bind(&data.name)
    .bind(data.price_modifier)
    .bind(data.display_order)
    .bind(&data.receipt_name)
    .bind(&data.kitchen_print_name)
    .bind(data.enable_quantity)
    .bind(data.max_quantity)
    .fetch_one(&mut *tx)
    .await?;

    let source_id = super::snowflake_id();
    sqlx::query("UPDATE store_attribute_options SET source_id = $1 WHERE id = $2")
        .bind(source_id)
        .bind(pg_id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;
    Ok(source_id)
}

pub async fn update_option_direct(
    pool: &PgPool,
    edge_server_id: i64,
    option_source_id: i64,
    data: &shared::models::attribute::AttributeOptionUpdate,
) -> Result<(), BoxError> {
    let rows = sqlx::query(
        r#"UPDATE store_attribute_options SET
            name = COALESCE($1, name),
            price_modifier = COALESCE($2, price_modifier),
            display_order = COALESCE($3, display_order),
            is_active = COALESCE($4, is_active),
            receipt_name = COALESCE($5, receipt_name),
            kitchen_print_name = COALESCE($6, kitchen_print_name),
            enable_quantity = COALESCE($7, enable_quantity),
            max_quantity = COALESCE($8, max_quantity)
        WHERE source_id = $9
            AND attribute_id IN (SELECT id FROM store_attributes WHERE edge_server_id = $10)"#,
    )
    .bind(&data.name)
    .bind(data.price_modifier)
    .bind(data.display_order)
    .bind(data.is_active)
    .bind(&data.receipt_name)
    .bind(&data.kitchen_print_name)
    .bind(data.enable_quantity)
    .bind(data.max_quantity)
    .bind(option_source_id)
    .bind(edge_server_id)
    .execute(pool)
    .await?;
    if rows.rows_affected() == 0 {
        return Err("Attribute option not found".into());
    }
    Ok(())
}

pub async fn delete_option_direct(
    pool: &PgPool,
    edge_server_id: i64,
    option_source_id: i64,
) -> Result<(), BoxError> {
    let rows = sqlx::query(
        r#"DELETE FROM store_attribute_options
        WHERE source_id = $1
            AND attribute_id IN (SELECT id FROM store_attributes WHERE edge_server_id = $2)"#,
    )
    .bind(option_source_id)
    .bind(edge_server_id)
    .execute(pool)
    .await?;
    if rows.rows_affected() == 0 {
        return Err("Attribute option not found".into());
    }
    Ok(())
}

pub async fn batch_update_option_sort_order(
    pool: &PgPool,
    edge_server_id: i64,
    items: &[shared::cloud::store_op::SortOrderItem],
) -> Result<(), BoxError> {
    let mut tx = pool.begin().await?;
    for item in items {
        sqlx::query(
            r#"UPDATE store_attribute_options SET display_order = $1
            WHERE source_id = $2
                AND attribute_id IN (SELECT id FROM store_attributes WHERE edge_server_id = $3)"#,
        )
        .bind(item.sort_order)
        .bind(item.id)
        .bind(edge_server_id)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(())
}

// ── Binding CRUD ──

pub struct BindAttributeParams<'a> {
    pub owner_type: &'a str,
    pub owner_id: i64,
    pub attribute_id: i64,
    pub is_required: bool,
    pub display_order: i32,
    pub default_option_ids: Option<Vec<i32>>,
}

pub async fn bind_attribute_direct(
    pool: &PgPool,
    edge_server_id: i64,
    params: BindAttributeParams<'_>,
) -> Result<i64, BoxError> {
    let default_ids_json = params
        .default_option_ids
        .as_ref()
        .map(serde_json::to_value)
        .transpose()?;
    let mut tx = pool.begin().await?;
    let (pg_id,): (i64,) = sqlx::query_as("INSERT INTO store_attribute_bindings (edge_server_id, source_id, owner_type, owner_source_id, attribute_source_id, is_required, display_order, default_option_ids) VALUES ($1, 0, $2, $3, $4, $5, $6, $7) RETURNING id")
        .bind(edge_server_id).bind(params.owner_type).bind(params.owner_id).bind(params.attribute_id).bind(params.is_required).bind(params.display_order).bind(&default_ids_json)
        .fetch_one(&mut *tx).await?;
    let source_id = super::snowflake_id();
    sqlx::query("UPDATE store_attribute_bindings SET source_id = $1 WHERE id = $2")
        .bind(source_id)
        .bind(pg_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(source_id)
}

pub async fn unbind_attribute_direct(
    pool: &PgPool,
    edge_server_id: i64,
    binding_id: i64,
) -> Result<(), BoxError> {
    let rows = sqlx::query(
        "DELETE FROM store_attribute_bindings WHERE edge_server_id = $1 AND source_id = $2",
    )
    .bind(edge_server_id)
    .bind(binding_id)
    .execute(pool)
    .await?;
    if rows.rows_affected() == 0 {
        return Err("Attribute binding not found".into());
    }
    Ok(())
}
