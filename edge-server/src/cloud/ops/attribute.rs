//! Attribute + Binding + Tag operations (via repository)

use shared::cloud::SyncResource;
use shared::cloud::store_op::{BindingOwner, StoreOpResult};
use shared::models::attribute::{AttributeCreate, AttributeUpdate};

use crate::core::state::ServerState;

// ── Attribute ──

pub async fn create(
    state: &ServerState,
    assigned_id: Option<i64>,
    data: AttributeCreate,
) -> StoreOpResult {
    use crate::db::repository::attribute;

    match attribute::create(&state.pool, assigned_id, data).await {
        Ok(attr) => {
            state
                .broadcast_sync(
                    SyncResource::Attribute,
                    "created",
                    &attr.id.to_string(),
                    Some(&attr),
                )
                .await;
            StoreOpResult::created(attr.id)
        }
        Err(e) => StoreOpResult::err(e.to_string()),
    }
}

pub async fn update(state: &ServerState, id: i64, data: AttributeUpdate) -> StoreOpResult {
    use crate::db::repository::attribute;

    match attribute::update(&state.pool, id, data).await {
        Ok(attr) => {
            state
                .broadcast_sync(
                    SyncResource::Attribute,
                    "updated",
                    &attr.id.to_string(),
                    Some(&attr),
                )
                .await;
            StoreOpResult::ok()
        }
        Err(e) => StoreOpResult::err(e.to_string()),
    }
}

pub async fn delete(state: &ServerState, id: i64) -> StoreOpResult {
    use crate::db::repository::attribute;

    // attribute_option 和 attribute_binding 都有 ON DELETE CASCADE，无需检查引用
    match attribute::delete(&state.pool, id).await {
        Ok(_) => {
            state
                .broadcast_sync::<()>(SyncResource::Attribute, "deleted", &id.to_string(), None)
                .await;
            StoreOpResult::ok()
        }
        Err(e) => StoreOpResult::err(e.to_string()),
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
) -> StoreOpResult {
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
                    SyncResource::AttributeBinding,
                    "created",
                    &binding.id.to_string(),
                    Some(&binding),
                )
                .await;
            StoreOpResult::created(binding.id)
        }
        Err(e) => StoreOpResult::err(e.to_string()),
    }
}

pub async fn unbind(state: &ServerState, binding_id: i64) -> StoreOpResult {
    use crate::db::repository::attribute;

    // Get binding before delete to know what to refresh
    let binding = match attribute::find_binding_by_id(&state.pool, binding_id).await {
        Ok(b) => b,
        Err(e) => return StoreOpResult::err(e.to_string()),
    };

    if let Err(e) = attribute::delete_binding(&state.pool, binding_id).await {
        return StoreOpResult::err(e.to_string());
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
            SyncResource::AttributeBinding,
            "deleted",
            &binding_id.to_string(),
            None,
        )
        .await;
    StoreOpResult::ok()
}

// ── Tag ──

pub async fn create_tag(
    state: &ServerState,
    assigned_id: Option<i64>,
    data: &shared::models::tag::TagCreate,
) -> StoreOpResult {
    use crate::db::repository::tag;

    match tag::create(&state.pool, assigned_id, data.clone()).await {
        Ok(t) => {
            state
                .broadcast_sync(SyncResource::Tag, "created", &t.id.to_string(), Some(&t))
                .await;
            StoreOpResult::created(t.id)
        }
        Err(e) => StoreOpResult::err(e.to_string()),
    }
}

pub async fn update_tag(
    state: &ServerState,
    id: i64,
    data: &shared::models::tag::TagUpdate,
) -> StoreOpResult {
    use crate::db::repository::tag;

    match tag::update(&state.pool, id, data.clone()).await {
        Ok(t) => {
            state
                .broadcast_sync(SyncResource::Tag, "updated", &t.id.to_string(), Some(&t))
                .await;
            StoreOpResult::ok()
        }
        Err(e) => StoreOpResult::err(e.to_string()),
    }
}

pub async fn delete_tag(state: &ServerState, id: i64) -> StoreOpResult {
    use crate::db::repository::tag;

    // product_tag 和 category_tag 都有 ON DELETE CASCADE，无需检查引用
    match tag::delete(&state.pool, id).await {
        Ok(_) => {
            state
                .broadcast_sync::<()>(SyncResource::Tag, "deleted", &id.to_string(), None)
                .await;
            StoreOpResult::ok()
        }
        Err(e) => StoreOpResult::err(e.to_string()),
    }
}
