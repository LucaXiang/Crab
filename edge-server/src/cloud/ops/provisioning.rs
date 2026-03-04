//! FullSync (store provisioning) + EnsureImage + CatalogSyncData
//!
//! FullSync: Cloud→Edge StoreOp with StoreSnapshot (Create types, no IDs).
//! CatalogSyncData: Cloud→Edge full catalog with real IDs (re-bind scenario).

use shared::cloud::SyncResource;
use shared::cloud::store_op::{BindingOwner, FullSyncResult, StoreOpResult, StoreSnapshot};
use shared::message::SyncChangeType;

use crate::core::state::ServerState;

use super::attribute;

/// Clear all catalog tables in SQLite before FullSync rebuild.
/// Uses FK-safe deletion order: bindings → products → categories → attributes → tags
async fn clear_catalog(state: &ServerState) -> Result<(), String> {
    let pool = &state.pool;
    // attribute_binding (references products/categories + attributes)
    sqlx::query("DELETE FROM attribute_binding")
        .execute(pool)
        .await
        .map_err(|e| format!("clear attribute_binding: {e}"))?;
    // product_spec (references products)
    sqlx::query("DELETE FROM product_spec")
        .execute(pool)
        .await
        .map_err(|e| format!("clear product_spec: {e}"))?;
    // product_tag (references products + tags)
    sqlx::query("DELETE FROM product_tag")
        .execute(pool)
        .await
        .map_err(|e| format!("clear product_tag: {e}"))?;
    // product
    sqlx::query("DELETE FROM product")
        .execute(pool)
        .await
        .map_err(|e| format!("clear product: {e}"))?;
    // category_print_destination (references categories)
    sqlx::query("DELETE FROM category_print_destination")
        .execute(pool)
        .await
        .map_err(|e| format!("clear category_print_destination: {e}"))?;
    // category_tag (references categories + tags)
    sqlx::query("DELETE FROM category_tag")
        .execute(pool)
        .await
        .map_err(|e| format!("clear category_tag: {e}"))?;
    // category
    sqlx::query("DELETE FROM category")
        .execute(pool)
        .await
        .map_err(|e| format!("clear category: {e}"))?;
    // attribute_option (references attributes)
    sqlx::query("DELETE FROM attribute_option")
        .execute(pool)
        .await
        .map_err(|e| format!("clear attribute_option: {e}"))?;
    // attribute
    sqlx::query("DELETE FROM attribute")
        .execute(pool)
        .await
        .map_err(|e| format!("clear attribute: {e}"))?;
    // tag
    sqlx::query("DELETE FROM tag")
        .execute(pool)
        .await
        .map_err(|e| format!("clear tag: {e}"))?;

    // Invalidate CatalogService cache since we cleared everything
    state.catalog_service.invalidate();

    tracing::info!("Catalog cleared for FullSync rebuild");
    Ok(())
}

async fn broadcast_catalog_refresh(state: &ServerState) {
    let resources = [
        SyncResource::Tag,
        SyncResource::Category,
        SyncResource::Product,
        SyncResource::Attribute,
        SyncResource::Zone,
        SyncResource::DiningTable,
        SyncResource::PriceRule,
    ];

    for resource in resources {
        state
            .broadcast_sync::<()>(resource, SyncChangeType::Updated, 0, None, true)
            .await;
    }
}

/// Fire-and-forget image download
pub fn ensure_image(state: &ServerState, presigned_url: &str, hash: &str) -> StoreOpResult {
    let images_dir = state.work_dir().join("images");
    let url = presigned_url.to_string();
    let h = hash.to_string();
    tokio::spawn(async move {
        crate::services::image_download::download_and_save(&url, &h, &images_dir).await;
    });
    StoreOpResult::ok()
}

/// Apply a full store snapshot (initial provisioning)
pub async fn apply_full_sync(state: &ServerState, snapshot: &StoreSnapshot) -> StoreOpResult {
    match do_full_sync(state, snapshot).await {
        Ok(result) => {
            let success = result.errors.is_empty();
            StoreOpResult {
                success,
                created_id: None,
                data: None,
                error: if result.errors.is_empty() {
                    None
                } else {
                    Some(result.errors.join("; "))
                },
            }
        }
        Err(e) => StoreOpResult::err(e),
    }
}

