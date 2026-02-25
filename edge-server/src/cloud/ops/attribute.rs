//! Attribute + Binding + Tag operations (via repository)

use shared::cloud::SyncResource;
use shared::cloud::store_op::{BindingOwner, StoreOpResult};
use shared::message::SyncChangeType;
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
                    SyncChangeType::Created,
                    &attr.id.to_string(),
                    Some(&attr),
                    true,
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
                    SyncChangeType::Updated,
                    &attr.id.to_string(),
                    Some(&attr),
                    true,
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
                .broadcast_sync::<()>(
                    SyncResource::Attribute,
                    SyncChangeType::Deleted,
                    &id.to_string(),
                    None,
                    true,
                )
                .await;
            StoreOpResult::ok()
        }
        Err(e) => StoreOpResult::err(e.to_string()),
    }
}

// ── Attribute Option ──

pub async fn create_option(
    state: &ServerState,
    attribute_id: i64,
    assigned_id: Option<i64>,
    data: shared::models::attribute::AttributeOptionCreate,
) -> StoreOpResult {
    use crate::db::repository::attribute;

    // Verify attribute exists
    if attribute::find_by_id(&state.pool, attribute_id)
        .await
        .ok()
        .flatten()
        .is_none()
    {
        return StoreOpResult::err(format!("Attribute {attribute_id} not found"));
    }

    let id = assigned_id.unwrap_or_else(shared::util::snowflake_id);
    match sqlx::query(
        "INSERT INTO attribute_option (id, attribute_id, name, price_modifier, display_order, is_active, receipt_name, kitchen_print_name, enable_quantity, max_quantity) VALUES (?1, ?2, ?3, ?4, ?5, 1, ?6, ?7, ?8, ?9)",
    )
    .bind(id)
    .bind(attribute_id)
    .bind(&data.name)
    .bind(data.price_modifier)
    .bind(data.display_order)
    .bind(&data.receipt_name)
    .bind(&data.kitchen_print_name)
    .bind(data.enable_quantity)
    .bind(data.max_quantity)
    .execute(&state.pool)
    .await
    {
        Ok(_) => {}
        Err(e) => return StoreOpResult::err(e.to_string()),
    }

    state
        .broadcast_sync::<()>(
            SyncResource::Attribute,
            SyncChangeType::Updated,
            &attribute_id.to_string(),
            None,
            true,
        )
        .await;
    StoreOpResult::created(id)
}

pub async fn update_option(
    state: &ServerState,
    id: i64,
    data: shared::models::attribute::AttributeOptionUpdate,
) -> StoreOpResult {
    match sqlx::query(
        "UPDATE attribute_option SET name = COALESCE(?1, name), price_modifier = COALESCE(?2, price_modifier), display_order = COALESCE(?3, display_order), is_active = COALESCE(?4, is_active), receipt_name = COALESCE(?5, receipt_name), kitchen_print_name = COALESCE(?6, kitchen_print_name), enable_quantity = COALESCE(?7, enable_quantity), max_quantity = COALESCE(?8, max_quantity) WHERE id = ?9",
    )
    .bind(&data.name)
    .bind(data.price_modifier)
    .bind(data.display_order)
    .bind(data.is_active)
    .bind(&data.receipt_name)
    .bind(&data.kitchen_print_name)
    .bind(data.enable_quantity)
    .bind(data.max_quantity)
    .bind(id)
    .execute(&state.pool)
    .await
    {
        Ok(r) if r.rows_affected() == 0 => {
            return StoreOpResult::err(format!("Attribute option {id} not found"))
        }
        Err(e) => return StoreOpResult::err(e.to_string()),
        _ => {}
    }

    // Get attribute_id for sync broadcast
    if let Ok(Some(attr_id)) =
        sqlx::query_scalar::<_, i64>("SELECT attribute_id FROM attribute_option WHERE id = ?")
            .bind(id)
            .fetch_optional(&state.pool)
            .await
    {
        state
            .broadcast_sync::<()>(
                SyncResource::Attribute,
                SyncChangeType::Updated,
                &attr_id.to_string(),
                None,
                true,
            )
            .await;
    }

    StoreOpResult::ok()
}

pub async fn delete_option(state: &ServerState, id: i64) -> StoreOpResult {
    // Get attribute_id before delete for sync broadcast
    let attr_id =
        sqlx::query_scalar::<_, i64>("SELECT attribute_id FROM attribute_option WHERE id = ?")
            .bind(id)
            .fetch_optional(&state.pool)
            .await
            .ok()
            .flatten();

    match sqlx::query("DELETE FROM attribute_option WHERE id = ?")
        .bind(id)
        .execute(&state.pool)
        .await
    {
        Ok(r) if r.rows_affected() == 0 => {
            return StoreOpResult::err(format!("Attribute option {id} not found"));
        }
        Err(e) => return StoreOpResult::err(e.to_string()),
        _ => {}
    }

    if let Some(aid) = attr_id {
        state
            .broadcast_sync::<()>(
                SyncResource::Attribute,
                SyncChangeType::Updated,
                &aid.to_string(),
                None,
                true,
            )
            .await;
    }

    StoreOpResult::ok()
}

pub async fn batch_update_option_sort_order(
    state: &ServerState,
    attribute_id: i64,
    items: Vec<shared::cloud::store_op::SortOrderItem>,
) -> StoreOpResult {
    for item in &items {
        if let Err(e) = sqlx::query("UPDATE attribute_option SET display_order = ?1 WHERE id = ?2")
            .bind(item.sort_order)
            .bind(item.id)
            .execute(&state.pool)
            .await
        {
            return StoreOpResult::err(e.to_string());
        }
    }

    state
        .broadcast_sync::<()>(
            SyncResource::Attribute,
            SyncChangeType::Updated,
            &attribute_id.to_string(),
            None,
            true,
        )
        .await;
    StoreOpResult::ok()
}

// ── Attribute Binding ──

pub async fn bind(
    state: &ServerState,
    owner: &BindingOwner,
    attribute_id: i64,
    is_required: bool,
    display_order: i32,
    default_option_ids: Option<Vec<i64>>,
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
                    SyncChangeType::Created,
                    &binding.id.to_string(),
                    Some(&binding),
                    true,
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
            SyncChangeType::Deleted,
            &binding_id.to_string(),
            None,
            true,
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
                .broadcast_sync(
                    SyncResource::Tag,
                    SyncChangeType::Created,
                    &t.id.to_string(),
                    Some(&t),
                    true,
                )
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
                .broadcast_sync(
                    SyncResource::Tag,
                    SyncChangeType::Updated,
                    &t.id.to_string(),
                    Some(&t),
                    true,
                )
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
                .broadcast_sync::<()>(
                    SyncResource::Tag,
                    SyncChangeType::Deleted,
                    &id.to_string(),
                    None,
                    true,
                )
                .await;
            StoreOpResult::ok()
        }
        Err(e) => StoreOpResult::err(e.to_string()),
    }
}
