//! Data Transfer handlers — catalog export/import via ZIP
//!
//! Export: queries all catalog data from SQLite + images, returns ZIP bytes.
//! Import: accepts ZIP bytes, replaces catalog data + images, refreshes cache.

use std::io::{Cursor, Read, Write};

use axum::body::Bytes;
use axum::extract::State;
use axum::http::header;
use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};
use shared::cloud::SyncResource;
use shared::message::SyncChangeType;
use zip::write::FileOptions;
use zip::{ZipArchive, ZipWriter};

use crate::core::ServerState;
use crate::db::repository::{attribute, tag};
use crate::utils::AppError;
use shared::models::{Attribute, AttributeBinding, Category, ProductFull, Tag};

// =============================================================================
// Export types
// =============================================================================

#[derive(Serialize, Deserialize)]
struct CatalogExport {
    pub version: u32,
    pub exported_at: i64,
    pub tags: Vec<Tag>,
    pub categories: Vec<Category>,
    pub products: Vec<ProductFull>,
    pub attributes: Vec<Attribute>,
    pub attribute_bindings: Vec<AttributeBinding>,
}

// =============================================================================
// HTTP handlers (delegate to core functions)
// =============================================================================

/// GET /api/data-transfer/export
pub async fn export(State(state): State<ServerState>) -> Result<impl IntoResponse, AppError> {
    let zip_bytes = export_zip(&state).await?;

    Ok((
        [
            (header::CONTENT_TYPE, "application/zip"),
            (
                header::CONTENT_DISPOSITION,
                "attachment; filename=\"catalog_export.zip\"",
            ),
        ],
        zip_bytes,
    ))
}

/// POST /api/data-transfer/import
pub async fn import(
    State(state): State<ServerState>,
    body: Bytes,
) -> Result<impl IntoResponse, AppError> {
    import_zip(&state, body.as_ref()).await?;
    Ok(axum::Json(shared::error::ApiResponse::<()>::ok()))
}

// =============================================================================
// Core functions (usable from both HTTP and in-process)
// =============================================================================

/// Build catalog export ZIP bytes
pub(super) async fn export_zip(state: &ServerState) -> Result<Vec<u8>, AppError> {
    let categories = state.catalog_service.list_categories();
    let products = state.catalog_service.list_products();
    let mut tags = tag::find_all_with_inactive(&state.pool)
        .await
        .map_err(AppError::from)?;
    tags.sort_by_key(|t| t.display_order);
    let attributes = attribute::find_all(&state.pool)
        .await
        .map_err(AppError::from)?;
    let all_bindings = attribute::find_all_bindings(&state.pool)
        .await
        .map_err(AppError::from)?;

    // Filter bindings: only include those referencing exported entities + attributes
    let exported_category_ids: std::collections::HashSet<i64> =
        categories.iter().map(|c| c.id).collect();
    let exported_product_ids: std::collections::HashSet<i64> =
        products.iter().map(|p| p.id).collect();
    let exported_attribute_ids: std::collections::HashSet<i64> =
        attributes.iter().map(|a| a.id).collect();

    let bindings: Vec<_> = all_bindings
        .into_iter()
        .filter(|b| {
            let owner_valid = match b.owner_type.as_str() {
                "product" => exported_product_ids.contains(&b.owner_id),
                "category" => exported_category_ids.contains(&b.owner_id),
                _ => false,
            };
            owner_valid && exported_attribute_ids.contains(&b.attribute_id)
        })
        .collect();

    let catalog = CatalogExport {
        version: 1,
        exported_at: shared::util::now_millis(),
        tags,
        categories,
        products,
        attributes,
        attribute_bindings: bindings,
    };

    let catalog_json =
        serde_json::to_vec_pretty(&catalog).map_err(|e| AppError::internal(e.to_string()))?;

    // Build ZIP in memory
    let mut buf = Cursor::new(Vec::new());
    {
        let mut zip = ZipWriter::new(&mut buf);
        let options: FileOptions<()> =
            FileOptions::default().compression_method(zip::CompressionMethod::Deflated);

        // Write catalog.json
        zip.start_file("catalog.json", options)
            .map_err(|e| AppError::internal(e.to_string()))?;
        zip.write_all(&catalog_json)
            .map_err(|e| AppError::internal(e.to_string()))?;

        // Write images
        let images_dir = state.work_dir().join("images");
        if images_dir.exists()
            && let Ok(entries) = std::fs::read_dir(&images_dir)
        {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file()
                    && let Some(name) = path.file_name().and_then(|n| n.to_str())
                {
                    let zip_path = format!("images/{name}");
                    zip.start_file(&zip_path, options)
                        .map_err(|e| AppError::internal(e.to_string()))?;
                    let data =
                        std::fs::read(&path).map_err(|e| AppError::internal(e.to_string()))?;
                    zip.write_all(&data)
                        .map_err(|e| AppError::internal(e.to_string()))?;
                }
            }
        }

        zip.finish()
            .map_err(|e| AppError::internal(e.to_string()))?;
    }

    Ok(buf.into_inner())
}

