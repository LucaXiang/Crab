//! Catalog data transfer — export/import via ZIP
//!
//! - GET  /api/tenant/stores/{id}/data-transfer/export → application/zip
//! - POST /api/tenant/stores/{id}/data-transfer/import ← application/zip

use std::collections::{HashMap, HashSet};
use std::io::{Cursor, Read, Write};

use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::header;
use axum::response::IntoResponse;
use axum::{Extension, Json};
use shared::cloud::CloudMessage;
use shared::cloud::store_op::StoreOpResult;
use shared::error::AppError;
use shared::models::{
    Attribute, AttributeBinding, AttributeOption, Category, DiningTable, PriceRule, ProductFull,
    ProductSpec, Tag, Zone, validate_catalog,
};
use zip::ZipArchive;
use zip::write::FileOptions;

use crate::auth::tenant_auth::TenantIdentity;
use crate::db::store;
use crate::db::store::data_transfer::CatalogExport;
use crate::state::AppState;

use super::{internal, verify_store};

// ── Reusable catalog builders for WS connect + CatalogSyncData ──

/// Build a CatalogExport from PG catalog data (all resources with real source_ids).
/// Used by ws.rs for RequestCatalogSync response.
pub async fn build_catalog_export(
    pool: &sqlx::PgPool,
    store_id: i64,
) -> Result<CatalogExport, Box<dyn std::error::Error + Send + Sync>> {
    let store_tags = store::list_tags(pool, store_id).await?;
    let store_categories = store::list_categories(pool, store_id).await?;
    let store_products = store::list_products(pool, store_id).await?;
    let store_attributes = store::list_attributes(pool, store_id).await?;
    let store_bindings = store::list_all_bindings(pool, store_id).await?;
    let zones: Vec<Zone> = store::list_zones(pool, store_id).await?;
    let dining_tables: Vec<DiningTable> = store::list_tables(pool, store_id).await?;
    let price_rules: Vec<PriceRule> = store::list_price_rules(pool, store_id).await?;

    // Build tag lookup: source_id → Tag
    let tag_map: HashMap<i64, Tag> = store_tags
        .iter()
        .map(|t| {
            (
                t.source_id,
                Tag {
                    id: t.source_id,
                    name: t.name.clone(),
                    color: t.color.clone(),
                    display_order: t.display_order,
                    is_active: t.is_active,
                    is_system: t.is_system,
                },
            )
        })
        .collect();

    let tags: Vec<Tag> = tag_map.values().cloned().collect();

    let categories: Vec<Category> = store_categories
        .iter()
        .map(|c| Category {
            id: c.source_id,
            name: c.name.clone(),
            sort_order: c.sort_order,
            is_kitchen_print_enabled: c.is_kitchen_print_enabled,
            is_label_print_enabled: c.is_label_print_enabled,
            is_active: c.is_active,
            is_virtual: c.is_virtual,
            match_mode: c.match_mode.clone(),
            is_display: c.is_display,
            kitchen_print_destinations: c.kitchen_print_destinations.clone(),
            label_print_destinations: c.label_print_destinations.clone(),
            tag_ids: c.tag_ids.clone(),
        })
        .collect();

    let products: Vec<ProductFull> = store_products
        .iter()
        .map(|p| {
            let product_tags: Vec<Tag> = p
                .tag_ids
                .iter()
                .filter_map(|tid| tag_map.get(tid).cloned())
                .collect();
            ProductFull {
                id: p.source_id,
                name: p.name.clone(),
                image: p.image.clone(),
                category_id: p.category_source_id,
                sort_order: p.sort_order,
                tax_rate: p.tax_rate,
                receipt_name: p.receipt_name.clone(),
                kitchen_print_name: p.kitchen_print_name.clone(),
                is_kitchen_print_enabled: p.is_kitchen_print_enabled,
                is_label_print_enabled: p.is_label_print_enabled,
                is_active: p.is_active,
                external_id: p.external_id,
                specs: p
                    .specs
                    .iter()
                    .map(|s| ProductSpec {
                        id: s.source_id,
                        product_id: p.source_id,
                        name: s.name.clone(),
                        price: s.price,
                        display_order: s.display_order,
                        is_default: s.is_default,
                        is_active: s.is_active,
                        receipt_name: s.receipt_name.clone(),
                        is_root: s.is_root,
                    })
                    .collect(),
                attributes: vec![],
                tags: product_tags,
            }
        })
        .collect();

    let attributes: Vec<Attribute> = store_attributes
        .iter()
        .map(|a| Attribute {
            id: a.source_id,
            name: a.name.clone(),
            is_multi_select: a.is_multi_select,
            max_selections: a.max_selections,
            default_option_ids: a.default_option_ids.clone(),
            display_order: a.display_order,
            is_active: a.is_active,
            show_on_receipt: a.show_on_receipt,
            receipt_name: a.receipt_name.clone(),
            show_on_kitchen_print: a.show_on_kitchen_print,
            kitchen_print_name: a.kitchen_print_name.clone(),
            options: a
                .options
                .iter()
                .map(|o| AttributeOption {
                    id: o.source_id,
                    attribute_id: a.source_id,
                    name: o.name.clone(),
                    price_modifier: o.price_modifier,
                    display_order: o.display_order,
                    is_active: o.is_active,
                    receipt_name: o.receipt_name.clone(),
                    kitchen_print_name: o.kitchen_print_name.clone(),
                    enable_quantity: o.enable_quantity,
                    max_quantity: o.max_quantity,
                })
                .collect(),
        })
        .collect();

    let attribute_bindings: Vec<AttributeBinding> = store_bindings
        .iter()
        .map(|b| {
            let default_ids: Option<Vec<i64>> = b
                .default_option_ids
                .as_ref()
                .and_then(|v| serde_json::from_value(v.clone()).ok());
            AttributeBinding {
                id: b.source_id,
                owner_type: b.owner_type.clone(),
                owner_id: b.owner_source_id,
                attribute_id: b.attribute_source_id,
                is_required: b.is_required,
                display_order: b.display_order,
                default_option_ids: default_ids,
            }
        })
        .collect();

    Ok(CatalogExport {
        version: 1,
        exported_at: shared::util::now_millis(),
        tags,
        categories,
        products,
        attributes,
        attribute_bindings,
        price_rules,
        zones,
        dining_tables,
    })
}

