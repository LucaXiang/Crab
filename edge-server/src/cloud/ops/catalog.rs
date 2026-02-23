//! Product + Category operations (via CatalogService)

use shared::cloud::catalog::{CatalogOpData, CatalogOpResult};
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
) -> CatalogOpResult {
    match state
        .catalog_service
        .create_product(assigned_id, data)
        .await
    {
        Ok(p) => {
            state
                .broadcast_sync("product", "created", &p.id.to_string(), Some(&p))
                .await;
            CatalogOpResult::created(p.id).with_data(CatalogOpData::Product(p))
        }
        Err(e) => CatalogOpResult::err(e.to_string()),
    }
}

pub async fn update_product(state: &ServerState, id: i64, data: ProductUpdate) -> CatalogOpResult {
    match state.catalog_service.update_product(id, data).await {
        Ok(p) => {
            state
                .broadcast_sync("product", "updated", &p.id.to_string(), Some(&p))
                .await;
            CatalogOpResult::ok().with_data(CatalogOpData::Product(p))
        }
        Err(e) => CatalogOpResult::err(e.to_string()),
    }
}

pub async fn delete_product(state: &ServerState, id: i64) -> CatalogOpResult {
    match state.catalog_service.delete_product(id).await {
        Ok(()) => {
            state
                .broadcast_sync::<()>("product", "deleted", &id.to_string(), None)
                .await;
            CatalogOpResult::ok()
        }
        Err(e) => CatalogOpResult::err(e.to_string()),
    }
}

// ── Category ──

pub async fn create_category(
    state: &ServerState,
    assigned_id: Option<i64>,
    data: CategoryCreate,
) -> CatalogOpResult {
    match state
        .catalog_service
        .create_category(assigned_id, data)
        .await
    {
        Ok(c) => {
            state
                .broadcast_sync("category", "created", &c.id.to_string(), Some(&c))
                .await;
            CatalogOpResult::created(c.id).with_data(CatalogOpData::Category(c))
        }
        Err(e) => CatalogOpResult::err(e.to_string()),
    }
}

pub async fn update_category(
    state: &ServerState,
    id: i64,
    data: CategoryUpdate,
) -> CatalogOpResult {
    match state.catalog_service.update_category(id, data).await {
        Ok(c) => {
            state
                .broadcast_sync("category", "updated", &c.id.to_string(), Some(&c))
                .await;
            CatalogOpResult::ok().with_data(CatalogOpData::Category(c))
        }
        Err(e) => CatalogOpResult::err(e.to_string()),
    }
}

pub async fn delete_category(state: &ServerState, id: i64) -> CatalogOpResult {
    match state.catalog_service.delete_category(id).await {
        Ok(()) => {
            state
                .broadcast_sync::<()>("category", "deleted", &id.to_string(), None)
                .await;
            CatalogOpResult::ok()
        }
        Err(e) => CatalogOpResult::err(e.to_string()),
    }
}
