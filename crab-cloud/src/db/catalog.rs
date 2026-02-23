//! Catalog normalized storage — CRUD for products, categories, attributes, tags
//!
//! Two write paths:
//! 1. Edge sync: upsert_product_from_sync / upsert_category_from_sync (edge→cloud)
//! 2. Console CRUD: create/update/delete (cloud→PG, then RPC to edge)

use serde::{Deserialize, Serialize};
use shared::models::{
    attribute::{Attribute, AttributeBinding},
    category::Category,
    dining_table::DiningTable,
    employee::Employee,
    product::{Product, ProductSpec},
    tag::Tag,
    zone::Zone,
};
use sqlx::PgPool;

type BoxError = Box<dyn std::error::Error + Send + Sync>;

// ════════════════════════════════════════════════════════════════
// Edge Sync writes (edge → cloud normalized tables)
// ════════════════════════════════════════════════════════════════

/// Upsert product from edge sync data (ProductFull JSON → normalized tables)
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

    // Upsert product
    let row: Option<(i64,)> = sqlx::query_as(
        r#"
        INSERT INTO catalog_products (
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
    sqlx::query("DELETE FROM catalog_product_specs WHERE product_id = $1")
        .bind(pg_id)
        .execute(&mut *tx)
        .await?;

    for spec in &specs {
        sqlx::query(
            r#"
            INSERT INTO catalog_product_specs (
                product_id, source_id, name, price, display_order,
                is_default, is_active, receipt_name, is_root
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            "#,
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

    // Replace product tags
    sqlx::query("DELETE FROM catalog_product_tag WHERE product_id = $1")
        .bind(pg_id)
        .execute(&mut *tx)
        .await?;

    // Tags can be either full Tag objects or bare i64 IDs
    for tag_val in &tags {
        let tag_id = if let Some(id) = tag_val.as_i64() {
            id
        } else if let Some(id) = tag_val.get("id").and_then(|v| v.as_i64()) {
            id
        } else {
            continue;
        };
        sqlx::query(
            "INSERT INTO catalog_product_tag (product_id, tag_source_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
        )
        .bind(pg_id)
        .bind(tag_id)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    Ok(())
}

/// Upsert category from edge sync data (Category JSON → normalized tables)
pub async fn upsert_category_from_sync(
    pool: &PgPool,
    edge_server_id: i64,
    source_id: i64,
    data: &serde_json::Value,
    _version: i64,
    now: i64,
) -> Result<(), BoxError> {
    let cat: Category = serde_json::from_value(data.clone())?;

    let mut tx = pool.begin().await?;

    let row: Option<(i64,)> = sqlx::query_as(
        r#"
        INSERT INTO catalog_categories (
            edge_server_id, source_id, name, sort_order,
            is_kitchen_print_enabled, is_label_print_enabled,
            is_active, is_virtual, match_mode, is_display, updated_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
        ON CONFLICT (edge_server_id, source_id)
        DO UPDATE SET
            name = EXCLUDED.name, sort_order = EXCLUDED.sort_order,
            is_kitchen_print_enabled = EXCLUDED.is_kitchen_print_enabled,
            is_label_print_enabled = EXCLUDED.is_label_print_enabled,
            is_active = EXCLUDED.is_active, is_virtual = EXCLUDED.is_virtual,
            match_mode = EXCLUDED.match_mode, is_display = EXCLUDED.is_display,
            updated_at = EXCLUDED.updated_at
        RETURNING id
        "#,
    )
    .bind(edge_server_id)
    .bind(source_id)
    .bind(&cat.name)
    .bind(cat.sort_order)
    .bind(cat.is_kitchen_print_enabled)
    .bind(cat.is_label_print_enabled)
    .bind(cat.is_active)
    .bind(cat.is_virtual)
    .bind(&cat.match_mode)
    .bind(cat.is_display)
    .bind(now)
    .fetch_optional(&mut *tx)
    .await?;

    let Some((pg_id,)) = row else {
        tx.commit().await?;
        return Ok(());
    };

    // Replace print destinations
    sqlx::query("DELETE FROM catalog_category_print_dest WHERE category_id = $1")
        .bind(pg_id)
        .execute(&mut *tx)
        .await?;

    for dest_id in &cat.kitchen_print_destinations {
        sqlx::query(
            "INSERT INTO catalog_category_print_dest (category_id, dest_source_id, purpose) VALUES ($1, $2, 'kitchen')",
        )
        .bind(pg_id)
        .bind(dest_id)
        .execute(&mut *tx)
        .await?;
    }
    for dest_id in &cat.label_print_destinations {
        sqlx::query(
            "INSERT INTO catalog_category_print_dest (category_id, dest_source_id, purpose) VALUES ($1, $2, 'label')",
        )
        .bind(pg_id)
        .bind(dest_id)
        .execute(&mut *tx)
        .await?;
    }

    // Replace tag associations
    sqlx::query("DELETE FROM catalog_category_tag WHERE category_id = $1")
        .bind(pg_id)
        .execute(&mut *tx)
        .await?;

    for tag_id in &cat.tag_ids {
        sqlx::query(
            "INSERT INTO catalog_category_tag (category_id, tag_source_id) VALUES ($1, $2)",
        )
        .bind(pg_id)
        .bind(tag_id)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    Ok(())
}

/// Delete product from normalized tables
pub async fn delete_product(
    pool: &PgPool,
    edge_server_id: i64,
    source_id: i64,
) -> Result<(), BoxError> {
    sqlx::query("DELETE FROM catalog_products WHERE edge_server_id = $1 AND source_id = $2")
        .bind(edge_server_id)
        .bind(source_id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Delete category from normalized tables
pub async fn delete_category(
    pool: &PgPool,
    edge_server_id: i64,
    source_id: i64,
) -> Result<(), BoxError> {
    sqlx::query("DELETE FROM catalog_categories WHERE edge_server_id = $1 AND source_id = $2")
        .bind(edge_server_id)
        .bind(source_id)
        .execute(pool)
        .await?;
    Ok(())
}

// ════════════════════════════════════════════════════════════════
// Console reads (PG → API response)
// ════════════════════════════════════════════════════════════════

/// Product entry for console API (replaces JSONB ProductEntry)
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct CatalogProductRow {
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
    // joined
    pub category_name: Option<String>,
}

/// Product spec for console API
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct CatalogProductSpecRow {
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

/// Console product response (assembled from normalized data)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogProduct {
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
    pub specs: Vec<CatalogSpec>,
    pub tag_ids: Vec<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogSpec {
    pub source_id: i64,
    pub name: String,
    pub price: f64,
    pub display_order: i32,
    pub is_default: bool,
    pub is_active: bool,
    pub receipt_name: Option<String>,
    pub is_root: bool,
}

/// Console category response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogCategory {
    pub source_id: i64,
    pub name: String,
    pub sort_order: i32,
    pub is_kitchen_print_enabled: bool,
    pub is_label_print_enabled: bool,
    pub is_active: bool,
    pub is_virtual: bool,
    pub match_mode: String,
    pub is_display: bool,
    pub kitchen_print_destinations: Vec<i64>,
    pub label_print_destinations: Vec<i64>,
    pub tag_ids: Vec<i64>,
}

/// Console tag response
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct CatalogTag {
    pub source_id: i64,
    pub name: String,
    pub color: String,
    pub display_order: i32,
    pub is_active: bool,
    pub is_system: bool,
}

/// Console attribute response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogAttribute {
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
    pub options: Vec<CatalogAttributeOption>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogAttributeOption {
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

/// List price rules for a store (console read) — returns shared::models::PriceRule
pub async fn list_price_rules(
    pool: &PgPool,
    edge_server_id: i64,
) -> Result<Vec<shared::models::PriceRule>, BoxError> {
    let rows = sqlx::query_as::<_, PriceRuleRow>(
        r#"
        SELECT source_id, name, display_name, receipt_name, description,
               rule_type, product_scope, target_id, zone_scope,
               adjustment_type, adjustment_value, is_stackable, is_exclusive,
               valid_from, valid_until, active_days, active_start_time, active_end_time,
               is_active, created_by, created_at
        FROM catalog_price_rules
        WHERE edge_server_id = $1
        ORDER BY created_at DESC
        "#,
    )
    .bind(edge_server_id)
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(|r| r.into_price_rule()).collect())
}

/// PG row → shared PriceRule mapping
#[derive(sqlx::FromRow)]
struct PriceRuleRow {
    source_id: i64,
    name: String,
    display_name: String,
    receipt_name: String,
    description: Option<String>,
    rule_type: String,
    product_scope: String,
    target_id: Option<i64>,
    zone_scope: String,
    adjustment_type: String,
    adjustment_value: f64,
    is_stackable: bool,
    is_exclusive: bool,
    valid_from: Option<i64>,
    valid_until: Option<i64>,
    active_days: Option<i32>,
    active_start_time: Option<String>,
    active_end_time: Option<String>,
    is_active: bool,
    created_by: Option<i64>,
    created_at: i64,
}

impl PriceRuleRow {
    fn into_price_rule(self) -> shared::models::PriceRule {
        use shared::models::price_rule::{AdjustmentType, ProductScope, RuleType};

        shared::models::PriceRule {
            id: self.source_id,
            name: self.name,
            display_name: self.display_name,
            receipt_name: self.receipt_name,
            description: self.description,
            rule_type: serde_json::from_value::<RuleType>(serde_json::Value::String(
                self.rule_type.clone(),
            ))
            .unwrap_or_else(|e| {
                tracing::warn!(rule_type = %self.rule_type, error = %e, "Invalid rule_type, defaulting to Discount");
                RuleType::Discount
            }),
            product_scope: serde_json::from_value::<ProductScope>(serde_json::Value::String(
                self.product_scope.clone(),
            ))
            .unwrap_or_else(|e| {
                tracing::warn!(product_scope = %self.product_scope, error = %e, "Invalid product_scope, defaulting to Global");
                ProductScope::Global
            }),
            target_id: self.target_id,
            zone_scope: self.zone_scope,
            adjustment_type: serde_json::from_value::<AdjustmentType>(
                serde_json::Value::String(self.adjustment_type.clone()),
            )
            .unwrap_or_else(|e| {
                tracing::warn!(adjustment_type = %self.adjustment_type, error = %e, "Invalid adjustment_type, defaulting to Percentage");
                AdjustmentType::Percentage
            }),
            adjustment_value: self.adjustment_value,
            is_stackable: self.is_stackable,
            is_exclusive: self.is_exclusive,
            valid_from: self.valid_from,
            valid_until: self.valid_until,
            active_days: self.active_days.map(|mask| {
                (0..7).filter(|bit| mask & (1 << bit) != 0).collect()
            }),
            active_start_time: self.active_start_time,
            active_end_time: self.active_end_time,
            is_active: self.is_active,
            created_by: self.created_by,
            created_at: self.created_at,
        }
    }
}

/// List products for a store (console read)
pub async fn list_products(
    pool: &PgPool,
    edge_server_id: i64,
) -> Result<Vec<CatalogProduct>, BoxError> {
    let rows: Vec<CatalogProductRow> = sqlx::query_as(
        r#"
        SELECT p.id, p.source_id, p.name, p.image, p.category_source_id,
               p.sort_order, p.tax_rate, p.receipt_name, p.kitchen_print_name,
               p.is_kitchen_print_enabled, p.is_label_print_enabled,
               p.is_active, p.external_id, p.updated_at,
               c.name AS category_name
        FROM catalog_products p
        LEFT JOIN catalog_categories c
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

    // Batch load specs
    let specs: Vec<CatalogProductSpecRow> = sqlx::query_as(
        r#"
        SELECT id, product_id, source_id, name, price, display_order,
               is_default, is_active, receipt_name, is_root
        FROM catalog_product_specs
        WHERE product_id = ANY($1)
        ORDER BY display_order
        "#,
    )
    .bind(&pg_ids)
    .fetch_all(pool)
    .await?;

    // Batch load product tags
    let tag_rows: Vec<(i64, i64)> = sqlx::query_as(
        "SELECT product_id, tag_source_id FROM catalog_product_tag WHERE product_id = ANY($1)",
    )
    .bind(&pg_ids)
    .fetch_all(pool)
    .await?;

    // Assemble
    let mut spec_map: std::collections::HashMap<i64, Vec<CatalogSpec>> =
        std::collections::HashMap::new();
    for s in specs {
        spec_map.entry(s.product_id).or_default().push(CatalogSpec {
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
        .map(|r| CatalogProduct {
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

/// List categories for a store (console read)
pub async fn list_categories(
    pool: &PgPool,
    edge_server_id: i64,
) -> Result<Vec<CatalogCategory>, BoxError> {
    #[derive(sqlx::FromRow)]
    struct Row {
        id: i64,
        source_id: i64,
        name: String,
        sort_order: i32,
        is_kitchen_print_enabled: bool,
        is_label_print_enabled: bool,
        is_active: bool,
        is_virtual: bool,
        match_mode: String,
        is_display: bool,
    }

    let rows: Vec<Row> = sqlx::query_as(
        r#"
        SELECT id, source_id, name, sort_order,
               is_kitchen_print_enabled, is_label_print_enabled,
               is_active, is_virtual, match_mode, is_display
        FROM catalog_categories
        WHERE edge_server_id = $1
        ORDER BY sort_order, source_id
        "#,
    )
    .bind(edge_server_id)
    .fetch_all(pool)
    .await?;

    let pg_ids: Vec<i64> = rows.iter().map(|r| r.id).collect();
    if pg_ids.is_empty() {
        return Ok(vec![]);
    }

    // Print destinations (with purpose)
    let dest_rows: Vec<(i64, i64, String)> = sqlx::query_as(
        "SELECT category_id, dest_source_id, purpose FROM catalog_category_print_dest WHERE category_id = ANY($1)",
    )
    .bind(&pg_ids)
    .fetch_all(pool)
    .await?;

    // Tag IDs
    let tag_rows: Vec<(i64, i64)> = sqlx::query_as(
        "SELECT category_id, tag_source_id FROM catalog_category_tag WHERE category_id = ANY($1)",
    )
    .bind(&pg_ids)
    .fetch_all(pool)
    .await?;

    let mut kitchen_dest_map: std::collections::HashMap<i64, Vec<i64>> =
        std::collections::HashMap::new();
    let mut label_dest_map: std::collections::HashMap<i64, Vec<i64>> =
        std::collections::HashMap::new();
    for (cat_id, dest_id, purpose) in dest_rows {
        match purpose.as_str() {
            "kitchen" => kitchen_dest_map.entry(cat_id).or_default().push(dest_id),
            "label" => label_dest_map.entry(cat_id).or_default().push(dest_id),
            _ => {} // ignore unknown purpose
        }
    }

    let mut tag_map: std::collections::HashMap<i64, Vec<i64>> = std::collections::HashMap::new();
    for (cat_id, tag_id) in tag_rows {
        tag_map.entry(cat_id).or_default().push(tag_id);
    }

    Ok(rows
        .into_iter()
        .map(|r| CatalogCategory {
            source_id: r.source_id,
            name: r.name,
            sort_order: r.sort_order,
            is_kitchen_print_enabled: r.is_kitchen_print_enabled,
            is_label_print_enabled: r.is_label_print_enabled,
            is_active: r.is_active,
            is_virtual: r.is_virtual,
            match_mode: r.match_mode,
            is_display: r.is_display,
            kitchen_print_destinations: kitchen_dest_map.remove(&r.id).unwrap_or_default(),
            label_print_destinations: label_dest_map.remove(&r.id).unwrap_or_default(),
            tag_ids: tag_map.remove(&r.id).unwrap_or_default(),
        })
        .collect())
}

/// List tags for a store (console read)
pub async fn list_tags(pool: &PgPool, edge_server_id: i64) -> Result<Vec<CatalogTag>, BoxError> {
    let rows: Vec<CatalogTag> = sqlx::query_as(
        r#"
        SELECT source_id, name, color, display_order, is_active, is_system
        FROM catalog_tags
        WHERE edge_server_id = $1
        ORDER BY display_order, source_id
        "#,
    )
    .bind(edge_server_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// List attributes for a store (console read)
pub async fn list_attributes(
    pool: &PgPool,
    edge_server_id: i64,
) -> Result<Vec<CatalogAttribute>, BoxError> {
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
        FROM catalog_attributes
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
        FROM catalog_attribute_options
        WHERE attribute_id = ANY($1)
        ORDER BY display_order
        "#,
    )
    .bind(&pg_ids)
    .fetch_all(pool)
    .await?;

    let mut opt_map: std::collections::HashMap<i64, Vec<CatalogAttributeOption>> =
        std::collections::HashMap::new();
    for o in opts {
        opt_map
            .entry(o.attribute_id)
            .or_default()
            .push(CatalogAttributeOption {
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
            CatalogAttribute {
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

// ════════════════════════════════════════════════════════════════
// Sync writes for tags, attributes, bindings
// ════════════════════════════════════════════════════════════════

/// Upsert tag from edge sync
pub async fn upsert_tag_from_sync(
    pool: &PgPool,
    edge_server_id: i64,
    source_id: i64,
    data: &serde_json::Value,
    now: i64,
) -> Result<(), BoxError> {
    let tag: Tag = serde_json::from_value(data.clone())?;
    sqlx::query(
        r#"
        INSERT INTO catalog_tags (edge_server_id, source_id, name, color, display_order, is_active, is_system, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        ON CONFLICT (edge_server_id, source_id)
        DO UPDATE SET name = EXCLUDED.name, color = EXCLUDED.color,
                      display_order = EXCLUDED.display_order, is_active = EXCLUDED.is_active,
                      is_system = EXCLUDED.is_system, updated_at = EXCLUDED.updated_at
        "#,
    )
    .bind(edge_server_id)
    .bind(source_id)
    .bind(&tag.name)
    .bind(&tag.color)
    .bind(tag.display_order)
    .bind(tag.is_active)
    .bind(tag.is_system)
    .bind(now)
    .execute(pool)
    .await?;
    Ok(())
}

/// Upsert attribute from edge sync (Attribute + options JSON)
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

    let row: (i64,) = sqlx::query_as(
        r#"
        INSERT INTO catalog_attributes (
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
    .fetch_one(&mut *tx)
    .await?;

    // Replace options
    sqlx::query("DELETE FROM catalog_attribute_options WHERE attribute_id = $1")
        .bind(row.0)
        .execute(&mut *tx)
        .await?;

    for opt in &attr.options {
        sqlx::query(
            r#"
            INSERT INTO catalog_attribute_options (
                attribute_id, source_id, name, price_modifier, display_order,
                is_active, receipt_name, kitchen_print_name, enable_quantity, max_quantity
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            "#,
        )
        .bind(row.0)
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

/// Upsert attribute binding from edge sync
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
        INSERT INTO catalog_attribute_bindings (
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

/// Upsert price rule from edge sync
pub async fn upsert_price_rule_from_sync(
    pool: &PgPool,
    edge_server_id: i64,
    source_id: i64,
    data: &serde_json::Value,
    now: i64,
) -> Result<(), BoxError> {
    let rule: shared::models::PriceRule = serde_json::from_value(data.clone())?;
    let active_days_mask: Option<i32> = rule
        .active_days
        .as_ref()
        .map(|days| days.iter().fold(0i32, |mask, &day| mask | (1 << day)));

    sqlx::query(
        r#"
        INSERT INTO catalog_price_rules (
            edge_server_id, source_id, name, display_name, receipt_name, description,
            rule_type, product_scope, target_id, zone_scope,
            adjustment_type, adjustment_value, is_stackable, is_exclusive,
            valid_from, valid_until, active_days, active_start_time, active_end_time,
            is_active, created_by, created_at, updated_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22, $23)
        ON CONFLICT (edge_server_id, source_id)
        DO UPDATE SET
            name = EXCLUDED.name, display_name = EXCLUDED.display_name,
            receipt_name = EXCLUDED.receipt_name, description = EXCLUDED.description,
            rule_type = EXCLUDED.rule_type, product_scope = EXCLUDED.product_scope,
            target_id = EXCLUDED.target_id, zone_scope = EXCLUDED.zone_scope,
            adjustment_type = EXCLUDED.adjustment_type, adjustment_value = EXCLUDED.adjustment_value,
            is_stackable = EXCLUDED.is_stackable, is_exclusive = EXCLUDED.is_exclusive,
            valid_from = EXCLUDED.valid_from, valid_until = EXCLUDED.valid_until,
            active_days = EXCLUDED.active_days, active_start_time = EXCLUDED.active_start_time,
            active_end_time = EXCLUDED.active_end_time, is_active = EXCLUDED.is_active,
            updated_at = EXCLUDED.updated_at
        "#,
    )
    .bind(edge_server_id)
    .bind(source_id)
    .bind(&rule.name)
    .bind(&rule.display_name)
    .bind(&rule.receipt_name)
    .bind(&rule.description)
    .bind(serde_json::to_value(&rule.rule_type).ok().and_then(|v| v.as_str().map(String::from)).unwrap_or_default())
    .bind(serde_json::to_value(&rule.product_scope).ok().and_then(|v| v.as_str().map(String::from)).unwrap_or_default())
    .bind(rule.target_id)
    .bind(&rule.zone_scope)
    .bind(serde_json::to_value(&rule.adjustment_type).ok().and_then(|v| v.as_str().map(String::from)).unwrap_or_default())
    .bind(rule.adjustment_value)
    .bind(rule.is_stackable)
    .bind(rule.is_exclusive)
    .bind(rule.valid_from)
    .bind(rule.valid_until)
    .bind(active_days_mask)
    .bind(&rule.active_start_time)
    .bind(&rule.active_end_time)
    .bind(rule.is_active)
    .bind(rule.created_by)
    .bind(rule.created_at)
    .bind(now)
    .execute(pool)
    .await?;
    Ok(())
}

// ════════════════════════════════════════════════════════════════
// JSONB mirror table reads (employees, zones, dining_tables)
// ════════════════════════════════════════════════════════════════

/// List employees for a store (from JSONB mirror table)
pub async fn list_employees(pool: &PgPool, edge_server_id: i64) -> Result<Vec<Employee>, BoxError> {
    let rows: Vec<(serde_json::Value,)> = sqlx::query_as(
        "SELECT data FROM cloud_employees WHERE edge_server_id = $1 ORDER BY synced_at DESC",
    )
    .bind(edge_server_id)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .filter_map(|(data,)| serde_json::from_value(data).ok())
        .collect())
}

/// List zones for a store (from JSONB mirror table)
pub async fn list_zones(pool: &PgPool, edge_server_id: i64) -> Result<Vec<Zone>, BoxError> {
    let rows: Vec<(serde_json::Value,)> = sqlx::query_as(
        "SELECT data FROM cloud_zones WHERE edge_server_id = $1 ORDER BY synced_at DESC",
    )
    .bind(edge_server_id)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .filter_map(|(data,)| serde_json::from_value(data).ok())
        .collect())
}

/// List dining tables for a store (from JSONB mirror table)
pub async fn list_tables(pool: &PgPool, edge_server_id: i64) -> Result<Vec<DiningTable>, BoxError> {
    let rows: Vec<(serde_json::Value,)> = sqlx::query_as(
        "SELECT data FROM cloud_dining_tables WHERE edge_server_id = $1 ORDER BY synced_at DESC",
    )
    .bind(edge_server_id)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .filter_map(|(data,)| serde_json::from_value(data).ok())
        .collect())
}

// ════════════════════════════════════════════════════════════════
// Console direct writes (PG authoritative CRUD)
// ════════════════════════════════════════════════════════════════

use shared::cloud::catalog::CatalogOpData;
use shared::models::{
    category::CategoryCreate,
    dining_table::DiningTableCreate,
    employee::EmployeeCreate,
    price_rule::PriceRuleCreate,
    product::{ProductCreate, ProductFull},
    tag::TagCreate,
    zone::ZoneCreate,
};

/// Increment catalog version for an edge server, returning the new version.
pub async fn increment_catalog_version(
    pool: &PgPool,
    edge_server_id: i64,
) -> Result<i64, BoxError> {
    let now = shared::util::now_millis();
    let row: (i64,) = sqlx::query_as(
        r#"
        INSERT INTO catalog_versions (edge_server_id, version, updated_at)
        VALUES ($1, 1, $2)
        ON CONFLICT (edge_server_id) DO UPDATE SET
            version = catalog_versions.version + 1,
            updated_at = EXCLUDED.updated_at
        RETURNING version
        "#,
    )
    .bind(edge_server_id)
    .bind(now)
    .fetch_one(pool)
    .await?;
    Ok(row.0)
}

// ── Product CRUD ──

/// Create product in PG, returns (source_id, CatalogOpData).
pub async fn create_product_direct(
    pool: &PgPool,
    edge_server_id: i64,
    data: &ProductCreate,
) -> Result<(i64, CatalogOpData), BoxError> {
    let now = shared::util::now_millis();
    let image = data.image.as_deref().unwrap_or("");
    let sort_order = data.sort_order.unwrap_or(0);
    let tax_rate = data.tax_rate.unwrap_or(0);
    let is_kitchen_print_enabled = data.is_kitchen_print_enabled.unwrap_or(-1);
    let is_label_print_enabled = data.is_label_print_enabled.unwrap_or(-1);

    let mut tx = pool.begin().await?;

    // INSERT with source_id = 0 first, then set source_id = id
    let (pg_id,): (i64,) = sqlx::query_as(
        r#"
        INSERT INTO catalog_products (
            edge_server_id, source_id, name, image, category_source_id,
            sort_order, tax_rate, receipt_name, kitchen_print_name,
            is_kitchen_print_enabled, is_label_print_enabled,
            is_active, external_id, updated_at
        )
        VALUES ($1, 0, $2, $3, $4, $5, $6, $7, $8, $9, $10, TRUE, $11, $12)
        RETURNING id
        "#,
    )
    .bind(edge_server_id)
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

    // source_id = id for cloud-created resources
    sqlx::query("UPDATE catalog_products SET source_id = id WHERE id = $1")
        .bind(pg_id)
        .execute(&mut *tx)
        .await?;

    // Insert specs
    for (i, spec) in data.specs.iter().enumerate() {
        sqlx::query(
            r#"
            INSERT INTO catalog_product_specs (
                product_id, source_id, name, price, display_order,
                is_default, is_active, receipt_name, is_root
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            "#,
        )
        .bind(pg_id)
        .bind(i as i64 + 1)
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

    // Insert tags
    if let Some(ref tags) = data.tags {
        for tag_id in tags {
            sqlx::query(
                "INSERT INTO catalog_product_tag (product_id, tag_source_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
            )
            .bind(pg_id)
            .bind(tag_id)
            .execute(&mut *tx)
            .await?;
        }
    }

    tx.commit().await?;

    let product_full = build_product_full_from_create(pg_id, data);
    Ok((pg_id, CatalogOpData::Product(product_full)))
}

fn build_product_full_from_create(source_id: i64, data: &ProductCreate) -> ProductFull {
    use shared::models::product::ProductSpec;

    let specs: Vec<ProductSpec> = data
        .specs
        .iter()
        .enumerate()
        .map(|(i, s)| ProductSpec {
            id: i as i64 + 1,
            product_id: source_id,
            name: s.name.clone(),
            price: s.price,
            display_order: s.display_order,
            is_default: s.is_default,
            is_active: s.is_active,
            receipt_name: s.receipt_name.clone(),
            is_root: s.is_root,
        })
        .collect();

    ProductFull {
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
    }
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
        "SELECT id FROM catalog_products WHERE edge_server_id = $1 AND source_id = $2",
    )
    .bind(edge_server_id)
    .bind(source_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or("Product not found")?;

    sqlx::query(
        r#"
        UPDATE catalog_products SET
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
        sqlx::query("DELETE FROM catalog_product_specs WHERE product_id = $1")
            .bind(pg_id)
            .execute(&mut *tx)
            .await?;
        for (i, spec) in specs.iter().enumerate() {
            sqlx::query(
                r#"
                INSERT INTO catalog_product_specs (
                    product_id, source_id, name, price, display_order,
                    is_default, is_active, receipt_name, is_root
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
                "#,
            )
            .bind(pg_id)
            .bind(i as i64 + 1)
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
    }

    if let Some(ref tags) = data.tags {
        sqlx::query("DELETE FROM catalog_product_tag WHERE product_id = $1")
            .bind(pg_id)
            .execute(&mut *tx)
            .await?;
        for tag_id in tags {
            sqlx::query(
                "INSERT INTO catalog_product_tag (product_id, tag_source_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
            )
            .bind(pg_id)
            .bind(tag_id)
            .execute(&mut *tx)
            .await?;
        }
    }

    tx.commit().await?;
    Ok(())
}

// ── Category CRUD ──

pub async fn create_category_direct(
    pool: &PgPool,
    edge_server_id: i64,
    data: &CategoryCreate,
) -> Result<(i64, CatalogOpData), BoxError> {
    let now = shared::util::now_millis();
    let sort_order = data.sort_order.unwrap_or(0);
    let is_kitchen_print_enabled = data.is_kitchen_print_enabled.unwrap_or(false);
    let is_label_print_enabled = data.is_label_print_enabled.unwrap_or(false);
    let is_virtual = data.is_virtual.unwrap_or(false);
    let match_mode = data.match_mode.as_deref().unwrap_or("any");
    let is_display = data.is_display.unwrap_or(true);

    let mut tx = pool.begin().await?;

    let (pg_id,): (i64,) = sqlx::query_as(
        r#"
        INSERT INTO catalog_categories (
            edge_server_id, source_id, name, sort_order,
            is_kitchen_print_enabled, is_label_print_enabled,
            is_active, is_virtual, match_mode, is_display, updated_at
        )
        VALUES ($1, 0, $2, $3, $4, $5, TRUE, $6, $7, $8, $9)
        RETURNING id
        "#,
    )
    .bind(edge_server_id)
    .bind(&data.name)
    .bind(sort_order)
    .bind(is_kitchen_print_enabled)
    .bind(is_label_print_enabled)
    .bind(is_virtual)
    .bind(match_mode)
    .bind(is_display)
    .bind(now)
    .fetch_one(&mut *tx)
    .await?;

    sqlx::query("UPDATE catalog_categories SET source_id = id WHERE id = $1")
        .bind(pg_id)
        .execute(&mut *tx)
        .await?;

    for dest_id in &data.kitchen_print_destinations {
        sqlx::query("INSERT INTO catalog_category_print_dest (category_id, dest_source_id, purpose) VALUES ($1, $2, 'kitchen')")
            .bind(pg_id)
            .bind(dest_id)
            .execute(&mut *tx)
            .await?;
    }
    for dest_id in &data.label_print_destinations {
        sqlx::query("INSERT INTO catalog_category_print_dest (category_id, dest_source_id, purpose) VALUES ($1, $2, 'label')")
            .bind(pg_id)
            .bind(dest_id)
            .execute(&mut *tx)
            .await?;
    }
    for tag_id in &data.tag_ids {
        sqlx::query(
            "INSERT INTO catalog_category_tag (category_id, tag_source_id) VALUES ($1, $2)",
        )
        .bind(pg_id)
        .bind(tag_id)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;

    let cat = Category {
        id: pg_id,
        name: data.name.clone(),
        sort_order,
        is_kitchen_print_enabled,
        is_label_print_enabled,
        is_active: true,
        is_virtual,
        match_mode: match_mode.to_string(),
        is_display,
        kitchen_print_destinations: data.kitchen_print_destinations.clone(),
        label_print_destinations: data.label_print_destinations.clone(),
        tag_ids: data.tag_ids.clone(),
    };
    Ok((pg_id, CatalogOpData::Category(cat)))
}

pub async fn update_category_direct(
    pool: &PgPool,
    edge_server_id: i64,
    source_id: i64,
    data: &shared::models::category::CategoryUpdate,
) -> Result<(), BoxError> {
    let now = shared::util::now_millis();
    let mut tx = pool.begin().await?;

    let pg_id: i64 = sqlx::query_scalar(
        "SELECT id FROM catalog_categories WHERE edge_server_id = $1 AND source_id = $2",
    )
    .bind(edge_server_id)
    .bind(source_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or("Category not found")?;

    sqlx::query(
        r#"
        UPDATE catalog_categories SET
            name = COALESCE($1, name),
            sort_order = COALESCE($2, sort_order),
            is_kitchen_print_enabled = COALESCE($3, is_kitchen_print_enabled),
            is_label_print_enabled = COALESCE($4, is_label_print_enabled),
            is_virtual = COALESCE($5, is_virtual),
            match_mode = COALESCE($6, match_mode),
            is_active = COALESCE($7, is_active),
            is_display = COALESCE($8, is_display),
            updated_at = $9
        WHERE id = $10
        "#,
    )
    .bind(&data.name)
    .bind(data.sort_order)
    .bind(data.is_kitchen_print_enabled)
    .bind(data.is_label_print_enabled)
    .bind(data.is_virtual)
    .bind(&data.match_mode)
    .bind(data.is_active)
    .bind(data.is_display)
    .bind(now)
    .bind(pg_id)
    .execute(&mut *tx)
    .await?;

    if data.kitchen_print_destinations.is_some() || data.label_print_destinations.is_some() {
        sqlx::query("DELETE FROM catalog_category_print_dest WHERE category_id = $1")
            .bind(pg_id)
            .execute(&mut *tx)
            .await?;
        if let Some(ref dests) = data.kitchen_print_destinations {
            for dest_id in dests {
                sqlx::query("INSERT INTO catalog_category_print_dest (category_id, dest_source_id, purpose) VALUES ($1, $2, 'kitchen')")
                    .bind(pg_id).bind(dest_id).execute(&mut *tx).await?;
            }
        }
        if let Some(ref dests) = data.label_print_destinations {
            for dest_id in dests {
                sqlx::query("INSERT INTO catalog_category_print_dest (category_id, dest_source_id, purpose) VALUES ($1, $2, 'label')")
                    .bind(pg_id).bind(dest_id).execute(&mut *tx).await?;
            }
        }
    }

    if let Some(ref tags) = data.tag_ids {
        sqlx::query("DELETE FROM catalog_category_tag WHERE category_id = $1")
            .bind(pg_id)
            .execute(&mut *tx)
            .await?;
        for tag_id in tags {
            sqlx::query(
                "INSERT INTO catalog_category_tag (category_id, tag_source_id) VALUES ($1, $2)",
            )
            .bind(pg_id)
            .bind(tag_id)
            .execute(&mut *tx)
            .await?;
        }
    }

    tx.commit().await?;
    Ok(())
}

// ── Tag CRUD ──

pub async fn create_tag_direct(
    pool: &PgPool,
    edge_server_id: i64,
    data: &TagCreate,
) -> Result<(i64, CatalogOpData), BoxError> {
    let now = shared::util::now_millis();
    let color = data.color.as_deref().unwrap_or("#3B82F6");
    let display_order = data.display_order.unwrap_or(0);

    let (pg_id,): (i64,) = sqlx::query_as(
        r#"
        INSERT INTO catalog_tags (edge_server_id, source_id, name, color, display_order, is_active, is_system, updated_at)
        VALUES ($1, 0, $2, $3, $4, TRUE, FALSE, $5)
        RETURNING id
        "#,
    )
    .bind(edge_server_id)
    .bind(&data.name)
    .bind(color)
    .bind(display_order)
    .bind(now)
    .fetch_one(pool)
    .await?;

    sqlx::query("UPDATE catalog_tags SET source_id = id WHERE id = $1")
        .bind(pg_id)
        .execute(pool)
        .await?;

    let tag = Tag {
        id: pg_id,
        name: data.name.clone(),
        color: color.to_string(),
        display_order,
        is_active: true,
        is_system: false,
    };
    Ok((pg_id, CatalogOpData::Tag(tag)))
}

pub async fn update_tag_direct(
    pool: &PgPool,
    edge_server_id: i64,
    source_id: i64,
    data: &shared::models::tag::TagUpdate,
) -> Result<(), BoxError> {
    let now = shared::util::now_millis();
    let rows = sqlx::query("UPDATE catalog_tags SET name = COALESCE($1, name), color = COALESCE($2, color), display_order = COALESCE($3, display_order), is_active = COALESCE($4, is_active), updated_at = $5 WHERE edge_server_id = $6 AND source_id = $7")
        .bind(&data.name).bind(&data.color).bind(data.display_order).bind(data.is_active).bind(now).bind(edge_server_id).bind(source_id)
        .execute(pool).await?;
    if rows.rows_affected() == 0 {
        return Err("Tag not found".into());
    }
    Ok(())
}

pub async fn delete_tag_direct(
    pool: &PgPool,
    edge_server_id: i64,
    source_id: i64,
) -> Result<(), BoxError> {
    sqlx::query("DELETE FROM catalog_tags WHERE edge_server_id = $1 AND source_id = $2")
        .bind(edge_server_id)
        .bind(source_id)
        .execute(pool)
        .await?;
    Ok(())
}

// ── Attribute CRUD ──

pub async fn create_attribute_direct(
    pool: &PgPool,
    edge_server_id: i64,
    data: &shared::models::attribute::AttributeCreate,
) -> Result<(i64,), BoxError> {
    let now = shared::util::now_millis();
    let default_ids_json = data
        .default_option_ids
        .as_ref()
        .map(serde_json::to_value)
        .transpose()?;
    let mut tx = pool.begin().await?;

    let (pg_id,): (i64,) = sqlx::query_as(
        r#"INSERT INTO catalog_attributes (edge_server_id, source_id, name, is_multi_select, max_selections, default_option_ids, display_order, is_active, show_on_receipt, receipt_name, show_on_kitchen_print, kitchen_print_name, updated_at) VALUES ($1, 0, $2, $3, $4, $5, $6, TRUE, $7, $8, $9, $10, $11) RETURNING id"#,
    )
    .bind(edge_server_id).bind(&data.name).bind(data.is_multi_select.unwrap_or(false)).bind(data.max_selections).bind(&default_ids_json).bind(data.display_order.unwrap_or(0)).bind(data.show_on_receipt.unwrap_or(false)).bind(&data.receipt_name).bind(data.show_on_kitchen_print.unwrap_or(false)).bind(&data.kitchen_print_name).bind(now)
    .fetch_one(&mut *tx).await?;

    sqlx::query("UPDATE catalog_attributes SET source_id = id WHERE id = $1")
        .bind(pg_id)
        .execute(&mut *tx)
        .await?;

    if let Some(ref options) = data.options {
        for opt in options {
            sqlx::query("INSERT INTO catalog_attribute_options (attribute_id, source_id, name, price_modifier, display_order, is_active, receipt_name, kitchen_print_name, enable_quantity, max_quantity) VALUES ($1, 0, $2, $3, $4, TRUE, $5, $6, $7, $8)")
                .bind(pg_id).bind(&opt.name).bind(opt.price_modifier).bind(opt.display_order).bind(&opt.receipt_name).bind(&opt.kitchen_print_name).bind(opt.enable_quantity).bind(opt.max_quantity)
                .execute(&mut *tx).await?;
        }
        sqlx::query("UPDATE catalog_attribute_options SET source_id = id WHERE attribute_id = $1")
            .bind(pg_id)
            .execute(&mut *tx)
            .await?;
    }

    tx.commit().await?;
    Ok((pg_id,))
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
        "SELECT id FROM catalog_attributes WHERE edge_server_id = $1 AND source_id = $2",
    )
    .bind(edge_server_id)
    .bind(source_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or("Attribute not found")?;

    sqlx::query("UPDATE catalog_attributes SET name = COALESCE($1, name), is_multi_select = COALESCE($2, is_multi_select), max_selections = COALESCE($3, max_selections), default_option_ids = COALESCE($4, default_option_ids), display_order = COALESCE($5, display_order), show_on_receipt = COALESCE($6, show_on_receipt), receipt_name = COALESCE($7, receipt_name), show_on_kitchen_print = COALESCE($8, show_on_kitchen_print), kitchen_print_name = COALESCE($9, kitchen_print_name), is_active = COALESCE($10, is_active), updated_at = $11 WHERE id = $12")
        .bind(&data.name).bind(data.is_multi_select).bind(data.max_selections).bind(&default_ids_json).bind(data.display_order).bind(data.show_on_receipt).bind(&data.receipt_name).bind(data.show_on_kitchen_print).bind(&data.kitchen_print_name).bind(data.is_active).bind(now).bind(pg_id)
        .execute(&mut *tx).await?;

    if let Some(ref options) = data.options {
        sqlx::query("DELETE FROM catalog_attribute_options WHERE attribute_id = $1")
            .bind(pg_id)
            .execute(&mut *tx)
            .await?;
        for opt in options {
            sqlx::query("INSERT INTO catalog_attribute_options (attribute_id, source_id, name, price_modifier, display_order, is_active, receipt_name, kitchen_print_name, enable_quantity, max_quantity) VALUES ($1, 0, $2, $3, $4, TRUE, $5, $6, $7, $8)")
                .bind(pg_id).bind(&opt.name).bind(opt.price_modifier).bind(opt.display_order).bind(&opt.receipt_name).bind(&opt.kitchen_print_name).bind(opt.enable_quantity).bind(opt.max_quantity)
                .execute(&mut *tx).await?;
        }
        sqlx::query("UPDATE catalog_attribute_options SET source_id = id WHERE attribute_id = $1")
            .bind(pg_id)
            .execute(&mut *tx)
            .await?;
    }

    tx.commit().await?;
    Ok(())
}

pub async fn delete_attribute_direct(
    pool: &PgPool,
    edge_server_id: i64,
    source_id: i64,
) -> Result<(), BoxError> {
    sqlx::query("DELETE FROM catalog_attributes WHERE edge_server_id = $1 AND source_id = $2")
        .bind(edge_server_id)
        .bind(source_id)
        .execute(pool)
        .await?;
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
    let (pg_id,): (i64,) = sqlx::query_as("INSERT INTO catalog_attribute_bindings (edge_server_id, source_id, owner_type, owner_source_id, attribute_source_id, is_required, display_order, default_option_ids) VALUES ($1, 0, $2, $3, $4, $5, $6, $7) RETURNING id")
        .bind(edge_server_id).bind(params.owner_type).bind(params.owner_id).bind(params.attribute_id).bind(params.is_required).bind(params.display_order).bind(&default_ids_json)
        .fetch_one(pool).await?;
    sqlx::query("UPDATE catalog_attribute_bindings SET source_id = id WHERE id = $1")
        .bind(pg_id)
        .execute(pool)
        .await?;
    Ok(pg_id)
}

pub async fn unbind_attribute_direct(
    pool: &PgPool,
    edge_server_id: i64,
    binding_id: i64,
) -> Result<(), BoxError> {
    sqlx::query(
        "DELETE FROM catalog_attribute_bindings WHERE edge_server_id = $1 AND source_id = $2",
    )
    .bind(edge_server_id)
    .bind(binding_id)
    .execute(pool)
    .await?;
    Ok(())
}

// ── Price Rule CRUD ──

pub async fn create_price_rule_direct(
    pool: &PgPool,
    edge_server_id: i64,
    data: &PriceRuleCreate,
) -> Result<(i64, CatalogOpData), BoxError> {
    let now = shared::util::now_millis();
    let zone_scope = data.zone_scope.as_deref().unwrap_or("all");
    let is_stackable = data.is_stackable.unwrap_or(true);
    let is_exclusive = data.is_exclusive.unwrap_or(false);
    let active_days_mask: Option<i32> = data
        .active_days
        .as_ref()
        .map(|days| days.iter().fold(0i32, |mask, &day| mask | (1 << day)));
    let rule_type_str = serde_json::to_value(&data.rule_type)
        .ok()
        .and_then(|v| v.as_str().map(String::from))
        .unwrap_or_default();
    let product_scope_str = serde_json::to_value(&data.product_scope)
        .ok()
        .and_then(|v| v.as_str().map(String::from))
        .unwrap_or_default();
    let adjustment_type_str = serde_json::to_value(&data.adjustment_type)
        .ok()
        .and_then(|v| v.as_str().map(String::from))
        .unwrap_or_default();

    let (pg_id,): (i64,) = sqlx::query_as(
        r#"INSERT INTO catalog_price_rules (edge_server_id, source_id, name, display_name, receipt_name, description, rule_type, product_scope, target_id, zone_scope, adjustment_type, adjustment_value, is_stackable, is_exclusive, valid_from, valid_until, active_days, active_start_time, active_end_time, is_active, created_by, created_at, updated_at) VALUES ($1, 0, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, TRUE, $19, $20, $20) RETURNING id"#,
    )
    .bind(edge_server_id).bind(&data.name).bind(&data.display_name).bind(&data.receipt_name).bind(&data.description).bind(&rule_type_str).bind(&product_scope_str).bind(data.target_id).bind(zone_scope).bind(&adjustment_type_str).bind(data.adjustment_value).bind(is_stackable).bind(is_exclusive).bind(data.valid_from).bind(data.valid_until).bind(active_days_mask).bind(&data.active_start_time).bind(&data.active_end_time).bind(data.created_by).bind(now)
    .fetch_one(pool).await?;

    sqlx::query("UPDATE catalog_price_rules SET source_id = id WHERE id = $1")
        .bind(pg_id)
        .execute(pool)
        .await?;

    let rule = shared::models::PriceRule {
        id: pg_id,
        name: data.name.clone(),
        display_name: data.display_name.clone(),
        receipt_name: data.receipt_name.clone(),
        description: data.description.clone(),
        rule_type: data.rule_type.clone(),
        product_scope: data.product_scope.clone(),
        target_id: data.target_id,
        zone_scope: zone_scope.to_string(),
        adjustment_type: data.adjustment_type.clone(),
        adjustment_value: data.adjustment_value,
        is_stackable,
        is_exclusive,
        valid_from: data.valid_from,
        valid_until: data.valid_until,
        active_days: data.active_days.clone(),
        active_start_time: data.active_start_time.clone(),
        active_end_time: data.active_end_time.clone(),
        is_active: true,
        created_by: data.created_by,
        created_at: now,
    };
    Ok((pg_id, CatalogOpData::PriceRule(rule)))
}

pub async fn update_price_rule_direct(
    pool: &PgPool,
    edge_server_id: i64,
    source_id: i64,
    data: &shared::models::price_rule::PriceRuleUpdate,
) -> Result<(), BoxError> {
    let now = shared::util::now_millis();
    let rule_type_str = data
        .rule_type
        .as_ref()
        .and_then(|v| serde_json::to_value(v).ok())
        .and_then(|v| v.as_str().map(String::from));
    let product_scope_str = data
        .product_scope
        .as_ref()
        .and_then(|v| serde_json::to_value(v).ok())
        .and_then(|v| v.as_str().map(String::from));
    let adjustment_type_str = data
        .adjustment_type
        .as_ref()
        .and_then(|v| serde_json::to_value(v).ok())
        .and_then(|v| v.as_str().map(String::from));
    let active_days_mask: Option<i32> = data
        .active_days
        .as_ref()
        .map(|days| days.iter().fold(0i32, |mask, &day| mask | (1 << day)));

    let rows = sqlx::query("UPDATE catalog_price_rules SET name = COALESCE($1, name), display_name = COALESCE($2, display_name), receipt_name = COALESCE($3, receipt_name), description = COALESCE($4, description), rule_type = COALESCE($5, rule_type), product_scope = COALESCE($6, product_scope), target_id = COALESCE($7, target_id), zone_scope = COALESCE($8, zone_scope), adjustment_type = COALESCE($9, adjustment_type), adjustment_value = COALESCE($10, adjustment_value), is_stackable = COALESCE($11, is_stackable), is_exclusive = COALESCE($12, is_exclusive), valid_from = COALESCE($13, valid_from), valid_until = COALESCE($14, valid_until), active_days = COALESCE($15, active_days), active_start_time = COALESCE($16, active_start_time), active_end_time = COALESCE($17, active_end_time), is_active = COALESCE($18, is_active), updated_at = $19 WHERE edge_server_id = $20 AND source_id = $21")
        .bind(&data.name).bind(&data.display_name).bind(&data.receipt_name).bind(&data.description).bind(&rule_type_str).bind(&product_scope_str).bind(data.target_id).bind(&data.zone_scope).bind(&adjustment_type_str).bind(data.adjustment_value).bind(data.is_stackable).bind(data.is_exclusive).bind(data.valid_from).bind(data.valid_until).bind(active_days_mask).bind(&data.active_start_time).bind(&data.active_end_time).bind(data.is_active).bind(now).bind(edge_server_id).bind(source_id)
        .execute(pool).await?;
    if rows.rows_affected() == 0 {
        return Err("Price rule not found".into());
    }
    Ok(())
}

pub async fn delete_price_rule_direct(
    pool: &PgPool,
    edge_server_id: i64,
    source_id: i64,
) -> Result<(), BoxError> {
    sqlx::query("DELETE FROM catalog_price_rules WHERE edge_server_id = $1 AND source_id = $2")
        .bind(edge_server_id)
        .bind(source_id)
        .execute(pool)
        .await?;
    Ok(())
}

// ── JSONB Resource CRUD (Employee, Zone, DiningTable) ──

pub async fn create_employee_direct(
    pool: &PgPool,
    edge_server_id: i64,
    tenant_id: &str,
    data: &EmployeeCreate,
) -> Result<(i64, CatalogOpData), BoxError> {
    let now = shared::util::now_millis();
    let display_name = data.display_name.as_deref().unwrap_or(&data.username);

    let hash_pass = {
        use argon2::{
            Argon2,
            password_hash::{PasswordHasher, SaltString, rand_core::OsRng},
        };
        let salt = SaltString::generate(&mut OsRng);
        Argon2::default()
            .hash_password(data.password.as_bytes(), &salt)
            .map_err(|e| format!("Failed to hash password: {e}"))?
            .to_string()
    };

    let (pg_id,): (i64,) = sqlx::query_as("INSERT INTO cloud_employees (edge_server_id, tenant_id, source_id, data, version, synced_at) VALUES ($1, $2, '0', '{}'::jsonb, 0, $3) RETURNING id")
        .bind(edge_server_id).bind(tenant_id).bind(now).fetch_one(pool).await?;

    let employee = Employee {
        id: pg_id,
        username: data.username.clone(),
        display_name: display_name.to_string(),
        role_id: data.role_id,
        is_system: false,
        is_active: true,
        created_at: now,
    };
    let mut stored_json = serde_json::to_value(&employee)?;
    if let Some(obj) = stored_json.as_object_mut() {
        obj.insert(
            "hash_pass".to_string(),
            serde_json::Value::String(hash_pass),
        );
    }

    sqlx::query("UPDATE cloud_employees SET source_id = $1, data = $2 WHERE id = $3")
        .bind(pg_id.to_string())
        .bind(&stored_json)
        .bind(pg_id)
        .execute(pool)
        .await?;

    Ok((pg_id, CatalogOpData::Employee(employee)))
}

pub async fn update_employee_direct(
    pool: &PgPool,
    edge_server_id: i64,
    source_id: i64,
    data: &shared::models::employee::EmployeeUpdate,
) -> Result<CatalogOpData, BoxError> {
    let now = shared::util::now_millis();
    let existing_json: serde_json::Value = sqlx::query_scalar(
        "SELECT data FROM cloud_employees WHERE edge_server_id = $1 AND source_id = $2::text",
    )
    .bind(edge_server_id)
    .bind(source_id.to_string())
    .fetch_optional(pool)
    .await?
    .ok_or("Employee not found")?;

    let mut obj = existing_json.as_object().cloned().unwrap_or_default();
    if let Some(ref v) = data.username {
        obj.insert("username".into(), serde_json::json!(v));
    }
    if let Some(ref v) = data.display_name {
        obj.insert("display_name".into(), serde_json::json!(v));
    }
    if let Some(v) = data.role_id {
        obj.insert("role_id".into(), serde_json::json!(v));
    }
    if let Some(v) = data.is_active {
        obj.insert("is_active".into(), serde_json::json!(v));
    }
    if let Some(ref password) = data.password {
        use argon2::{
            Argon2,
            password_hash::{PasswordHasher, SaltString, rand_core::OsRng},
        };
        let salt = SaltString::generate(&mut OsRng);
        let hash = Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| format!("Failed to hash password: {e}"))?
            .to_string();
        obj.insert("hash_pass".into(), serde_json::json!(hash));
    }

    let updated_json = serde_json::Value::Object(obj);
    sqlx::query("UPDATE cloud_employees SET data = $1, synced_at = $2 WHERE edge_server_id = $3 AND source_id = $4::text")
        .bind(&updated_json).bind(now).bind(edge_server_id).bind(source_id.to_string()).execute(pool).await?;

    let employee: Employee = serde_json::from_value(updated_json)?;
    Ok(CatalogOpData::Employee(employee))
}

pub async fn delete_employee_direct(
    pool: &PgPool,
    edge_server_id: i64,
    source_id: i64,
) -> Result<(), BoxError> {
    sqlx::query("DELETE FROM cloud_employees WHERE edge_server_id = $1 AND source_id = $2::text")
        .bind(edge_server_id)
        .bind(source_id.to_string())
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn create_zone_direct(
    pool: &PgPool,
    edge_server_id: i64,
    tenant_id: &str,
    data: &ZoneCreate,
) -> Result<(i64, CatalogOpData), BoxError> {
    let now = shared::util::now_millis();
    let (pg_id,): (i64,) = sqlx::query_as("INSERT INTO cloud_zones (edge_server_id, tenant_id, source_id, data, version, synced_at) VALUES ($1, $2, '0', '{}'::jsonb, 0, $3) RETURNING id")
        .bind(edge_server_id).bind(tenant_id).bind(now).fetch_one(pool).await?;

    let zone = Zone {
        id: pg_id,
        name: data.name.clone(),
        description: data.description.clone(),
        is_active: true,
    };
    let zone_json = serde_json::to_value(&zone)?;
    sqlx::query("UPDATE cloud_zones SET source_id = $1, data = $2 WHERE id = $3")
        .bind(pg_id.to_string())
        .bind(&zone_json)
        .bind(pg_id)
        .execute(pool)
        .await?;
    Ok((pg_id, CatalogOpData::Zone(zone)))
}

pub async fn update_zone_direct(
    pool: &PgPool,
    edge_server_id: i64,
    source_id: i64,
    data: &shared::models::zone::ZoneUpdate,
) -> Result<CatalogOpData, BoxError> {
    let now = shared::util::now_millis();
    let existing_json: serde_json::Value = sqlx::query_scalar(
        "SELECT data FROM cloud_zones WHERE edge_server_id = $1 AND source_id = $2::text",
    )
    .bind(edge_server_id)
    .bind(source_id.to_string())
    .fetch_optional(pool)
    .await?
    .ok_or("Zone not found")?;
    let mut zone: Zone = serde_json::from_value(existing_json)?;
    if let Some(ref v) = data.name {
        zone.name = v.clone();
    }
    if let Some(ref v) = data.description {
        zone.description = Some(v.clone());
    }
    if let Some(v) = data.is_active {
        zone.is_active = v;
    }
    let zone_json = serde_json::to_value(&zone)?;
    sqlx::query("UPDATE cloud_zones SET data = $1, synced_at = $2 WHERE edge_server_id = $3 AND source_id = $4::text").bind(&zone_json).bind(now).bind(edge_server_id).bind(source_id.to_string()).execute(pool).await?;
    Ok(CatalogOpData::Zone(zone))
}

pub async fn delete_zone_direct(
    pool: &PgPool,
    edge_server_id: i64,
    source_id: i64,
) -> Result<(), BoxError> {
    sqlx::query("DELETE FROM cloud_zones WHERE edge_server_id = $1 AND source_id = $2::text")
        .bind(edge_server_id)
        .bind(source_id.to_string())
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn create_table_direct(
    pool: &PgPool,
    edge_server_id: i64,
    tenant_id: &str,
    data: &DiningTableCreate,
) -> Result<(i64, CatalogOpData), BoxError> {
    let now = shared::util::now_millis();
    let (pg_id,): (i64,) = sqlx::query_as("INSERT INTO cloud_dining_tables (edge_server_id, tenant_id, source_id, data, version, synced_at) VALUES ($1, $2, '0', '{}'::jsonb, 0, $3) RETURNING id")
        .bind(edge_server_id).bind(tenant_id).bind(now).fetch_one(pool).await?;

    let table = DiningTable {
        id: pg_id,
        name: data.name.clone(),
        zone_id: data.zone_id,
        capacity: data.capacity.unwrap_or(4),
        is_active: true,
    };
    let table_json = serde_json::to_value(&table)?;
    sqlx::query("UPDATE cloud_dining_tables SET source_id = $1, data = $2 WHERE id = $3")
        .bind(pg_id.to_string())
        .bind(&table_json)
        .bind(pg_id)
        .execute(pool)
        .await?;
    Ok((pg_id, CatalogOpData::Table(table)))
}

pub async fn update_table_direct(
    pool: &PgPool,
    edge_server_id: i64,
    source_id: i64,
    data: &shared::models::dining_table::DiningTableUpdate,
) -> Result<CatalogOpData, BoxError> {
    let now = shared::util::now_millis();
    let existing_json: serde_json::Value = sqlx::query_scalar(
        "SELECT data FROM cloud_dining_tables WHERE edge_server_id = $1 AND source_id = $2::text",
    )
    .bind(edge_server_id)
    .bind(source_id.to_string())
    .fetch_optional(pool)
    .await?
    .ok_or("Dining table not found")?;
    let mut table: DiningTable = serde_json::from_value(existing_json)?;
    if let Some(ref v) = data.name {
        table.name = v.clone();
    }
    if let Some(v) = data.zone_id {
        table.zone_id = v;
    }
    if let Some(v) = data.capacity {
        table.capacity = v;
    }
    if let Some(v) = data.is_active {
        table.is_active = v;
    }
    let table_json = serde_json::to_value(&table)?;
    sqlx::query("UPDATE cloud_dining_tables SET data = $1, synced_at = $2 WHERE edge_server_id = $3 AND source_id = $4::text").bind(&table_json).bind(now).bind(edge_server_id).bind(source_id.to_string()).execute(pool).await?;
    Ok(CatalogOpData::Table(table))
}

pub async fn delete_table_direct(
    pool: &PgPool,
    edge_server_id: i64,
    source_id: i64,
) -> Result<(), BoxError> {
    sqlx::query(
        "DELETE FROM cloud_dining_tables WHERE edge_server_id = $1 AND source_id = $2::text",
    )
    .bind(edge_server_id)
    .bind(source_id.to_string())
    .execute(pool)
    .await?;
    Ok(())
}