/// Parse ZIP and import catalog data + images
pub(super) async fn import_zip(state: &ServerState, zip_bytes: &[u8]) -> Result<(), AppError> {
    let cursor = Cursor::new(zip_bytes);
    let mut archive =
        ZipArchive::new(cursor).map_err(|e| AppError::validation(format!("Invalid ZIP: {e}")))?;

    // Read catalog.json
    let catalog: CatalogExport = {
        let mut file = archive
            .by_name("catalog.json")
            .map_err(|_| AppError::validation("ZIP missing catalog.json"))?;
        let mut json_bytes = Vec::new();
        file.read_to_end(&mut json_bytes)
            .map_err(|e| AppError::internal(e.to_string()))?;
        serde_json::from_slice(&json_bytes)
            .map_err(|e| AppError::validation(format!("Invalid catalog.json: {e}")))?
    };

    // Extract images
    let images_dir = state.work_dir().join("images");
    std::fs::create_dir_all(&images_dir).map_err(|e| AppError::internal(e.to_string()))?;

    for i in 0..archive.len() {
        let mut file = archive
            .by_index(i)
            .map_err(|e| AppError::internal(e.to_string()))?;
        let name = file.name().to_string();
        if let Some(image_name) = name.strip_prefix("images/")
            && !image_name.is_empty()
            && !image_name.contains("..")
        {
            let dest = images_dir.join(image_name);
            let mut data = Vec::new();
            file.read_to_end(&mut data)
                .map_err(|e| AppError::internal(e.to_string()))?;
            std::fs::write(&dest, &data).map_err(|e| AppError::internal(e.to_string()))?;
        }
    }

    // Import data within a transaction
    import_catalog_data(state, &catalog).await?;

    // Refresh catalog cache
    state
        .catalog_service
        .warmup()
        .await
        .map_err(|e| AppError::internal(e.to_string()))?;

    // Broadcast sync events from actual DB data (IDs are remapped during import)
    // CloudSyncWorker debounces (500ms) and batches these for cloud push
    broadcast_catalog_sync(state).await;

    Ok(())
}

// =============================================================================
// SQL import logic
// =============================================================================

use std::collections::HashMap;

/// ID 重映射表: old_id → new_id (snowflake)
struct IdRemap {
    tags: HashMap<i64, i64>,
    categories: HashMap<i64, i64>,
    products: HashMap<i64, i64>,
    specs: HashMap<i64, i64>,
    attributes: HashMap<i64, i64>,
    options: HashMap<i64, i64>,
}

impl IdRemap {
    fn new() -> Self {
        Self {
            tags: HashMap::new(),
            categories: HashMap::new(),
            products: HashMap::new(),
            specs: HashMap::new(),
            attributes: HashMap::new(),
            options: HashMap::new(),
        }
    }

    /// 按 name 查找已存在记录的 ID，否则生成新 snowflake ID
    fn get_or_create(map: &mut HashMap<i64, i64>, old_id: i64, existing_id: Option<i64>) -> i64 {
        if let Some(&mapped) = map.get(&old_id) {
            return mapped;
        }
        let new_id = existing_id.unwrap_or_else(shared::util::snowflake_id);
        map.insert(old_id, new_id);
        new_id
    }
}