// ── Export handler ──

pub async fn export_catalog(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path(store_id): Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    verify_store(&state, store_id, identity.tenant_id).await?;

    let store_tags = store::list_tags(&state.pool, store_id)
        .await
        .map_err(internal)?;
    let store_categories = store::list_categories(&state.pool, store_id)
        .await
        .map_err(internal)?;
    let store_products = store::list_products(&state.pool, store_id)
        .await
        .map_err(internal)?;
    let store_attributes = store::list_attributes(&state.pool, store_id)
        .await
        .map_err(internal)?;
    let store_bindings = store::list_all_bindings(&state.pool, store_id)
        .await
        .map_err(internal)?;
    let zones: Vec<Zone> = store::list_zones(&state.pool, store_id)
        .await
        .map_err(internal)?;
    let dining_tables: Vec<DiningTable> = store::list_tables(&state.pool, store_id)
        .await
        .map_err(internal)?;
    let price_rules: Vec<PriceRule> = store::list_price_rules(&state.pool, store_id)
        .await
        .map_err(internal)?;

    // Build tag lookup: source_id → Tag (for ProductFull.tags)
    let tag_map: HashMap<i64, Tag> = store_tags
        .iter()
        .map(|t| {
            (
                t.source_id,
                Tag {
                    id: t.source_id,
                    name: t.name.clone(),
                    color: t.color.clone(),
                    display_order: t.display_order,
                    is_active: t.is_active,
                    is_system: t.is_system,
                },
            )
        })
        .collect();

    // Convert Store* → shared::models
    let tags: Vec<Tag> = tag_map.values().cloned().collect();

    let categories: Vec<Category> = store_categories
        .iter()
        .map(|c| Category {
            id: c.source_id,
            name: c.name.clone(),
            sort_order: c.sort_order,
            is_kitchen_print_enabled: c.is_kitchen_print_enabled,
            is_label_print_enabled: c.is_label_print_enabled,
            is_active: c.is_active,
            is_virtual: c.is_virtual,
            match_mode: c.match_mode.clone(),
            is_display: c.is_display,
            kitchen_print_destinations: c.kitchen_print_destinations.clone(),
            label_print_destinations: c.label_print_destinations.clone(),
            tag_ids: c.tag_ids.clone(),
        })
        .collect();

    let products: Vec<ProductFull> = store_products
        .iter()
        .map(|p| {
            let product_tags: Vec<Tag> = p
                .tag_ids
                .iter()
                .filter_map(|tid| tag_map.get(tid).cloned())
                .collect();
            ProductFull {
                id: p.source_id,
                name: p.name.clone(),
                image: p.image.clone(),
                category_id: p.category_source_id,
                sort_order: p.sort_order,
                tax_rate: p.tax_rate,
                receipt_name: p.receipt_name.clone(),
                kitchen_print_name: p.kitchen_print_name.clone(),
                is_kitchen_print_enabled: p.is_kitchen_print_enabled,
                is_label_print_enabled: p.is_label_print_enabled,
                is_active: p.is_active,
                external_id: p.external_id,
                specs: p
                    .specs
                    .iter()
                    .map(|s| ProductSpec {
                        id: s.source_id,
                        product_id: p.source_id,
                        name: s.name.clone(),
                        price: s.price,
                        display_order: s.display_order,
                        is_default: s.is_default,
                        is_active: s.is_active,
                        receipt_name: s.receipt_name.clone(),
                        is_root: s.is_root,
                    })
                    .collect(),
                attributes: vec![], // Not needed for catalog export
                tags: product_tags,
            }
        })
        .collect();

    let attributes: Vec<Attribute> = store_attributes
        .iter()
        .map(|a| Attribute {
            id: a.source_id,
            name: a.name.clone(),
            is_multi_select: a.is_multi_select,
            max_selections: a.max_selections,
            default_option_ids: a.default_option_ids.clone(),
            display_order: a.display_order,
            is_active: a.is_active,
            show_on_receipt: a.show_on_receipt,
            receipt_name: a.receipt_name.clone(),
            show_on_kitchen_print: a.show_on_kitchen_print,
            kitchen_print_name: a.kitchen_print_name.clone(),
            options: a
                .options
                .iter()
                .map(|o| AttributeOption {
                    id: o.source_id,
                    attribute_id: a.source_id,
                    name: o.name.clone(),
                    price_modifier: o.price_modifier,
                    display_order: o.display_order,
                    is_active: o.is_active,
                    receipt_name: o.receipt_name.clone(),
                    kitchen_print_name: o.kitchen_print_name.clone(),
                    enable_quantity: o.enable_quantity,
                    max_quantity: o.max_quantity,
                })
                .collect(),
        })
        .collect();

    let attribute_bindings: Vec<AttributeBinding> = store_bindings
        .iter()
        .map(|b| {
            let default_ids: Option<Vec<i64>> = b
                .default_option_ids
                .as_ref()
                .and_then(|v| serde_json::from_value(v.clone()).ok());
            AttributeBinding {
                id: b.source_id,
                owner_type: b.owner_type.clone(),
                owner_id: b.owner_source_id,
                attribute_id: b.attribute_source_id,
                is_required: b.is_required,
                display_order: b.display_order,
                default_option_ids: default_ids,
            }
        })
        .collect();

    let catalog = CatalogExport {
        version: 1,
        exported_at: shared::util::now_millis(),
        tags,
        categories,
        products,
        attributes,
        attribute_bindings,
        price_rules,
        zones,
        dining_tables,
    };

    let catalog_json =
        serde_json::to_vec_pretty(&catalog).map_err(|e| AppError::internal(e.to_string()))?;

    // Build ZIP in memory
    let mut buf = Cursor::new(Vec::new());
    {
        let mut zip = zip::ZipWriter::new(&mut buf);
        let options: FileOptions<()> =
            FileOptions::default().compression_method(zip::CompressionMethod::Deflated);

        zip.start_file("catalog.json", options)
            .map_err(|e| AppError::internal(e.to_string()))?;
        zip.write_all(&catalog_json)
            .map_err(|e| AppError::internal(e.to_string()))?;
        zip.finish()
            .map_err(|e| AppError::internal(e.to_string()))?;
    }

    Ok((
        [
            (header::CONTENT_TYPE, "application/zip"),
            (
                header::CONTENT_DISPOSITION,
                "attachment; filename=\"catalog_export.zip\"",
            ),
        ],
        buf.into_inner(),
    ))
}

