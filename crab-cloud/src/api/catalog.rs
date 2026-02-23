//! Catalog REST API — Console CRUD forwarded to edge via typed RPC
//!
//! Each write operation:
//! 1. Verify store ownership
//! 2. Construct CatalogOp
//! 3. Send CloudMessage::Rpc to edge via connected_edges channel
//! 4. Await RpcResult via pending_rpcs oneshot (10s timeout)
//! 5. Return CatalogOpResult to console
//!
//! Read operations query the normalized PG tables directly (no edge round-trip).

use axum::{
    Extension, Json,
    extract::{Path, State},
};
use shared::cloud::catalog::{CatalogOp, CatalogOpResult};
use shared::cloud::ws::{CloudMessage, CloudRpc, CloudRpcResult};
use shared::error::{AppError, ErrorCode};

use crate::auth::tenant_auth::TenantIdentity;
use crate::db::catalog;
use crate::state::AppState;

type ApiResult<T> = Result<Json<T>, AppError>;

// ── Read endpoints (direct PG query, no edge round-trip) ──

/// GET /api/tenant/stores/:id/catalog/products
pub async fn list_products(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path(store_id): Path<i64>,
) -> ApiResult<Vec<catalog::CatalogProduct>> {
    verify_store(&state, store_id, &identity.tenant_id).await?;
    let products = catalog::list_products(&state.pool, store_id)
        .await
        .map_err(internal)?;
    Ok(Json(products))
}

/// GET /api/tenant/stores/:id/catalog/categories
pub async fn list_categories(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path(store_id): Path<i64>,
) -> ApiResult<Vec<catalog::CatalogCategory>> {
    verify_store(&state, store_id, &identity.tenant_id).await?;
    let categories = catalog::list_categories(&state.pool, store_id)
        .await
        .map_err(internal)?;
    Ok(Json(categories))
}

/// GET /api/tenant/stores/:id/catalog/tags
pub async fn list_tags(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path(store_id): Path<i64>,
) -> ApiResult<Vec<catalog::CatalogTag>> {
    verify_store(&state, store_id, &identity.tenant_id).await?;
    let tags = catalog::list_tags(&state.pool, store_id)
        .await
        .map_err(internal)?;
    Ok(Json(tags))
}

/// GET /api/tenant/stores/:id/catalog/attributes
pub async fn list_attributes(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path(store_id): Path<i64>,
) -> ApiResult<Vec<catalog::CatalogAttribute>> {
    verify_store(&state, store_id, &identity.tenant_id).await?;
    let attributes = catalog::list_attributes(&state.pool, store_id)
        .await
        .map_err(internal)?;
    Ok(Json(attributes))
}

// ── Write endpoints (RPC forwarded to edge) ──

/// POST /api/tenant/stores/:id/catalog/products
pub async fn create_product(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path(store_id): Path<i64>,
    Json(data): Json<shared::models::product::ProductCreate>,
) -> ApiResult<CatalogOpResult> {
    verify_store(&state, store_id, &identity.tenant_id).await?;
    fire_ensure_image(&state, store_id, &identity.tenant_id, data.image.as_deref()).await;
    send_catalog_rpc(&state, store_id, CatalogOp::CreateProduct { data }).await
}

/// PUT /api/tenant/stores/:id/catalog/products/:pid
pub async fn update_product(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path((store_id, product_id)): Path<(i64, i64)>,
    Json(data): Json<shared::models::product::ProductUpdate>,
) -> ApiResult<CatalogOpResult> {
    verify_store(&state, store_id, &identity.tenant_id).await?;
    fire_ensure_image(&state, store_id, &identity.tenant_id, data.image.as_deref()).await;
    send_catalog_rpc(
        &state,
        store_id,
        CatalogOp::UpdateProduct {
            id: product_id,
            data,
        },
    )
    .await
}

/// DELETE /api/tenant/stores/:id/catalog/products/:pid
pub async fn delete_product(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path((store_id, product_id)): Path<(i64, i64)>,
) -> ApiResult<CatalogOpResult> {
    verify_store(&state, store_id, &identity.tenant_id).await?;
    send_catalog_rpc(
        &state,
        store_id,
        CatalogOp::DeleteProduct { id: product_id },
    )
    .await
}

/// POST /api/tenant/stores/:id/catalog/categories
pub async fn create_category(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path(store_id): Path<i64>,
    Json(data): Json<shared::models::category::CategoryCreate>,
) -> ApiResult<CatalogOpResult> {
    verify_store(&state, store_id, &identity.tenant_id).await?;
    send_catalog_rpc(&state, store_id, CatalogOp::CreateCategory { data }).await
}

