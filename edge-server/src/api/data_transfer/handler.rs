//! Data Transfer handlers — catalog export/import via ZIP
//!
//! Export: queries all catalog data from SQLite + images, returns ZIP bytes.
//! Import: accepts ZIP bytes, replaces catalog data + images, refreshes cache.

use std::io::{Cursor, Read, Write};

use axum::body::Bytes;
use axum::extract::State;
use axum::http::header;
use axum::response::IntoResponse;
use shared::cloud::SyncResource;
use shared::message::SyncChangeType;
use sqlx;
use zip::write::FileOptions;
use zip::{ZipArchive, ZipWriter};

use crate::core::ServerState;
use crate::db::repository::{attribute, dining_table, price_rule, tag, zone};
use crate::utils::AppError;
use shared::models::{AttributeBinding, CatalogExport, validate_catalog};

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
    let price_rules = price_rule::find_all_with_inactive(&state.pool)
        .await
        .map_err(AppError::from)?;
    let zones = zone::find_all_with_inactive(&state.pool)
        .await
        .map_err(AppError::from)?;
    let dining_tables = dining_table::find_all_with_inactive(&state.pool)
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
        price_rules,
        zones,
        dining_tables,
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

    // Validate referential integrity before touching the database
    validate_catalog(&catalog)
        .map_err(|e| AppError::validation(format!("Catalog validation failed: {e}")))?;

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
// SQL import logic — full replacement (DELETE ALL + INSERT ALL)
// =============================================================================

