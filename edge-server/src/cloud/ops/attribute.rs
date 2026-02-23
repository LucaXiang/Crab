//! Attribute + Binding + Tag operations (via repository)

use shared::cloud::catalog::{BindingOwner, CatalogOpResult};
use shared::models::attribute::{AttributeCreate, AttributeUpdate};

use crate::core::state::ServerState;

// ── Attribute ──

pub async fn create(
    state: &ServerState,
    assigned_id: Option<i64>,
    data: AttributeCreate,
) -> CatalogOpResult {
    use crate::db::repository::attribute;

    match attribute::create(&state.pool, assigned_id, data).await {
        Ok(attr) => {
            state
                .broadcast_sync("attribute", "created", &attr.id.to_string(), Some(&attr))
                .await;
            CatalogOpResult::created(attr.id)
        }
        Err(e) => CatalogOpResult::err(e.to_string()),
    }
}

pub async fn update(state: &ServerState, id: i64, data: AttributeUpdate) -> CatalogOpResult {
    use crate::db::repository::attribute;

    match attribute::update(&state.pool, id, data).await {
        Ok(attr) => {
            state
                .broadcast_sync("attribute", "updated", &attr.id.to_string(), Some(&attr))
                .await;
            CatalogOpResult::ok()
        }
        Err(e) => CatalogOpResult::err(e.to_string()),
    }
}

pub async fn delete(state: &ServerState, id: i64) -> CatalogOpResult {
    use crate::db::repository::attribute;

    // attribute_option 和 attribute_binding 都有 ON DELETE CASCADE，无需检查引用
    match attribute::delete(&state.pool, id).await {
        Ok(_) => {
            state
                .broadcast_sync::<()>("attribute", "deleted", &id.to_string(), None)
                .await;
            CatalogOpResult::ok()
        }
        Err(e) => CatalogOpResult::err(e.to_string()),
    }
}

// ── Attribute Binding ──

pub async fn bind(
    state: &ServerState,
    owner: &BindingOwner,
    attribute_id: i64,
    is_required: bool,
    display_order: i32,
    default_option_ids: Option<Vec<i32>>,
) -> CatalogOpResult {
    use crate::db::repository::attribute;

    match attribute::link(
        &state.pool,
        owner.owner_type(),
        owner.owner_id(),
        attribute_id,
        is_required,
        display_order,
        default_option_ids,
    )
    .await
    {
        Ok(binding) => {
            // Refresh affected product/category caches
            match owner {
                BindingOwner::Product(pid) => {
                    let _ = state.catalog_service.refresh_product_cache(*pid).await;
                }
                BindingOwner::Category(cid) => {
                    let _ = state
                        .catalog_service
                        .refresh_products_in_category(*cid)
                        .await;
                }
            }

            state
                .broadcast_sync(
                    "attribute_binding",
                    "created",
                    &binding.id.to_string(),
                    Some(&binding),
                )
                .await;
            CatalogOpResult::created(binding.id)
        }
        Err(e) => CatalogOpResult::err(e.to_string()),
    }
}

pub async fn unbind(state: &ServerState, binding_id: i64) -> CatalogOpResult {
    use crate::db::repository::attribute;

    // Get binding before delete to know what to refresh
    let binding = match attribute::find_binding_by_id(&state.pool, binding_id).await {
        Ok(b) => b,
        Err(e) => return CatalogOpResult::err(e.to_string()),
    };

    if let Err(e) = attribute::delete_binding(&state.pool, binding_id).await {
        return CatalogOpResult::err(e.to_string());
    }

    if let Some(b) = binding {
        match b.owner_type.as_str() {
            "product" => {
                let _ = state
                    .catalog_service
                    .refresh_product_cache(b.owner_id)
                    .await;
            }
            "category" => {
                let _ = state
                    .catalog_service
                    .refresh_products_in_category(b.owner_id)
                    .await;
            }
            _ => {}
        }
    }

    state
        .broadcast_sync::<()>(
            "attribute_binding",
            "deleted",
            &binding_id.to_string(),
            None,
        )
        .await;
    CatalogOpResult::ok()
}

// ── Tag ──

pub async fn create_tag(
    state: &ServerState,
    assigned_id: Option<i64>,
    data: &shared::models::tag::TagCreate,
) -> CatalogOpResult {
    use crate::db::repository::tag;

    match tag::create(&state.pool, assigned_id, data.clone()).await {
        Ok(t) => {
            state
                .broadcast_sync("tag", "created", &t.id.to_string(), Some(&t))
                .await;
            CatalogOpResult::created(t.id)
        }
        Err(e) => CatalogOpResult::err(e.to_string()),
    }
}

pub async fn update_tag(
    state: &ServerState,
    id: i64,
    data: &shared::models::tag::TagUpdate,
) -> CatalogOpResult {
    use crate::db::repository::tag;

    match tag::update(&state.pool, id, data.clone()).await {
        Ok(t) => {
            state
                .broadcast_sync("tag", "updated", &t.id.to_string(), Some(&t))
                .await;
            CatalogOpResult::ok()
        }
        Err(e) => CatalogOpResult::err(e.to_string()),
    }
}

pub async fn delete_tag(state: &ServerState, id: i64) -> CatalogOpResult {
    use crate::db::repository::tag;

    // product_tag 和 category_tag 都有 ON DELETE CASCADE，无需检查引用
    match tag::delete(&state.pool, id).await {
        Ok(_) => {
            state
                .broadcast_sync::<()>("tag", "deleted", &id.to_string(), None)
                .await;
            CatalogOpResult::ok()
        }
        Err(e) => CatalogOpResult::err(e.to_string()),
    }
}
