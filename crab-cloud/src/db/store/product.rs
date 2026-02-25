//! Product database operations

use serde::{Deserialize, Serialize};
use shared::cloud::store_op::StoreOpData;
use shared::models::product::{Product, ProductCreate, ProductFull, ProductSpec};
use sqlx::PgPool;

use super::BoxError;

// ── Edge Sync ──

pub async fn upsert_product_from_sync(
    pool: &PgPool,
    edge_server_id: i64,
    source_id: i64,
    data: &serde_json::Value,
    _version: i64,
    now: i64,
) -> Result<(), BoxError> {
    let product: Product = serde_json::from_value(data.clone())?;
    let specs: Vec<ProductSpec> =
        serde_json::from_value(data.get("specs").cloned().unwrap_or(serde_json::json!([])))?;
    let tags: Vec<serde_json::Value> =
        serde_json::from_value(data.get("tags").cloned().unwrap_or(serde_json::json!([])))?;

    let mut tx = pool.begin().await?;

    let row: Option<(i64,)> = sqlx::query_as(
        r#"
        INSERT INTO store_products (
            edge_server_id, source_id, name, image, category_source_id,
            sort_order, tax_rate, receipt_name, kitchen_print_name,
            is_kitchen_print_enabled, is_label_print_enabled,
            is_active, external_id, updated_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
        ON CONFLICT (edge_server_id, source_id)
        DO UPDATE SET
            name = EXCLUDED.name, image = EXCLUDED.image,
            category_source_id = EXCLUDED.category_source_id,
            sort_order = EXCLUDED.sort_order, tax_rate = EXCLUDED.tax_rate,
            receipt_name = EXCLUDED.receipt_name, kitchen_print_name = EXCLUDED.kitchen_print_name,
            is_kitchen_print_enabled = EXCLUDED.is_kitchen_print_enabled,
            is_label_print_enabled = EXCLUDED.is_label_print_enabled,
            is_active = EXCLUDED.is_active, external_id = EXCLUDED.external_id,
            updated_at = EXCLUDED.updated_at
        WHERE store_products.updated_at <= EXCLUDED.updated_at
        RETURNING id
        "#,
    )
    .bind(edge_server_id)
    .bind(source_id)
    .bind(&product.name)
    .bind(&product.image)
    .bind(product.category_id)
    .bind(product.sort_order)
    .bind(product.tax_rate)
    .bind(&product.receipt_name)
    .bind(&product.kitchen_print_name)
    .bind(product.is_kitchen_print_enabled)
    .bind(product.is_label_print_enabled)
    .bind(product.is_active)
    .bind(product.external_id)
    .bind(now)
    .fetch_optional(&mut *tx)
    .await?;

    let Some((pg_id,)) = row else {
        tx.commit().await?;
        return Ok(());
    };

    // Replace specs
    sqlx::query("DELETE FROM store_product_specs WHERE product_id = $1")
        .bind(pg_id)
        .execute(&mut *tx)
        .await?;

    if !specs.is_empty() {
        let product_ids: Vec<i64> = specs.iter().map(|_| pg_id).collect();
        let source_ids: Vec<i64> = specs.iter().map(|s| s.id).collect();
        let names: Vec<String> = specs.iter().map(|s| s.name.clone()).collect();
        let prices: Vec<f64> = specs.iter().map(|s| s.price).collect();
        let display_orders: Vec<i32> = specs.iter().map(|s| s.display_order).collect();
        let is_defaults: Vec<bool> = specs.iter().map(|s| s.is_default).collect();
        let is_actives: Vec<bool> = specs.iter().map(|s| s.is_active).collect();
        let receipt_names: Vec<Option<String>> =
            specs.iter().map(|s| s.receipt_name.clone()).collect();
        let is_roots: Vec<bool> = specs.iter().map(|s| s.is_root).collect();
        sqlx::query(
            r#"
            INSERT INTO store_product_specs (
                product_id, source_id, name, price, display_order,
                is_default, is_active, receipt_name, is_root
            )
            SELECT * FROM UNNEST($1::bigint[], $2::bigint[], $3::text[], $4::double precision[], $5::integer[], $6::boolean[], $7::boolean[], $8::text[], $9::boolean[])
            "#,
        )
        .bind(&product_ids)
        .bind(&source_ids)
        .bind(&names)
        .bind(&prices)
        .bind(&display_orders)
        .bind(&is_defaults)
        .bind(&is_actives)
        .bind(&receipt_names)
        .bind(&is_roots)
        .execute(&mut *tx)
        .await?;
    }

    // Replace product tags
    sqlx::query("DELETE FROM store_product_tag WHERE product_id = $1")
        .bind(pg_id)
        .execute(&mut *tx)
        .await?;

    let tag_ids: Vec<i64> = tags
        .iter()
        .filter_map(|tag_val| {
            tag_val
                .as_i64()
                .or_else(|| tag_val.get("id").and_then(|v| v.as_i64()))
        })
        .collect();
    if !tag_ids.is_empty() {
        let product_ids: Vec<i64> = tag_ids.iter().map(|_| pg_id).collect();
        sqlx::query(
            "INSERT INTO store_product_tag (product_id, tag_source_id) SELECT * FROM UNNEST($1::bigint[], $2::bigint[]) ON CONFLICT DO NOTHING",
        )
        .bind(&product_ids)
        .bind(&tag_ids)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    Ok(())
}

pub async fn delete_product(
    pool: &PgPool,
    edge_server_id: i64,
    source_id: i64,
) -> Result<(), BoxError> {
    sqlx::query("DELETE FROM store_products WHERE edge_server_id = $1 AND source_id = $2")
        .bind(edge_server_id)
        .bind(source_id)
        .execute(pool)
        .await?;
    Ok(())
}

// ── Console Read Types ──

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct StoreProductRow {
    pub id: i64,
    pub source_id: i64,
    pub name: String,
    pub image: String,
    pub category_source_id: i64,
    pub sort_order: i32,
    pub tax_rate: i32,
    pub receipt_name: Option<String>,
    pub kitchen_print_name: Option<String>,
    pub is_kitchen_print_enabled: i32,
    pub is_label_print_enabled: i32,
    pub is_active: bool,
    pub external_id: Option<i64>,
    pub updated_at: i64,
    pub category_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct StoreProductSpecRow {
    pub id: i64,
    pub product_id: i64,
    pub source_id: i64,
    pub name: String,
    pub price: f64,
    pub display_order: i32,
    pub is_default: bool,
    pub is_active: bool,
    pub receipt_name: Option<String>,
    pub is_root: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreProduct {
    pub source_id: i64,
    pub name: String,
    pub image: String,
    pub category_source_id: i64,
    pub category_name: Option<String>,
    pub sort_order: i32,
    pub tax_rate: i32,
    pub receipt_name: Option<String>,
    pub kitchen_print_name: Option<String>,
    pub is_kitchen_print_enabled: i32,
    pub is_label_print_enabled: i32,
    pub is_active: bool,
    pub external_id: Option<i64>,
    pub specs: Vec<StoreSpec>,
    pub tag_ids: Vec<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreSpec {
    pub source_id: i64,
    pub name: String,
    pub price: f64,
    pub display_order: i32,
    pub is_default: bool,
    pub is_active: bool,
    pub receipt_name: Option<String>,
    pub is_root: bool,
}

// ── Console Read ──

pub async fn list_products(
    pool: &PgPool,
    edge_server_id: i64,
) -> Result<Vec<StoreProduct>, BoxError> {
    let rows: Vec<StoreProductRow> = sqlx::query_as(
        r#"
        SELECT p.id, p.source_id, p.name, p.image, p.category_source_id,
               p.sort_order, p.tax_rate, p.receipt_name, p.kitchen_print_name,
               p.is_kitchen_print_enabled, p.is_label_print_enabled,
               p.is_active, p.external_id, p.updated_at,
               c.name AS category_name
        FROM store_products p
        LEFT JOIN store_categories c
            ON c.edge_server_id = p.edge_server_id AND c.source_id = p.category_source_id
        WHERE p.edge_server_id = $1
        ORDER BY c.sort_order, p.sort_order, p.source_id
        "#,
    )
    .bind(edge_server_id)
    .fetch_all(pool)
    .await?;

    let pg_ids: Vec<i64> = rows.iter().map(|r| r.id).collect();
    if pg_ids.is_empty() {
        return Ok(vec![]);
    }

    let specs: Vec<StoreProductSpecRow> = sqlx::query_as(
        r#"
        SELECT id, product_id, source_id, name, price, display_order,
               is_default, is_active, receipt_name, is_root
        FROM store_product_specs
        WHERE product_id = ANY($1)
        ORDER BY display_order
        "#,
    )
    .bind(&pg_ids)
    .fetch_all(pool)
    .await?;

    let tag_rows: Vec<(i64, i64)> = sqlx::query_as(
        "SELECT product_id, tag_source_id FROM store_product_tag WHERE product_id = ANY($1)",
    )
    .bind(&pg_ids)
    .fetch_all(pool)
    .await?;

    let mut spec_map: std::collections::HashMap<i64, Vec<StoreSpec>> =
        std::collections::HashMap::new();
    for s in specs {
        spec_map.entry(s.product_id).or_default().push(StoreSpec {
            source_id: s.source_id,
            name: s.name,
            price: s.price,
            display_order: s.display_order,
            is_default: s.is_default,
            is_active: s.is_active,
            receipt_name: s.receipt_name,
            is_root: s.is_root,
        });
    }

    let mut tag_map: std::collections::HashMap<i64, Vec<i64>> = std::collections::HashMap::new();
    for (product_id, tag_id) in tag_rows {
        tag_map.entry(product_id).or_default().push(tag_id);
    }

    Ok(rows
        .into_iter()
        .map(|r| StoreProduct {
            specs: spec_map.remove(&r.id).unwrap_or_default(),
            tag_ids: tag_map.remove(&r.id).unwrap_or_default(),
            source_id: r.source_id,
            name: r.name,
            image: r.image,
            category_source_id: r.category_source_id,
            category_name: r.category_name,
            sort_order: r.sort_order,
            tax_rate: r.tax_rate,
            receipt_name: r.receipt_name,
            kitchen_print_name: r.kitchen_print_name,
            is_kitchen_print_enabled: r.is_kitchen_print_enabled,
            is_label_print_enabled: r.is_label_print_enabled,
            is_active: r.is_active,
            external_id: r.external_id,
        })
        .collect())
}

// ── Console CRUD ──

pub async fn create_product_direct(
    pool: &PgPool,
    edge_server_id: i64,
    data: &ProductCreate,
) -> Result<(i64, StoreOpData), BoxError> {
    let now = shared::util::now_millis();
    let image = data.image.as_deref().unwrap_or("");
    let sort_order = data.sort_order.unwrap_or(0);
    let tax_rate = data.tax_rate.unwrap_or(0);
    let is_kitchen_print_enabled = data.is_kitchen_print_enabled.unwrap_or(-1);
    let is_label_print_enabled = data.is_label_print_enabled.unwrap_or(-1);

    let source_id = super::snowflake_id();
    let mut tx = pool.begin().await?;

    let (pg_id,): (i64,) = sqlx::query_as(
        r#"
        INSERT INTO store_products (
            edge_server_id, source_id, name, image, category_source_id,
            sort_order, tax_rate, receipt_name, kitchen_print_name,
            is_kitchen_print_enabled, is_label_print_enabled,
            is_active, external_id, updated_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, TRUE, $12, $13)
        RETURNING id
        "#,
    )
    .bind(edge_server_id)
    .bind(source_id)
    .bind(&data.name)
    .bind(image)
    .bind(data.category_id)
    .bind(sort_order)
    .bind(tax_rate)
    .bind(&data.receipt_name)
    .bind(&data.kitchen_print_name)
    .bind(is_kitchen_print_enabled)
    .bind(is_label_print_enabled)
    .bind(data.external_id)
    .bind(now)
    .fetch_one(&mut *tx)
    .await?;

    if !data.specs.is_empty() {
        let product_ids: Vec<i64> = data.specs.iter().map(|_| pg_id).collect();
        let spec_source_ids: Vec<i64> = data.specs.iter().map(|_| super::snowflake_id()).collect();
        let names: Vec<String> = data.specs.iter().map(|s| s.name.clone()).collect();
        let prices: Vec<f64> = data.specs.iter().map(|s| s.price).collect();
        let display_orders: Vec<i32> = data.specs.iter().map(|s| s.display_order).collect();
        let is_defaults: Vec<bool> = data.specs.iter().map(|s| s.is_default).collect();
        let is_actives: Vec<bool> = data.specs.iter().map(|s| s.is_active).collect();
        let receipt_names: Vec<Option<String>> =
            data.specs.iter().map(|s| s.receipt_name.clone()).collect();
        let is_roots: Vec<bool> = data.specs.iter().map(|s| s.is_root).collect();
        sqlx::query(
            r#"
            INSERT INTO store_product_specs (
                product_id, source_id, name, price, display_order,
                is_default, is_active, receipt_name, is_root
            )
            SELECT * FROM UNNEST($1::bigint[], $2::bigint[], $3::text[], $4::double precision[], $5::integer[], $6::boolean[], $7::boolean[], $8::text[], $9::boolean[])
            "#,
        )
        .bind(&product_ids)
        .bind(&spec_source_ids)
        .bind(&names)
        .bind(&prices)
        .bind(&display_orders)
        .bind(&is_defaults)
        .bind(&is_actives)
        .bind(&receipt_names)
        .bind(&is_roots)
        .execute(&mut *tx)
        .await?;
    }

    if let Some(ref tags) = data.tags
        && !tags.is_empty()
    {
        let product_ids: Vec<i64> = tags.iter().map(|_| pg_id).collect();
        sqlx::query(
                "INSERT INTO store_product_tag (product_id, tag_source_id) SELECT * FROM UNNEST($1::bigint[], $2::bigint[]) ON CONFLICT DO NOTHING",
            )
            .bind(&product_ids)
            .bind(tags)
            .execute(&mut *tx)
            .await?;
    }

    let spec_rows: Vec<StoreProductSpecRow> = sqlx::query_as(
        "SELECT id, product_id, source_id, name, price, display_order, is_default, is_active, receipt_name, is_root FROM store_product_specs WHERE product_id = $1 ORDER BY display_order",
    )
    .bind(pg_id)
    .fetch_all(&mut *tx)
    .await?;

    tx.commit().await?;

    let specs: Vec<ProductSpec> = spec_rows
        .into_iter()
        .map(|r| ProductSpec {
            id: r.source_id,
            product_id: source_id,
            name: r.name,
            price: r.price,
            display_order: r.display_order,
            is_default: r.is_default,
            is_active: r.is_active,
            receipt_name: r.receipt_name,
            is_root: r.is_root,
        })
        .collect();

    let product_full = ProductFull {
        id: source_id,
        name: data.name.clone(),
        image: data.image.clone().unwrap_or_default(),
        category_id: data.category_id,
        sort_order: data.sort_order.unwrap_or(0),
        tax_rate: data.tax_rate.unwrap_or(0),
        receipt_name: data.receipt_name.clone(),
        kitchen_print_name: data.kitchen_print_name.clone(),
        is_kitchen_print_enabled: data.is_kitchen_print_enabled.unwrap_or(-1),
        is_label_print_enabled: data.is_label_print_enabled.unwrap_or(-1),
        is_active: true,
        external_id: data.external_id,
        specs,
        attributes: vec![],
        tags: vec![],
    };
    Ok((source_id, StoreOpData::Product(product_full)))
}

pub async fn update_product_direct(
    pool: &PgPool,
    edge_server_id: i64,
    source_id: i64,
    data: &shared::models::product::ProductUpdate,
) -> Result<(), BoxError> {
    let now = shared::util::now_millis();
    let mut tx = pool.begin().await?;

    let pg_id: i64 = sqlx::query_scalar(
        "SELECT id FROM store_products WHERE edge_server_id = $1 AND source_id = $2",
    )
    .bind(edge_server_id)
    .bind(source_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or("Product not found")?;

    sqlx::query(
        r#"
        UPDATE store_products SET
            name = COALESCE($1, name),
            image = COALESCE($2, image),
            category_source_id = COALESCE($3, category_source_id),
            sort_order = COALESCE($4, sort_order),
            tax_rate = COALESCE($5, tax_rate),
            receipt_name = COALESCE($6, receipt_name),
            kitchen_print_name = COALESCE($7, kitchen_print_name),
            is_kitchen_print_enabled = COALESCE($8, is_kitchen_print_enabled),
            is_label_print_enabled = COALESCE($9, is_label_print_enabled),
            is_active = COALESCE($10, is_active),
            external_id = COALESCE($11, external_id),
            updated_at = $12
        WHERE id = $13
        "#,
    )
    .bind(&data.name)
    .bind(&data.image)
    .bind(data.category_id)
    .bind(data.sort_order)
    .bind(data.tax_rate)
    .bind(&data.receipt_name)
    .bind(&data.kitchen_print_name)
    .bind(data.is_kitchen_print_enabled)
    .bind(data.is_label_print_enabled)
    .bind(data.is_active)
    .bind(data.external_id)
    .bind(now)
    .bind(pg_id)
    .execute(&mut *tx)
    .await?;

    if let Some(ref specs) = data.specs {
        sqlx::query("DELETE FROM store_product_specs WHERE product_id = $1")
            .bind(pg_id)
            .execute(&mut *tx)
            .await?;
        if !specs.is_empty() {
            let product_ids: Vec<i64> = specs.iter().map(|_| pg_id).collect();
            let spec_source_ids: Vec<i64> = specs.iter().map(|_| super::snowflake_id()).collect();
            let names: Vec<String> = specs.iter().map(|s| s.name.clone()).collect();
            let prices: Vec<f64> = specs.iter().map(|s| s.price).collect();
            let display_orders: Vec<i32> = specs.iter().map(|s| s.display_order).collect();
            let is_defaults: Vec<bool> = specs.iter().map(|s| s.is_default).collect();
            let is_actives: Vec<bool> = specs.iter().map(|s| s.is_active).collect();
            let receipt_names: Vec<Option<String>> =
                specs.iter().map(|s| s.receipt_name.clone()).collect();
            let is_roots: Vec<bool> = specs.iter().map(|s| s.is_root).collect();
            sqlx::query(
                r#"
                INSERT INTO store_product_specs (
                    product_id, source_id, name, price, display_order,
                    is_default, is_active, receipt_name, is_root
                )
                SELECT * FROM UNNEST($1::bigint[], $2::bigint[], $3::text[], $4::double precision[], $5::integer[], $6::boolean[], $7::boolean[], $8::text[], $9::boolean[])
                "#,
            )
            .bind(&product_ids)
            .bind(&spec_source_ids)
            .bind(&names)
            .bind(&prices)
            .bind(&display_orders)
            .bind(&is_defaults)
            .bind(&is_actives)
            .bind(&receipt_names)
            .bind(&is_roots)
            .execute(&mut *tx)
            .await?;
        }
    }

    if let Some(ref tags) = data.tags {
        sqlx::query("DELETE FROM store_product_tag WHERE product_id = $1")
            .bind(pg_id)
            .execute(&mut *tx)
            .await?;
        if !tags.is_empty() {
            let product_ids: Vec<i64> = tags.iter().map(|_| pg_id).collect();
            sqlx::query(
                "INSERT INTO store_product_tag (product_id, tag_source_id) SELECT * FROM UNNEST($1::bigint[], $2::bigint[]) ON CONFLICT DO NOTHING",
            )
            .bind(&product_ids)
            .bind(tags)
            .execute(&mut *tx)
            .await?;
        }
    }

    tx.commit().await?;
    Ok(())
}

pub async fn batch_update_sort_order_products(
    pool: &PgPool,
    edge_server_id: i64,
    items: &[shared::cloud::store_op::SortOrderItem],
) -> Result<(), BoxError> {
    if items.is_empty() {
        return Ok(());
    }
    let now = shared::util::now_millis();
    let ids: Vec<i64> = items.iter().map(|i| i.id).collect();
    let orders: Vec<i32> = items.iter().map(|i| i.sort_order).collect();
    let nows: Vec<i64> = items.iter().map(|_| now).collect();
    sqlx::query(
        r#"UPDATE store_products SET sort_order = u.sort_order, updated_at = u.updated_at
        FROM (SELECT * FROM UNNEST($1::bigint[], $2::integer[], $3::bigint[])) AS u(source_id, sort_order, updated_at)
        WHERE store_products.edge_server_id = $4 AND store_products.source_id = u.source_id"#,
    )
    .bind(&ids)
    .bind(&orders)
    .bind(&nows)
    .bind(edge_server_id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn bulk_delete_products(
    pool: &PgPool,
    edge_server_id: i64,
    source_ids: &[i64],
) -> Result<(), BoxError> {
    if source_ids.is_empty() {
        return Ok(());
    }
    sqlx::query("DELETE FROM store_products WHERE edge_server_id = $1 AND source_id = ANY($2)")
        .bind(edge_server_id)
        .bind(source_ids)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn delete_product_direct(
    pool: &PgPool,
    edge_server_id: i64,
    source_id: i64,
) -> Result<(), BoxError> {
    let rows =
        sqlx::query("DELETE FROM store_products WHERE edge_server_id = $1 AND source_id = $2")
            .bind(edge_server_id)
            .bind(source_id)
            .execute(pool)
            .await?;
    if rows.rows_affected() == 0 {
        return Err("Product not found".into());
    }
    Ok(())
}
