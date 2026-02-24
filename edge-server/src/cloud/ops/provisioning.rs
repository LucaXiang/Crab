//! FullSync (initial store provisioning) + EnsureImage

use shared::cloud::store_op::{BindingOwner, FullSyncResult, StoreOpResult, StoreSnapshot};
use shared::message::SyncChangeType;

use crate::core::state::ServerState;

use super::attribute;

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
                        &c.id.to_string(),
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
                        &p.id.to_string(),
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

    Ok(result)
}