/// Import catalog data: incremental merge with snowflake ID remapping.
///
/// - 按 name 匹配已有记录 → 复用其 ID (upsert)
/// - 新记录 → 生成 snowflake ID
/// - 所有 FK 引用通过 IdRemap 转换
async fn import_catalog_data(state: &ServerState, catalog: &CatalogExport) -> Result<(), AppError> {
    let mut tx = state
        .pool
        .begin()
        .await
        .map_err(|e| AppError::database(e.to_string()))?;

    let mut remap = IdRemap::new();

    // === Tags: match by name ===
    for tag in &catalog.tags {
        let existing: Option<(i64,)> = sqlx::query_as("SELECT id FROM tag WHERE name = ?")
            .bind(&tag.name)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|e| AppError::database(e.to_string()))?;

        let new_id = IdRemap::get_or_create(&mut remap.tags, tag.id, existing.map(|r| r.0));

        sqlx::query(
            "INSERT INTO tag (id, name, color, display_order, is_active, is_system)
             VALUES (?, ?, ?, ?, ?, ?)
             ON CONFLICT(id) DO UPDATE SET name=excluded.name, color=excluded.color,
                display_order=excluded.display_order, is_active=excluded.is_active",
        )
        .bind(new_id)
        .bind(&tag.name)
        .bind(&tag.color)
        .bind(tag.display_order)
        .bind(tag.is_active)
        .bind(tag.is_system)
        .execute(&mut *tx)
        .await
        .map_err(|e| AppError::database(e.to_string()))?;
    }

    // === Categories: match by name ===
    for cat in &catalog.categories {
        let existing: Option<(i64,)> = sqlx::query_as("SELECT id FROM category WHERE name = ?")
            .bind(&cat.name)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|e| AppError::database(e.to_string()))?;

        let new_id = IdRemap::get_or_create(&mut remap.categories, cat.id, existing.map(|r| r.0));

        sqlx::query(
            "INSERT INTO category (id, name, sort_order, is_kitchen_print_enabled, is_label_print_enabled, is_active, is_virtual, match_mode, is_display)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(id) DO UPDATE SET name=excluded.name, sort_order=excluded.sort_order,
                is_kitchen_print_enabled=excluded.is_kitchen_print_enabled,
                is_label_print_enabled=excluded.is_label_print_enabled,
                is_active=excluded.is_active, is_virtual=excluded.is_virtual,
                match_mode=excluded.match_mode, is_display=excluded.is_display",
        )
        .bind(new_id)
        .bind(&cat.name)
        .bind(cat.sort_order)
        .bind(cat.is_kitchen_print_enabled)
        .bind(cat.is_label_print_enabled)
        .bind(cat.is_active)
        .bind(cat.is_virtual)
        .bind(&cat.match_mode)
        .bind(cat.is_display)
        .execute(&mut *tx)
        .await
        .map_err(|e| AppError::database(e.to_string()))?;

        // Category print destinations
        let mut seen = std::collections::HashSet::new();
        for dest_id in cat
            .kitchen_print_destinations
            .iter()
            .chain(cat.label_print_destinations.iter())
        {
            if seen.insert(dest_id) {
                sqlx::query(
                    "INSERT OR IGNORE INTO category_print_dest (category_id, print_destination_id) VALUES (?, ?)",
                )
                .bind(new_id)
                .bind(dest_id)
                .execute(&mut *tx)
                .await
                .map_err(|e| AppError::database(e.to_string()))?;
            }
        }

        // Category tags (remap tag IDs)
        for old_tag_id in &cat.tag_ids {
            if let Some(&new_tag_id) = remap.tags.get(old_tag_id) {
                sqlx::query(
                    "INSERT OR IGNORE INTO category_tag (category_id, tag_id) VALUES (?, ?)",
                )
                .bind(new_id)
                .bind(new_tag_id)
                .execute(&mut *tx)
                .await
                .map_err(|e| AppError::database(e.to_string()))?;
            }
        }
    }

    // === Products: match by name + category ===
    for product in &catalog.products {
        let new_cat_id = remap
            .categories
            .get(&product.category_id)
            .copied()
            .unwrap_or(product.category_id);

        let existing: Option<(i64,)> =
            sqlx::query_as("SELECT id FROM product WHERE name = ? AND category_id = ?")
                .bind(&product.name)
                .bind(new_cat_id)
                .fetch_optional(&mut *tx)
                .await
                .map_err(|e| AppError::database(e.to_string()))?;

        let new_id = IdRemap::get_or_create(&mut remap.products, product.id, existing.map(|r| r.0));

        sqlx::query(
            "INSERT INTO product (id, name, image, category_id, sort_order, tax_rate, receipt_name, kitchen_print_name, is_kitchen_print_enabled, is_label_print_enabled, is_active, external_id)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(id) DO UPDATE SET name=excluded.name, image=excluded.image,
                category_id=excluded.category_id, sort_order=excluded.sort_order,
                tax_rate=excluded.tax_rate, receipt_name=excluded.receipt_name,
                kitchen_print_name=excluded.kitchen_print_name,
                is_kitchen_print_enabled=excluded.is_kitchen_print_enabled,
                is_label_print_enabled=excluded.is_label_print_enabled,
                is_active=excluded.is_active, external_id=excluded.external_id",
        )
        .bind(new_id)
        .bind(&product.name)
        .bind(&product.image)
        .bind(new_cat_id)
        .bind(product.sort_order)
        .bind(product.tax_rate)
        .bind(&product.receipt_name)
        .bind(&product.kitchen_print_name)
        .bind(product.is_kitchen_print_enabled)
        .bind(product.is_label_print_enabled)
        .bind(product.is_active)
        .bind(product.external_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| AppError::database(e.to_string()))?;

        // Product specs: match by name + product
        for spec in &product.specs {
            let existing_spec: Option<(i64,)> =
                sqlx::query_as("SELECT id FROM product_spec WHERE product_id = ? AND name = ?")
                    .bind(new_id)
                    .bind(&spec.name)
                    .fetch_optional(&mut *tx)
                    .await
                    .map_err(|e| AppError::database(e.to_string()))?;

            let new_spec_id =
                IdRemap::get_or_create(&mut remap.specs, spec.id, existing_spec.map(|r| r.0));

            sqlx::query(
                "INSERT INTO product_spec (id, product_id, name, price, display_order, is_default, is_active, receipt_name, is_root)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
                 ON CONFLICT(id) DO UPDATE SET name=excluded.name, price=excluded.price,
                    display_order=excluded.display_order, is_default=excluded.is_default,
                    is_active=excluded.is_active, receipt_name=excluded.receipt_name,
                    is_root=excluded.is_root",
            )
            .bind(new_spec_id)
            .bind(new_id)
            .bind(&spec.name)
            .bind(spec.price)
            .bind(spec.display_order)
            .bind(spec.is_default)
            .bind(spec.is_active)
            .bind(&spec.receipt_name)
            .bind(spec.is_root)
            .execute(&mut *tx)
            .await
            .map_err(|e| AppError::database(e.to_string()))?;
        }

        // Product tags (remap)
        for tag in &product.tags {
            if let Some(&new_tag_id) = remap.tags.get(&tag.id) {
                sqlx::query("INSERT OR IGNORE INTO product_tag (product_id, tag_id) VALUES (?, ?)")
                    .bind(new_id)
                    .bind(new_tag_id)
                    .execute(&mut *tx)
                    .await
                    .map_err(|e| AppError::database(e.to_string()))?;
            }
        }
    }

    // === Attributes: match by name ===
    // Phase 1: Insert attributes + options (populate remap.options)
    for attr in &catalog.attributes {
        let existing: Option<(i64,)> = sqlx::query_as("SELECT id FROM attribute WHERE name = ?")
            .bind(&attr.name)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|e| AppError::database(e.to_string()))?;

        let new_id = IdRemap::get_or_create(&mut remap.attributes, attr.id, existing.map(|r| r.0));

        // Insert attribute without default_option_ids first (options not yet remapped)
        sqlx::query(
            "INSERT INTO attribute (id, name, is_multi_select, max_selections, default_option_ids, display_order, is_active, show_on_receipt, receipt_name, show_on_kitchen_print, kitchen_print_name)
             VALUES (?, ?, ?, ?, NULL, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(id) DO UPDATE SET name=excluded.name, is_multi_select=excluded.is_multi_select,
                max_selections=excluded.max_selections,
                display_order=excluded.display_order, is_active=excluded.is_active,
                show_on_receipt=excluded.show_on_receipt, receipt_name=excluded.receipt_name,
                show_on_kitchen_print=excluded.show_on_kitchen_print,
                kitchen_print_name=excluded.kitchen_print_name",
        )
        .bind(new_id)
        .bind(&attr.name)
        .bind(attr.is_multi_select)
        .bind(attr.max_selections)
        .bind(attr.display_order)
        .bind(attr.is_active)
        .bind(attr.show_on_receipt)
        .bind(&attr.receipt_name)
        .bind(attr.show_on_kitchen_print)
        .bind(&attr.kitchen_print_name)
        .execute(&mut *tx)
        .await
        .map_err(|e| AppError::database(e.to_string()))?;

        // Attribute options: match by name + attribute
        for opt in &attr.options {
            let existing_opt: Option<(i64,)> = sqlx::query_as(
                "SELECT id FROM attribute_option WHERE attribute_id = ? AND name = ?",
            )
            .bind(new_id)
            .bind(&opt.name)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|e| AppError::database(e.to_string()))?;

            let new_opt_id =
                IdRemap::get_or_create(&mut remap.options, opt.id, existing_opt.map(|r| r.0));

            sqlx::query(
                "INSERT INTO attribute_option (id, attribute_id, name, price_modifier, display_order, is_active, receipt_name, kitchen_print_name, enable_quantity, max_quantity)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                 ON CONFLICT(id) DO UPDATE SET name=excluded.name, price_modifier=excluded.price_modifier,
                    display_order=excluded.display_order, is_active=excluded.is_active,
                    receipt_name=excluded.receipt_name, kitchen_print_name=excluded.kitchen_print_name,
                    enable_quantity=excluded.enable_quantity, max_quantity=excluded.max_quantity",
            )
            .bind(new_opt_id)
            .bind(new_id)
            .bind(&opt.name)
            .bind(opt.price_modifier)
            .bind(opt.display_order)
            .bind(opt.is_active)
            .bind(&opt.receipt_name)
            .bind(&opt.kitchen_print_name)
            .bind(opt.enable_quantity)
            .bind(opt.max_quantity)
            .execute(&mut *tx)
            .await
            .map_err(|e| AppError::database(e.to_string()))?;
        }
    }

    // Phase 2: Back-fill attribute default_option_ids with remapped option IDs
    for attr in &catalog.attributes {
        if let Some(ref old_ids) = attr.default_option_ids {
            let new_attr_id = remap.attributes[&attr.id];
            let remapped: Vec<i64> = old_ids
                .iter()
                .filter_map(|old| remap.options.get(old).copied())
                .collect();
            let json = serde_json::to_string(&remapped).unwrap_or_else(|_| "null".to_string());
            sqlx::query("UPDATE attribute SET default_option_ids = ? WHERE id = ?")
                .bind(&json)
                .bind(new_attr_id)
                .execute(&mut *tx)
                .await
                .map_err(|e| AppError::database(e.to_string()))?;
        }
    }

    // === Attribute bindings: remap all FK references ===
    for binding in &catalog.attribute_bindings {
        // Remap owner_id based on owner_type
        let new_owner_id = match binding.owner_type.as_str() {
            "product" => remap.products.get(&binding.owner_id).copied(),
            "category" => remap.categories.get(&binding.owner_id).copied(),
            _ => None,
        };
        let new_attr_id = remap.attributes.get(&binding.attribute_id).copied();

        // Skip bindings with unmapped references
        let (Some(owner_id), Some(attr_id)) = (new_owner_id, new_attr_id) else {
            tracing::warn!(
                owner_type = %binding.owner_type,
                owner_id = binding.owner_id,
                attribute_id = binding.attribute_id,
                owner_mapped = new_owner_id.is_some(),
                attr_mapped = new_attr_id.is_some(),
                "Skipping attribute binding: unmapped reference"
            );
            continue;
        };

        let new_id = shared::util::snowflake_id();

        // Remap default_option_ids to new option IDs
        let remapped_default_opts: Option<Vec<i64>> =
            binding.default_option_ids.as_ref().map(|ids| {
                ids.iter()
                    .filter_map(|old| remap.options.get(old).copied())
                    .collect()
            });
        let default_option_ids_json = remapped_default_opts
            .as_ref()
            .map(|v| serde_json::to_string(v).unwrap_or_else(|_| "null".to_string()));

        // Skip if binding already exists
        let existing: Option<(i64,)> = sqlx::query_as(
            "SELECT id FROM attribute_binding WHERE owner_type = ? AND owner_id = ? AND attribute_id = ?",
        )
        .bind(&binding.owner_type)
        .bind(owner_id)
        .bind(attr_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| AppError::database(e.to_string()))?;

        let binding_id = existing.map(|r| r.0).unwrap_or(new_id);

        sqlx::query(
            "INSERT INTO attribute_binding (id, owner_type, owner_id, attribute_id, is_required, display_order, default_option_ids)
             VALUES (?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(id) DO UPDATE SET is_required=excluded.is_required,
                display_order=excluded.display_order, default_option_ids=excluded.default_option_ids",
        )
        .bind(binding_id)
        .bind(&binding.owner_type)
        .bind(owner_id)
        .bind(attr_id)
        .bind(binding.is_required)
        .bind(binding.display_order)
        .bind(&default_option_ids_json)
        .execute(&mut *tx)
        .await
        .map_err(|e| AppError::database(e.to_string()))?;
    }

    tx.commit()
        .await
        .map_err(|e| AppError::database(e.to_string()))?;

    Ok(())
}