async fn do_full_sync(
    state: &ServerState,
    snapshot: &StoreSnapshot,
) -> Result<FullSyncResult, String> {
    let mut result = FullSyncResult {
        tags_created: 0,
        categories_created: 0,
        products_created: 0,
        attributes_created: 0,
        bindings_created: 0,
        errors: vec![],
    };

    // 0. Clear existing catalog data (FK reverse order) before rebuilding
    if let Err(e) = clear_catalog(state).await {
        return Err(format!("Failed to clear catalog: {e}"));
    }

    // 1. Tags first (products/categories may reference them)
    for tag_data in &snapshot.tags {
        let r = attribute::create_tag(state, None, tag_data).await;
        if r.success {
            result.tags_created += 1;
        } else if let Some(e) = r.error {
            result.errors.push(format!("Tag '{}': {e}", tag_data.name));
        }
    }

    // 2. Attributes (bindings reference them)
    let mut attr_id_map: Vec<i64> = Vec::new();
    for attr_item in &snapshot.attributes {
        let r = attribute::create(state, None, attr_item.data.clone()).await;
        if r.success {
            attr_id_map.push(r.created_id.unwrap_or(-1));
            result.attributes_created += 1;
        } else {
            attr_id_map.push(-1);
            if let Some(e) = r.error {
                result
                    .errors
                    .push(format!("Attribute '{}': {e}", attr_item.data.name));
            }
        }
    }

    // 3. Categories
    let mut cat_id_map: Vec<i64> = Vec::new();
    for cat_item in &snapshot.categories {
        match state
            .catalog_service
            .create_category(None, cat_item.data.clone())
            .await
        {
            Ok(c) => {
                state
                    .broadcast_sync(
                        shared::cloud::SyncResource::Category,
                        SyncChangeType::Created,
                        c.id,
                        Some(&c),
                        true,
                    )
                    .await;
                let cat_id = c.id;
                cat_id_map.push(cat_id);
                result.categories_created += 1;

                // Create bindings for this category
                for b in &cat_item.attribute_bindings {
                    if let Some(&attr_id) = attr_id_map.get(b.attribute_index)
                        && attr_id > 0
                    {
                        let r = attribute::bind(
                            state,
                            &BindingOwner::Category(cat_id),
                            attr_id,
                            b.is_required,
                            b.display_order,
                            b.default_option_ids.clone(),
                        )
                        .await;
                        if r.success {
                            result.bindings_created += 1;
                        } else if let Some(e) = r.error {
                            result.errors.push(format!("Category binding: {e}"));
                        }
                    }
                }
            }
            Err(e) => {
                cat_id_map.push(-1);
                result
                    .errors
                    .push(format!("Category '{}': {e}", cat_item.data.name));
            }
        }
    }

    // 4. Products (reference categories by index)
    for prod_item in &snapshot.products {
        let cat_id = cat_id_map
            .get(prod_item.category_index)
            .copied()
            .unwrap_or(-1);
        if cat_id <= 0 {
            result.errors.push(format!(
                "Product '{}': category index {} invalid",
                prod_item.data.name, prod_item.category_index
            ));
            continue;
        }

        let mut data = prod_item.data.clone();
        data.category_id = cat_id;

        match state.catalog_service.create_product(None, data).await {
            Ok(p) => {
                state
                    .broadcast_sync(
                        shared::cloud::SyncResource::Product,
                        SyncChangeType::Created,
                        p.id,
                        Some(&p),
                        true,
                    )
                    .await;
                let prod_id = p.id;
                result.products_created += 1;

                // Create bindings for this product
                for b in &prod_item.attribute_bindings {
                    if let Some(&attr_id) = attr_id_map.get(b.attribute_index)
                        && attr_id > 0
                    {
                        let r = attribute::bind(
                            state,
                            &BindingOwner::Product(prod_id),
                            attr_id,
                            b.is_required,
                            b.display_order,
                            b.default_option_ids.clone(),
                        )
                        .await;
                        if r.success {
                            result.bindings_created += 1;
                        } else if let Some(e) = r.error {
                            result.errors.push(format!("Product binding: {e}"));
                        }
                    }
                }
            }
            Err(e) => {
                result
                    .errors
                    .push(format!("Product '{}': {e}", prod_item.data.name));
            }
        }
    }

    broadcast_catalog_refresh(state).await;
    Ok(result)
}

/// Apply CatalogSyncData from Cloud (re-bind scenario).
///
/// Clears local catalog and inserts all items from the CatalogExport with their
/// original Cloud IDs. This ensures Edge and Cloud share the same IDs for
/// bidirectional sync.
pub async fn apply_catalog_sync_data(
    state: &ServerState,
    catalog: &shared::models::CatalogExport,
) -> Result<(), String> {
    // Block import when active orders exist
    let active = state
        .orders_manager
        .get_active_orders()
        .map_err(|e| format!("Failed to check active orders: {e}"))?;
    if !active.is_empty() {
        return Err(format!(
            "Cannot import catalog: {} active orders exist",
            active.len()
        ));
    }

    // Validate referential integrity
    shared::models::validate_catalog(catalog)
        .map_err(|e| format!("CatalogSyncData validation failed: {e}"))?;

    // Use the same import logic as data_transfer (clear + insert in transaction)
    crate::api::data_transfer::import_catalog_data(state, catalog)
        .await
        .map_err(|e| format!("CatalogSyncData import failed: {e}"))?;

    // Refresh catalog cache
    state
        .catalog_service
        .warmup()
        .await
        .map_err(|e| format!("CatalogService warmup failed: {e}"))?;

    broadcast_catalog_refresh(state).await;

    tracing::info!(
        tags = catalog.tags.len(),
        categories = catalog.categories.len(),
        products = catalog.products.len(),
        attributes = catalog.attributes.len(),
        "CatalogSyncData applied successfully"
    );

    Ok(())
}