/// Import catalog data: full replacement within a single transaction.
///
/// 1. DELETE all catalog data (reverse FK order, CASCADE handles children)
/// 2. INSERT all exported data with original IDs
/// 3. Attribute default_option_ids back-filled after options exist
async fn import_catalog_data(state: &ServerState, catalog: &CatalogExport) -> Result<(), AppError> {
    let mut tx = state
        .pool
        .begin()
        .await
        .map_err(|e| AppError::database(e.to_string()))?;

    // ── DELETE (reverse FK order; CASCADE handles junction/child tables) ──
    sqlx::query("DELETE FROM price_rule")
        .execute(&mut *tx)
        .await
        .map_err(|e| AppError::database(e.to_string()))?;
    sqlx::query("DELETE FROM dining_table")
        .execute(&mut *tx)
        .await
        .map_err(|e| AppError::database(e.to_string()))?;
    sqlx::query("DELETE FROM zone")
        .execute(&mut *tx)
        .await
        .map_err(|e| AppError::database(e.to_string()))?;
    sqlx::query("DELETE FROM attribute_binding")
        .execute(&mut *tx)
        .await
        .map_err(|e| AppError::database(e.to_string()))?;
    sqlx::query("DELETE FROM attribute")
        .execute(&mut *tx)
        .await
        .map_err(|e| AppError::database(e.to_string()))?;
    sqlx::query("DELETE FROM product")
        .execute(&mut *tx)
        .await
        .map_err(|e| AppError::database(e.to_string()))?;
    sqlx::query("DELETE FROM category")
        .execute(&mut *tx)
        .await
        .map_err(|e| AppError::database(e.to_string()))?;
    sqlx::query("DELETE FROM tag")
        .execute(&mut *tx)
        .await
        .map_err(|e| AppError::database(e.to_string()))?;

    // ── INSERT tags ──
    for tag in &catalog.tags {
        sqlx::query(
            "INSERT INTO tag (id, name, color, display_order, is_active, is_system) VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(tag.id)
        .bind(&tag.name)
        .bind(&tag.color)
        .bind(tag.display_order)
        .bind(tag.is_active)
        .bind(tag.is_system)
        .execute(&mut *tx)
        .await
        .map_err(|e| AppError::database(e.to_string()))?;
    }

    // ── INSERT categories ──
    for cat in &catalog.categories {
        sqlx::query(
            "INSERT INTO category (id, name, sort_order, is_kitchen_print_enabled, is_label_print_enabled, is_active, is_virtual, match_mode, is_display) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(cat.id)
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

        // Category → print_destination (kitchen + label share the same junction table)
        for dest_id in &cat.kitchen_print_destinations {
            sqlx::query("INSERT OR IGNORE INTO category_print_dest (category_id, print_destination_id) VALUES (?, ?)")
                .bind(cat.id)
                .bind(dest_id)
                .execute(&mut *tx)
                .await
                .map_err(|e| AppError::database(e.to_string()))?;
        }
        for dest_id in &cat.label_print_destinations {
            sqlx::query("INSERT OR IGNORE INTO category_print_dest (category_id, print_destination_id) VALUES (?, ?)")
                .bind(cat.id)
                .bind(dest_id)
                .execute(&mut *tx)
                .await
                .map_err(|e| AppError::database(e.to_string()))?;
        }

        // Category → tag
        for tag_id in &cat.tag_ids {
            sqlx::query("INSERT INTO category_tag (category_id, tag_id) VALUES (?, ?)")
                .bind(cat.id)
                .bind(tag_id)
                .execute(&mut *tx)
                .await
                .map_err(|e| AppError::database(e.to_string()))?;
        }
    }

    // ── INSERT products ──
    for product in &catalog.products {
        sqlx::query(
            "INSERT INTO product (id, name, image, category_id, sort_order, tax_rate, receipt_name, kitchen_print_name, is_kitchen_print_enabled, is_label_print_enabled, is_active, external_id) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
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
        .execute(&mut *tx)
        .await
        .map_err(|e| AppError::database(e.to_string()))?;

        // Product specs
        for spec in &product.specs {
            sqlx::query(
                "INSERT INTO product_spec (id, product_id, name, price, display_order, is_default, is_active, receipt_name, is_root) \
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(spec.id)
            .bind(product.id)
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

        // Product → tag
        for tag in &product.tags {
            sqlx::query("INSERT INTO product_tag (product_id, tag_id) VALUES (?, ?)")
                .bind(product.id)
                .bind(tag.id)
                .execute(&mut *tx)
                .await
                .map_err(|e| AppError::database(e.to_string()))?;
        }
    }

    // ── INSERT attributes (without default_option_ids first) ──
    for attr in &catalog.attributes {
        sqlx::query(
            "INSERT INTO attribute (id, name, is_multi_select, max_selections, default_option_ids, display_order, is_active, show_on_receipt, receipt_name, show_on_kitchen_print, kitchen_print_name) \
             VALUES (?, ?, ?, ?, NULL, ?, ?, ?, ?, ?, ?)",
        )
        .bind(attr.id)
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

        // Attribute options
        for opt in &attr.options {
            sqlx::query(
                "INSERT INTO attribute_option (id, attribute_id, name, price_modifier, display_order, is_active, receipt_name, kitchen_print_name, enable_quantity, max_quantity) \
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(opt.id)
            .bind(attr.id)
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

    // Back-fill attribute default_option_ids (options now exist)
    for attr in &catalog.attributes {
        if let Some(ref ids) = attr.default_option_ids {
            let json = serde_json::to_string(ids).unwrap_or_else(|_| "null".to_string());
            sqlx::query("UPDATE attribute SET default_option_ids = ? WHERE id = ?")
                .bind(&json)
                .bind(attr.id)
                .execute(&mut *tx)
                .await
                .map_err(|e| AppError::database(e.to_string()))?;
        }
    }

    // ── INSERT zones ──
    for z in &catalog.zones {
        sqlx::query("INSERT INTO zone (id, name, description, is_active) VALUES (?, ?, ?, ?)")
            .bind(z.id)
            .bind(&z.name)
            .bind(&z.description)
            .bind(z.is_active)
            .execute(&mut *tx)
            .await
            .map_err(|e| AppError::database(e.to_string()))?;
    }

    // ── INSERT dining_tables (after zones, FK → zone_id) ──
    for dt in &catalog.dining_tables {
        sqlx::query(
            "INSERT INTO dining_table (id, name, zone_id, capacity, is_active) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(dt.id)
        .bind(&dt.name)
        .bind(dt.zone_id)
        .bind(dt.capacity)
        .bind(dt.is_active)
        .execute(&mut *tx)
        .await
        .map_err(|e| AppError::database(e.to_string()))?;
    }

    // ── INSERT price_rules ──
    for pr in &catalog.price_rules {
        let active_days_json = pr
            .active_days
            .as_ref()
            .map(|v| serde_json::to_string(v).unwrap_or_else(|_| "null".to_string()));

        sqlx::query(
            "INSERT INTO price_rule (id, name, receipt_name, description, rule_type, product_scope, target_id, zone_scope, adjustment_type, adjustment_value, is_stackable, is_exclusive, valid_from, valid_until, active_days, active_start_time, active_end_time, is_active, created_by, created_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20)",
        )
        .bind(pr.id)
        .bind(&pr.name)
        .bind(&pr.receipt_name)
        .bind(&pr.description)
        .bind(&pr.rule_type)
        .bind(&pr.product_scope)
        .bind(pr.target_id)
        .bind(&pr.zone_scope)
        .bind(&pr.adjustment_type)
        .bind(pr.adjustment_value)
        .bind(pr.is_stackable)
        .bind(pr.is_exclusive)
        .bind(pr.valid_from)
        .bind(pr.valid_until)
        .bind(&active_days_json)
        .bind(&pr.active_start_time)
        .bind(&pr.active_end_time)
        .bind(pr.is_active)
        .bind(pr.created_by)
        .bind(pr.created_at)
        .execute(&mut *tx)
        .await
        .map_err(|e| AppError::database(e.to_string()))?;
    }

    // ── INSERT attribute bindings ──
    for binding in &catalog.attribute_bindings {
        let default_option_ids_json = binding
            .default_option_ids
            .as_ref()
            .map(|v| serde_json::to_string(v).unwrap_or_else(|_| "null".to_string()));

        sqlx::query(
            "INSERT INTO attribute_binding (id, owner_type, owner_id, attribute_id, is_required, display_order, default_option_ids) \
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(binding.id)
        .bind(&binding.owner_type)
        .bind(binding.owner_id)
        .bind(binding.attribute_id)
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
                    t.id,
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
                c.id,
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
                p.id,
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
                    a.id,
                    Some(a),
                    false,
                )
                .await;
        }
    }

    // Zones
    if let Ok(zones) = zone::find_all_with_inactive(&state.pool).await {
        for z in &zones {
            state
                .broadcast_sync(
                    SyncResource::Zone,
                    SyncChangeType::Updated,
                    z.id,
                    Some(z),
                    false,
                )
                .await;
        }
    }

    // Dining tables
    if let Ok(tables) = dining_table::find_all_with_inactive(&state.pool).await {
        for dt in &tables {
            state
                .broadcast_sync(
                    SyncResource::DiningTable,
                    SyncChangeType::Updated,
                    dt.id,
                    Some(dt),
                    false,
                )
                .await;
        }
    }

    // Price rules
    if let Ok(rules) = price_rule::find_all_with_inactive(&state.pool).await {
        for pr in &rules {
            state
                .broadcast_sync(
                    SyncResource::PriceRule,
                    SyncChangeType::Updated,
                    pr.id,
                    Some(pr),
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
                    b.id,
                    Some(b),
                    false,
                )
                .await;
        }
    }
}