/// PUT /api/tenant/stores/:id/catalog/categories/:cid
pub async fn update_category(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path((store_id, category_id)): Path<(i64, i64)>,
    Json(data): Json<shared::models::category::CategoryUpdate>,
) -> ApiResult<CatalogOpResult> {
    verify_store(&state, store_id, &identity.tenant_id).await?;
    send_catalog_rpc(
        &state,
        store_id,
        CatalogOp::UpdateCategory {
            id: category_id,
            data,
        },
    )
    .await
}

/// DELETE /api/tenant/stores/:id/catalog/categories/:cid
pub async fn delete_category(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path((store_id, category_id)): Path<(i64, i64)>,
) -> ApiResult<CatalogOpResult> {
    verify_store(&state, store_id, &identity.tenant_id).await?;
    send_catalog_rpc(
        &state,
        store_id,
        CatalogOp::DeleteCategory { id: category_id },
    )
    .await
}

/// POST /api/tenant/stores/:id/catalog/tags
pub async fn create_tag(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path(store_id): Path<i64>,
    Json(data): Json<shared::models::tag::TagCreate>,
) -> ApiResult<CatalogOpResult> {
    verify_store(&state, store_id, &identity.tenant_id).await?;
    send_catalog_rpc(&state, store_id, CatalogOp::CreateTag { data }).await
}

/// PUT /api/tenant/stores/:id/catalog/tags/:tid
pub async fn update_tag(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path((store_id, tag_id)): Path<(i64, i64)>,
    Json(data): Json<shared::models::tag::TagUpdate>,
) -> ApiResult<CatalogOpResult> {
    verify_store(&state, store_id, &identity.tenant_id).await?;
    send_catalog_rpc(&state, store_id, CatalogOp::UpdateTag { id: tag_id, data }).await
}

/// DELETE /api/tenant/stores/:id/catalog/tags/:tid
pub async fn delete_tag(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path((store_id, tag_id)): Path<(i64, i64)>,
) -> ApiResult<CatalogOpResult> {
    verify_store(&state, store_id, &identity.tenant_id).await?;
    send_catalog_rpc(&state, store_id, CatalogOp::DeleteTag { id: tag_id }).await
}

/// POST /api/tenant/stores/:id/catalog/attributes
pub async fn create_attribute(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path(store_id): Path<i64>,
    Json(data): Json<shared::models::attribute::AttributeCreate>,
) -> ApiResult<CatalogOpResult> {
    verify_store(&state, store_id, &identity.tenant_id).await?;
    send_catalog_rpc(&state, store_id, CatalogOp::CreateAttribute { data }).await
}

/// PUT /api/tenant/stores/:id/catalog/attributes/:aid
pub async fn update_attribute(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path((store_id, attr_id)): Path<(i64, i64)>,
    Json(data): Json<shared::models::attribute::AttributeUpdate>,
) -> ApiResult<CatalogOpResult> {
    verify_store(&state, store_id, &identity.tenant_id).await?;
    send_catalog_rpc(
        &state,
        store_id,
        CatalogOp::UpdateAttribute { id: attr_id, data },
    )
    .await
}

/// DELETE /api/tenant/stores/:id/catalog/attributes/:aid
pub async fn delete_attribute(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path((store_id, attr_id)): Path<(i64, i64)>,
) -> ApiResult<CatalogOpResult> {
    verify_store(&state, store_id, &identity.tenant_id).await?;
    send_catalog_rpc(&state, store_id, CatalogOp::DeleteAttribute { id: attr_id }).await
}

/// POST /api/tenant/stores/:id/catalog/attributes/bind
pub async fn bind_attribute(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path(store_id): Path<i64>,
    Json(req): Json<BindAttributeRequest>,
) -> ApiResult<CatalogOpResult> {
    verify_store(&state, store_id, &identity.tenant_id).await?;
    send_catalog_rpc(
        &state,
        store_id,
        CatalogOp::BindAttribute {
            owner: req.owner,
            attribute_id: req.attribute_id,
            is_required: req.is_required,
            display_order: req.display_order,
            default_option_ids: req.default_option_ids,
        },
    )
    .await
}

/// POST /api/tenant/stores/:id/catalog/attributes/unbind
pub async fn unbind_attribute(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path(store_id): Path<i64>,
    Json(req): Json<UnbindAttributeRequest>,
) -> ApiResult<CatalogOpResult> {
    verify_store(&state, store_id, &identity.tenant_id).await?;
    send_catalog_rpc(
        &state,
        store_id,
        CatalogOp::UnbindAttribute {
            binding_id: req.binding_id,
        },
    )
    .await
}

// ── Price Rule endpoints ──

