//! Product + Category operations (via CatalogService for store ops)

use shared::cloud::SyncResource;
use shared::cloud::store_op::{StoreOpData, StoreOpResult};
use shared::models::{
    category::{CategoryCreate, CategoryUpdate},
    product::{ProductCreate, ProductUpdate},
};

use crate::core::state::ServerState;

// ── Product ──

pub async fn create_product(
    state: &ServerState,
    assigned_id: Option<i64>,
    data: ProductCreate,
) -> StoreOpResult {
    match state
        .catalog_service
        .create_product(assigned_id, data)
        .await
    {
        Ok(p) => {
            state
                .broadcast_sync(
                    SyncResource::Product,
                    "created",
                    &p.id.to_string(),
                    Some(&p),
                )
                .await;
            StoreOpResult::created(p.id).with_data(StoreOpData::Product(p))
        }
        Err(e) => StoreOpResult::err(e.to_string()),
    }
}

pub async fn update_product(state: &ServerState, id: i64, data: ProductUpdate) -> StoreOpResult {
    match state.catalog_service.update_product(id, data).await {
        Ok(p) => {
            state
                .broadcast_sync(
                    SyncResource::Product,
                    "updated",
                    &p.id.to_string(),
                    Some(&p),
                )
                .await;
            StoreOpResult::ok().with_data(StoreOpData::Product(p))
        }
        Err(e) => StoreOpResult::err(e.to_string()),
    }
}

pub async fn delete_product(state: &ServerState, id: i64) -> StoreOpResult {
    match state.catalog_service.delete_product(id).await {
        Ok(()) => {
            state
                .broadcast_sync::<()>(SyncResource::Product, "deleted", &id.to_string(), None)
                .await;
            StoreOpResult::ok()
        }
        Err(e) => StoreOpResult::err(e.to_string()),
    }
}

// ── Category ──

pub async fn create_category(
    state: &ServerState,
    assigned_id: Option<i64>,
    data: CategoryCreate,
) -> StoreOpResult {
    match state
        .catalog_service
        .create_category(assigned_id, data)
        .await
    {
        Ok(c) => {
            state
                .broadcast_sync(
                    SyncResource::Category,
                    "created",
                    &c.id.to_string(),
                    Some(&c),
                )
                .await;
            StoreOpResult::created(c.id).with_data(StoreOpData::Category(c))
        }
        Err(e) => StoreOpResult::err(e.to_string()),
    }
}

pub async fn update_category(state: &ServerState, id: i64, data: CategoryUpdate) -> StoreOpResult {
    match state.catalog_service.update_category(id, data).await {
        Ok(c) => {
            state
                .broadcast_sync(
                    SyncResource::Category,
                    "updated",
                    &c.id.to_string(),
                    Some(&c),
                )
                .await;
            StoreOpResult::ok().with_data(StoreOpData::Category(c))
        }
        Err(e) => StoreOpResult::err(e.to_string()),
    }
}

pub async fn delete_category(state: &ServerState, id: i64) -> StoreOpResult {
    match state.catalog_service.delete_category(id).await {
        Ok(()) => {
            state
                .broadcast_sync::<()>(SyncResource::Category, "deleted", &id.to_string(), None)
                .await;
            StoreOpResult::ok()
        }
        Err(e) => StoreOpResult::err(e.to_string()),
    }
}