/// Read actual DB state and broadcast individual sync events for each catalog resource.
/// CloudSyncWorker picks these up and pushes to cloud.
async fn broadcast_catalog_sync(state: &ServerState) {
    // Tags
    if let Ok(tags) = tag::find_all(&state.pool).await {
        for t in &tags {
            state
                .broadcast_sync(
                    SyncResource::Tag,
                    SyncChangeType::Updated,
                    &t.id.to_string(),
                    Some(t),
                    false,
                )
                .await;
        }
    }

    // Categories (from cache)
    let categories = state.catalog_service.list_categories();
    for c in &categories {
        state
            .broadcast_sync(
                SyncResource::Category,
                SyncChangeType::Updated,
                &c.id.to_string(),
                Some(c),
                false,
            )
            .await;
    }

    // Products (from cache)
    let products = state.catalog_service.list_products();
    for p in &products {
        state
            .broadcast_sync(
                SyncResource::Product,
                SyncChangeType::Updated,
                &p.id.to_string(),
                Some(p),
                false,
            )
            .await;
    }

    // Attributes
    if let Ok(attrs) = attribute::find_all(&state.pool).await {
        for a in &attrs {
            state
                .broadcast_sync(
                    SyncResource::Attribute,
                    SyncChangeType::Updated,
                    &a.id.to_string(),
                    Some(a),
                    false,
                )
                .await;
        }
    }

    // Attribute bindings
    if let Ok(bindings) = sqlx::query_as::<_, AttributeBinding>(
        "SELECT id, owner_type, owner_id, attribute_id, is_required, display_order, \
         COALESCE(default_option_ids, 'null') as default_option_ids \
         FROM attribute_binding ORDER BY display_order",
    )
    .fetch_all(&state.pool)
    .await
    {
        for b in &bindings {
            state
                .broadcast_sync(
                    SyncResource::AttributeBinding,
                    SyncChangeType::Updated,
                    &b.id.to_string(),
                    Some(b),
                    false,
                )
                .await;
        }
    }
}