/// GET /api/tenant/stores/:id/catalog/price-rules
pub async fn list_price_rules(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path(store_id): Path<i64>,
) -> ApiResult<Vec<shared::models::PriceRule>> {
    verify_store(&state, store_id, &identity.tenant_id).await?;
    let rules = catalog::list_price_rules(&state.pool, store_id)
        .await
        .map_err(internal)?;
    Ok(Json(rules))
}

/// POST /api/tenant/stores/:id/catalog/price-rules
pub async fn create_price_rule(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path(store_id): Path<i64>,
    Json(data): Json<shared::models::price_rule::PriceRuleCreate>,
) -> ApiResult<CatalogOpResult> {
    verify_store(&state, store_id, &identity.tenant_id).await?;
    send_catalog_rpc(&state, store_id, CatalogOp::CreatePriceRule { data }).await
}

/// PUT /api/tenant/stores/:id/catalog/price-rules/:rid
pub async fn update_price_rule(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path((store_id, rule_id)): Path<(i64, i64)>,
    Json(data): Json<shared::models::price_rule::PriceRuleUpdate>,
) -> ApiResult<CatalogOpResult> {
    verify_store(&state, store_id, &identity.tenant_id).await?;
    send_catalog_rpc(
        &state,
        store_id,
        CatalogOp::UpdatePriceRule { id: rule_id, data },
    )
    .await
}

/// DELETE /api/tenant/stores/:id/catalog/price-rules/:rid
pub async fn delete_price_rule(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path((store_id, rule_id)): Path<(i64, i64)>,
) -> ApiResult<CatalogOpResult> {
    verify_store(&state, store_id, &identity.tenant_id).await?;
    send_catalog_rpc(&state, store_id, CatalogOp::DeletePriceRule { id: rule_id }).await
}

// ── Employee endpoints ──

/// GET /api/tenant/stores/:id/catalog/employees
pub async fn list_employees(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path(store_id): Path<i64>,
) -> ApiResult<Vec<shared::models::employee::Employee>> {
    verify_store(&state, store_id, &identity.tenant_id).await?;
    let employees = catalog::list_employees(&state.pool, store_id)
        .await
        .map_err(internal)?;
    Ok(Json(employees))
}

/// POST /api/tenant/stores/:id/catalog/employees
pub async fn create_employee(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path(store_id): Path<i64>,
    Json(data): Json<shared::models::employee::EmployeeCreate>,
) -> ApiResult<CatalogOpResult> {
    verify_store(&state, store_id, &identity.tenant_id).await?;
    send_catalog_rpc(&state, store_id, CatalogOp::CreateEmployee { data }).await
}

/// PUT /api/tenant/stores/:id/catalog/employees/:eid
pub async fn update_employee(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path((store_id, employee_id)): Path<(i64, i64)>,
    Json(data): Json<shared::models::employee::EmployeeUpdate>,
) -> ApiResult<CatalogOpResult> {
    verify_store(&state, store_id, &identity.tenant_id).await?;
    send_catalog_rpc(
        &state,
        store_id,
        CatalogOp::UpdateEmployee {
            id: employee_id,
            data,
        },
    )
    .await
}

/// DELETE /api/tenant/stores/:id/catalog/employees/:eid
pub async fn delete_employee(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path((store_id, employee_id)): Path<(i64, i64)>,
) -> ApiResult<CatalogOpResult> {
    verify_store(&state, store_id, &identity.tenant_id).await?;
    send_catalog_rpc(
        &state,
        store_id,
        CatalogOp::DeleteEmployee { id: employee_id },
    )
    .await
}

// ── Zone endpoints ──

/// GET /api/tenant/stores/:id/catalog/zones
pub async fn list_zones(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path(store_id): Path<i64>,
) -> ApiResult<Vec<shared::models::zone::Zone>> {
    verify_store(&state, store_id, &identity.tenant_id).await?;
    let zones = catalog::list_zones(&state.pool, store_id)
        .await
        .map_err(internal)?;
    Ok(Json(zones))
}

/// POST /api/tenant/stores/:id/catalog/zones
pub async fn create_zone(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path(store_id): Path<i64>,
    Json(data): Json<shared::models::zone::ZoneCreate>,
) -> ApiResult<CatalogOpResult> {
    verify_store(&state, store_id, &identity.tenant_id).await?;
    send_catalog_rpc(&state, store_id, CatalogOp::CreateZone { data }).await
}

/// PUT /api/tenant/stores/:id/catalog/zones/:zid
pub async fn update_zone(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path((store_id, zone_id)): Path<(i64, i64)>,
    Json(data): Json<shared::models::zone::ZoneUpdate>,
) -> ApiResult<CatalogOpResult> {
    verify_store(&state, store_id, &identity.tenant_id).await?;
    send_catalog_rpc(
        &state,
        store_id,
        CatalogOp::UpdateZone { id: zone_id, data },
    )
    .await
}

