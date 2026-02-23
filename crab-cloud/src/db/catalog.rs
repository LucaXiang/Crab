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