// ── Import handler ──

pub async fn import_catalog(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path(store_id): Path<i64>,
    body: Bytes,
) -> Result<Json<StoreOpResult>, AppError> {
    verify_store(&state, store_id, identity.tenant_id).await?;

    // Parse ZIP
    let cursor = Cursor::new(body.as_ref());
    let mut archive =
        ZipArchive::new(cursor).map_err(|e| AppError::validation(format!("Invalid ZIP: {e}")))?;

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

    // Validate referential integrity before touching the database
    validate_catalog(&catalog)
        .map_err(|e| AppError::validation(format!("Catalog validation failed: {e}")))?;

    // Collect old image hashes before import replaces them
    let old_hashes: HashSet<String> =
        crate::db::tenant_images::get_all_product_images(&state.pool, store_id)
            .await
            .unwrap_or_default()
            .into_iter()
            .collect();

    // Import within transaction
    store::data_transfer::import_catalog(&state.pool, store_id, &catalog)
        .await
        .map_err(internal)?;

    store::increment_store_version(&state.pool, store_id)
        .await
        .map_err(internal)?;

    // Register image refs and notify edge to download new images
    let new_hashes: HashSet<&str> = catalog
        .products
        .iter()
        .filter_map(|p| {
            if p.image.is_empty() {
                None
            } else {
                Some(p.image.as_str())
            }
        })
        .collect();
    let now = shared::util::now_millis();
    for hash in &new_hashes {
        if !old_hashes.contains(*hash) {
            let _ = crate::db::tenant_images::increment_ref(&state.pool, identity.tenant_id, hash)
                .await;
        }
        super::fire_ensure_image(&state, store_id, identity.tenant_id, Some(hash)).await;
    }
    // Decrement refs for old images no longer used
    for old_hash in &old_hashes {
        if !new_hashes.contains(old_hash.as_str()) {
            let _ = crate::db::tenant_images::decrement_ref(
                &state.pool,
                identity.tenant_id,
                old_hash,
                now,
            )
            .await;
        }
    }

    // Push CatalogSyncData to edge (preserves real IDs for bidirectional sync).
    // If edge is online, send directly via WebSocket channel.
    // If offline, edge will pull full catalog via RequestCatalogSync on reconnect.
    let msg = CloudMessage::CatalogSyncData {
        catalog: Box::new(catalog),
        recovery_state: None,
    };
    if let Some(sender) = state.edges.connected.get(&store_id)
        && sender.try_send(msg).is_err()
    {
        tracing::warn!(
            store_id,
            "WS channel full after import, edge will sync on reconnect"
        );
    }
    // Clear any stale pending_ops — full import supersedes all queued ops
    let _ = crate::db::store::pending_ops::delete_all(&state.pool, store_id).await;

    Ok(Json(StoreOpResult::ok()))
}