/// DELETE /api/tenant/stores/:id/catalog/zones/:zid
pub async fn delete_zone(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path((store_id, zone_id)): Path<(i64, i64)>,
) -> ApiResult<CatalogOpResult> {
    verify_store(&state, store_id, &identity.tenant_id).await?;
    send_catalog_rpc(&state, store_id, CatalogOp::DeleteZone { id: zone_id }).await
}

// ── DiningTable endpoints ──

/// GET /api/tenant/stores/:id/catalog/tables
pub async fn list_tables(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path(store_id): Path<i64>,
) -> ApiResult<Vec<shared::models::dining_table::DiningTable>> {
    verify_store(&state, store_id, &identity.tenant_id).await?;
    let tables = catalog::list_tables(&state.pool, store_id)
        .await
        .map_err(internal)?;
    Ok(Json(tables))
}

/// POST /api/tenant/stores/:id/catalog/tables
pub async fn create_table(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path(store_id): Path<i64>,
    Json(data): Json<shared::models::dining_table::DiningTableCreate>,
) -> ApiResult<CatalogOpResult> {
    verify_store(&state, store_id, &identity.tenant_id).await?;
    send_catalog_rpc(&state, store_id, CatalogOp::CreateTable { data }).await
}

/// PUT /api/tenant/stores/:id/catalog/tables/:tid
pub async fn update_table(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path((store_id, table_id)): Path<(i64, i64)>,
    Json(data): Json<shared::models::dining_table::DiningTableUpdate>,
) -> ApiResult<CatalogOpResult> {
    verify_store(&state, store_id, &identity.tenant_id).await?;
    send_catalog_rpc(
        &state,
        store_id,
        CatalogOp::UpdateTable { id: table_id, data },
    )
    .await
}

/// DELETE /api/tenant/stores/:id/catalog/tables/:tid
pub async fn delete_table(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path((store_id, table_id)): Path<(i64, i64)>,
) -> ApiResult<CatalogOpResult> {
    verify_store(&state, store_id, &identity.tenant_id).await?;
    send_catalog_rpc(&state, store_id, CatalogOp::DeleteTable { id: table_id }).await
}

// ── Request types ──

#[derive(serde::Deserialize)]
pub struct BindAttributeRequest {
    pub owner: shared::cloud::catalog::BindingOwner,
    pub attribute_id: i64,
    #[serde(default)]
    pub is_required: bool,
    #[serde(default)]
    pub display_order: i32,
    pub default_option_ids: Option<Vec<i32>>,
}

#[derive(serde::Deserialize)]
pub struct UnbindAttributeRequest {
    pub binding_id: i64,
}

// ── RPC helper ──

/// Send a CatalogOp to edge via typed RPC and await the result.
///
/// Returns error if edge is offline or doesn't respond within 10 seconds.
async fn send_catalog_rpc(
    state: &AppState,
    store_id: i64,
    op: CatalogOp,
) -> ApiResult<CatalogOpResult> {
    let rpc = CloudRpc::CatalogOp(Box::new(op));
    let result = crate::services::rpc::call_edge_rpc(&state.edges, store_id, rpc).await?;

    match result {
        CloudRpcResult::CatalogOp(r) => Ok(Json(*r)),
        CloudRpcResult::Json { error, .. } => Err(AppError::with_message(
            ErrorCode::InternalError,
            error.unwrap_or_else(|| "Unexpected RPC result type".to_string()),
        )),
    }
}

// ── Helpers ──

use super::tenant::verify_store;

fn internal(e: impl std::fmt::Display) -> AppError {
    tracing::error!("Catalog query error: {e}");
    AppError::new(ErrorCode::InternalError)
}

/// Fire-and-forget: send EnsureImage RPC to edge so it downloads the image from S3.
///
/// Does nothing if image hash is empty or edge is offline.
async fn fire_ensure_image(
    state: &AppState,
    store_id: i64,
    tenant_id: &str,
    image_hash: Option<&str>,
) {
    let hash = match image_hash {
        Some(h) if !h.is_empty() => h,
        _ => return,
    };

    let presigned_url = match super::image::presigned_get_url(state, tenant_id, hash).await {
        Ok(url) => url,
        Err(e) => {
            tracing::warn!(hash = %hash, error = %e, "Failed to generate presigned URL for image");
            return;
        }
    };

    let sender = match state.edges.connected.get(&store_id) {
        Some(s) => s.clone(),
        None => return, // edge offline, image will be downloaded on next connect
    };

    let msg = CloudMessage::Rpc {
        id: format!("img-{hash}"),
        payload: Box::new(CloudRpc::CatalogOp(Box::new(CatalogOp::EnsureImage {
            presigned_url,
            hash: hash.to_string(),
        }))),
    };

    let _ = sender.try_send(msg);
}
