//! Catalog data transfer types + import logic
//!
//! Uses `shared::models` types for ZIP format compatibility with edge-server.

use serde::{Deserialize, Serialize};
use shared::models::{Attribute, AttributeBinding, Category, ProductFull, Tag};
use sqlx::PgPool;

use super::BoxError;

/// Catalog export payload — the content of `catalog.json` inside the ZIP.
///
/// Uses `shared::models` types so the format matches edge-server exactly.
#[derive(Debug, Serialize, Deserialize)]
pub struct CatalogExport {
    pub version: u32,
    pub exported_at: i64,
    pub tags: Vec<Tag>,
    pub categories: Vec<Category>,
    pub products: Vec<ProductFull>,
    pub attributes: Vec<Attribute>,
    pub attribute_bindings: Vec<AttributeBinding>,
}

/// Delete all catalog data for a store and re-insert from the export.
///
/// Runs inside a single transaction. Uses `shared::models` field names (`id`, `category_id`).
pub async fn import_catalog(
    pool: &PgPool,
    store_id: i64,
    catalog: &CatalogExport,
) -> Result<(), BoxError> {
    let now = shared::util::now_millis();
    let mut tx = pool.begin().await?;

    // ── DELETE (FK reverse order) ──
    sqlx::query("DELETE FROM store_attribute_bindings WHERE store_id = $1")
        .bind(store_id)
        .execute(&mut *tx)
        .await?;

    sqlx::query(
        r#"DELETE FROM store_attribute_options
        WHERE attribute_id IN (SELECT id FROM store_attributes WHERE store_id = $1)"#,
    )
    .bind(store_id)
    .execute(&mut *tx)
    .await?;

    sqlx::query("DELETE FROM store_attributes WHERE store_id = $1")
        .bind(store_id)
        .execute(&mut *tx)
        .await?;

    sqlx::query(
        r#"DELETE FROM store_product_specs
        WHERE product_id IN (SELECT id FROM store_products WHERE store_id = $1)"#,
    )
    .bind(store_id)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        r#"DELETE FROM store_product_tag
        WHERE product_id IN (SELECT id FROM store_products WHERE store_id = $1)"#,
    )
    .bind(store_id)
    .execute(&mut *tx)
    .await?;

    sqlx::query("DELETE FROM store_products WHERE store_id = $1")
        .bind(store_id)
        .execute(&mut *tx)
        .await?;

    sqlx::query(
        r#"DELETE FROM store_category_print_dest
        WHERE category_id IN (SELECT id FROM store_categories WHERE store_id = $1)"#,
    )
    .bind(store_id)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        r#"DELETE FROM store_category_tag
        WHERE category_id IN (SELECT id FROM store_categories WHERE store_id = $1)"#,
    )
    .bind(store_id)
    .execute(&mut *tx)
    .await?;

    sqlx::query("DELETE FROM store_categories WHERE store_id = $1")
        .bind(store_id)
        .execute(&mut *tx)
        .await?;

    sqlx::query("DELETE FROM store_tags WHERE store_id = $1")
        .bind(store_id)
        .execute(&mut *tx)
        .await?;

    // ── INSERT tags ──
    for tag in &catalog.tags {
        sqlx::query(
            r#"INSERT INTO store_tags (store_id, source_id, name, color, display_order, is_active, is_system, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)"#,
        )
        .bind(store_id)
        .bind(tag.id)
        .bind(&tag.name)
        .bind(&tag.color)
        .bind(tag.display_order)
        .bind(tag.is_active)
        .bind(tag.is_system)
        .bind(now)
        .execute(&mut *tx)
        .await?;
    }

    // ── INSERT categories ──
    for cat in &catalog.categories {
        let (pg_id,): (i64,) = sqlx::query_as(
            r#"INSERT INTO store_categories (
                store_id, source_id, name, sort_order,
                is_kitchen_print_enabled, is_label_print_enabled,
                is_active, is_virtual, match_mode, is_display, updated_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11) RETURNING id"#,
        )
        .bind(store_id)
        .bind(cat.id)
        .bind(&cat.name)
        .bind(cat.sort_order)
        .bind(cat.is_kitchen_print_enabled)
        .bind(cat.is_label_print_enabled)
        .bind(cat.is_active)
        .bind(cat.is_virtual)
        .bind(&cat.match_mode)
        .bind(cat.is_display)
        .bind(now)
        .fetch_one(&mut *tx)
        .await?;

        for dest_id in &cat.kitchen_print_destinations {
            sqlx::query(
                "INSERT INTO store_category_print_dest (category_id, dest_source_id, purpose) VALUES ($1, $2, 'kitchen')",
            )
            .bind(pg_id)
            .bind(dest_id)
            .execute(&mut *tx)
            .await?;
        }
        for dest_id in &cat.label_print_destinations {
            sqlx::query(
                "INSERT INTO store_category_print_dest (category_id, dest_source_id, purpose) VALUES ($1, $2, 'label')",
            )
            .bind(pg_id)
            .bind(dest_id)
            .execute(&mut *tx)
            .await?;
        }
        for tag_id in &cat.tag_ids {
            sqlx::query(
                "INSERT INTO store_category_tag (category_id, tag_source_id) VALUES ($1, $2)",
            )
            .bind(pg_id)
            .bind(tag_id)
            .execute(&mut *tx)
            .await?;
        }
    }

    // ── INSERT products ──
    for product in &catalog.products {
        let (pg_id,): (i64,) = sqlx::query_as(
            r#"INSERT INTO store_products (
                store_id, source_id, name, image, category_source_id,
                sort_order, tax_rate, receipt_name, kitchen_print_name,
                is_kitchen_print_enabled, is_label_print_enabled,
                is_active, external_id, updated_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14) RETURNING id"#,
        )
        .bind(store_id)
        .bind(product.id)
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
        .fetch_one(&mut *tx)
        .await?;

        for spec in &product.specs {
            sqlx::query(
                r#"INSERT INTO store_product_specs (
                    product_id, source_id, name, price, display_order,
                    is_default, is_active, receipt_name, is_root
                ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)"#,
            )
            .bind(pg_id)
            .bind(spec.id)
            .bind(&spec.name)
            .bind(spec.price)
            .bind(spec.display_order)
            .bind(spec.is_default)
            .bind(spec.is_active)
            .bind(&spec.receipt_name)
            .bind(spec.is_root)
            .execute(&mut *tx)
            .await?;
        }

        for tag in &product.tags {
            sqlx::query(
                "INSERT INTO store_product_tag (product_id, tag_source_id) VALUES ($1, $2)",
            )
            .bind(pg_id)
            .bind(tag.id)
            .execute(&mut *tx)
            .await?;
        }
    }

    // ── INSERT attributes ──
    for attr in &catalog.attributes {
        let default_ids_json = attr
            .default_option_ids
            .as_ref()
            .map(serde_json::to_value)
            .transpose()?;

        let (pg_id,): (i64,) = sqlx::query_as(
            r#"INSERT INTO store_attributes (
                store_id, source_id, name, is_multi_select, max_selections,
                default_option_ids, display_order, is_active,
                show_on_receipt, receipt_name, show_on_kitchen_print, kitchen_print_name,
                updated_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13) RETURNING id"#,
        )
        .bind(store_id)
        .bind(attr.id)
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
        .fetch_one(&mut *tx)
        .await?;

        for opt in &attr.options {
            sqlx::query(
                r#"INSERT INTO store_attribute_options (
                    attribute_id, source_id, name, price_modifier, display_order,
                    is_active, receipt_name, kitchen_print_name, enable_quantity, max_quantity
                ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)"#,
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
    }

    // ── INSERT attribute_bindings ──
    for binding in &catalog.attribute_bindings {
        let default_ids_json = binding
            .default_option_ids
            .as_ref()
            .map(serde_json::to_value)
            .transpose()?;

        sqlx::query(
            r#"INSERT INTO store_attribute_bindings (
                store_id, source_id, owner_type, owner_source_id,
                attribute_source_id, is_required, display_order, default_option_ids
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)"#,
        )
        .bind(store_id)
        .bind(binding.id)
        .bind(&binding.owner_type)
        .bind(binding.owner_id)
        .bind(binding.attribute_id)
        .bind(binding.is_required)
        .bind(binding.display_order)
        .bind(&default_ids_json)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    Ok(())
}
