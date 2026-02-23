//! Catalog REST API — Cloud PG authoritative CRUD
//!
//! Each write operation:
//! 1. Verify store ownership
//! 2. Write directly to PG (authoritative source)
//! 3. Increment catalog version
//! 4. Fire-and-forget push CatalogOp to edge (if online)
//! 5. Return CatalogOpResult to console
//!
//! Read operations query the normalized PG tables directly (no edge round-trip).

use axum::{
    Extension, Json,
    extract::{Path, State},
};
use shared::cloud::catalog::{CatalogOp, CatalogOpResult};
use shared::cloud::ws::{CloudMessage, CloudRpc};
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

// ── Product write endpoints ──

/// POST /api/tenant/stores/:id/catalog/products
pub async fn create_product(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path(store_id): Path<i64>,
    Json(data): Json<shared::models::product::ProductCreate>,
) -> ApiResult<CatalogOpResult> {
    verify_store(&state, store_id, &identity.tenant_id).await?;
    fire_ensure_image(&state, store_id, &identity.tenant_id, data.image.as_deref()).await;

    let (source_id, op_data) = catalog::create_product_direct(&state.pool, store_id, &data)
        .await
        .map_err(internal)?;
    catalog::increment_catalog_version(&state.pool, store_id)
        .await
        .map_err(internal)?;

    push_to_edge_if_online(
        &state,
        store_id,
        CatalogOp::CreateProduct {
            id: Some(source_id),
            data,
        },
    );

    Ok(Json(CatalogOpResult::created(source_id).with_data(op_data)))
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

    catalog::update_product_direct(&state.pool, store_id, product_id, &data)
        .await
        .map_err(internal)?;
    catalog::increment_catalog_version(&state.pool, store_id)
        .await
        .map_err(internal)?;

    push_to_edge_if_online(
        &state,
        store_id,
        CatalogOp::UpdateProduct {
            id: product_id,
            data,
        },
    );

    Ok(Json(CatalogOpResult::ok()))
}

/// DELETE /api/tenant/stores/:id/catalog/products/:pid
pub async fn delete_product(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path((store_id, product_id)): Path<(i64, i64)>,
) -> ApiResult<CatalogOpResult> {
    verify_store(&state, store_id, &identity.tenant_id).await?;

    catalog::delete_product(&state.pool, store_id, product_id)
        .await
        .map_err(internal)?;
    catalog::increment_catalog_version(&state.pool, store_id)
        .await
        .map_err(internal)?;

    push_to_edge_if_online(
        &state,
        store_id,
        CatalogOp::DeleteProduct { id: product_id },
    );

    Ok(Json(CatalogOpResult::ok()))
}

// ── Category write endpoints ──

/// POST /api/tenant/stores/:id/catalog/categories
pub async fn create_category(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path(store_id): Path<i64>,
    Json(data): Json<shared::models::category::CategoryCreate>,
) -> ApiResult<CatalogOpResult> {
    verify_store(&state, store_id, &identity.tenant_id).await?;

    let (source_id, op_data) = catalog::create_category_direct(&state.pool, store_id, &data)
        .await
        .map_err(internal)?;
    catalog::increment_catalog_version(&state.pool, store_id)
        .await
        .map_err(internal)?;

    push_to_edge_if_online(
        &state,
        store_id,
        CatalogOp::CreateCategory {
            id: Some(source_id),
            data,
        },
    );

    Ok(Json(CatalogOpResult::created(source_id).with_data(op_data)))
}

/// PUT /api/tenant/stores/:id/catalog/categories/:cid
pub async fn update_category(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path((store_id, category_id)): Path<(i64, i64)>,
    Json(data): Json<shared::models::category::CategoryUpdate>,
) -> ApiResult<CatalogOpResult> {
    verify_store(&state, store_id, &identity.tenant_id).await?;

    catalog::update_category_direct(&state.pool, store_id, category_id, &data)
        .await
        .map_err(internal)?;
    catalog::increment_catalog_version(&state.pool, store_id)
        .await
        .map_err(internal)?;

    push_to_edge_if_online(
        &state,
        store_id,
        CatalogOp::UpdateCategory {
            id: category_id,
            data,
        },
    );

    Ok(Json(CatalogOpResult::ok()))
}

/// DELETE /api/tenant/stores/:id/catalog/categories/:cid
pub async fn delete_category(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path((store_id, category_id)): Path<(i64, i64)>,
) -> ApiResult<CatalogOpResult> {
    verify_store(&state, store_id, &identity.tenant_id).await?;

    catalog::delete_category(&state.pool, store_id, category_id)
        .await
        .map_err(internal)?;
    catalog::increment_catalog_version(&state.pool, store_id)
        .await
        .map_err(internal)?;

    push_to_edge_if_online(
        &state,
        store_id,
        CatalogOp::DeleteCategory { id: category_id },
    );

    Ok(Json(CatalogOpResult::ok()))
}

// ── Tag write endpoints ──

/// POST /api/tenant/stores/:id/catalog/tags
pub async fn create_tag(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path(store_id): Path<i64>,
    Json(data): Json<shared::models::tag::TagCreate>,
) -> ApiResult<CatalogOpResult> {
    verify_store(&state, store_id, &identity.tenant_id).await?;

    let (source_id, op_data) = catalog::create_tag_direct(&state.pool, store_id, &data)
        .await
        .map_err(internal)?;
    catalog::increment_catalog_version(&state.pool, store_id)
        .await
        .map_err(internal)?;

    push_to_edge_if_online(
        &state,
        store_id,
        CatalogOp::CreateTag {
            id: Some(source_id),
            data,
        },
    );

    Ok(Json(CatalogOpResult::created(source_id).with_data(op_data)))
}

/// PUT /api/tenant/stores/:id/catalog/tags/:tid
pub async fn update_tag(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path((store_id, tag_id)): Path<(i64, i64)>,
    Json(data): Json<shared::models::tag::TagUpdate>,
) -> ApiResult<CatalogOpResult> {
    verify_store(&state, store_id, &identity.tenant_id).await?;

    catalog::update_tag_direct(&state.pool, store_id, tag_id, &data)
        .await
        .map_err(internal)?;
    catalog::increment_catalog_version(&state.pool, store_id)
        .await
        .map_err(internal)?;

    push_to_edge_if_online(&state, store_id, CatalogOp::UpdateTag { id: tag_id, data });

    Ok(Json(CatalogOpResult::ok()))
}

/// DELETE /api/tenant/stores/:id/catalog/tags/:tid
pub async fn delete_tag(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path((store_id, tag_id)): Path<(i64, i64)>,
) -> ApiResult<CatalogOpResult> {
    verify_store(&state, store_id, &identity.tenant_id).await?;

    catalog::delete_tag_direct(&state.pool, store_id, tag_id)
        .await
        .map_err(internal)?;
    catalog::increment_catalog_version(&state.pool, store_id)
        .await
        .map_err(internal)?;

    push_to_edge_if_online(&state, store_id, CatalogOp::DeleteTag { id: tag_id });

    Ok(Json(CatalogOpResult::ok()))
}

// ── Attribute write endpoints ──

/// POST /api/tenant/stores/:id/catalog/attributes
pub async fn create_attribute(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path(store_id): Path<i64>,
    Json(data): Json<shared::models::attribute::AttributeCreate>,
) -> ApiResult<CatalogOpResult> {
    verify_store(&state, store_id, &identity.tenant_id).await?;

    let (source_id,) = catalog::create_attribute_direct(&state.pool, store_id, &data)
        .await
        .map_err(internal)?;
    catalog::increment_catalog_version(&state.pool, store_id)
        .await
        .map_err(internal)?;

    push_to_edge_if_online(
        &state,
        store_id,
        CatalogOp::CreateAttribute {
            id: Some(source_id),
            data,
        },
    );

    Ok(Json(CatalogOpResult::created(source_id)))
}

/// PUT /api/tenant/stores/:id/catalog/attributes/:aid
pub async fn update_attribute(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path((store_id, attr_id)): Path<(i64, i64)>,
    Json(data): Json<shared::models::attribute::AttributeUpdate>,
) -> ApiResult<CatalogOpResult> {
    verify_store(&state, store_id, &identity.tenant_id).await?;

    catalog::update_attribute_direct(&state.pool, store_id, attr_id, &data)
        .await
        .map_err(internal)?;
    catalog::increment_catalog_version(&state.pool, store_id)
        .await
        .map_err(internal)?;

    push_to_edge_if_online(
        &state,
        store_id,
        CatalogOp::UpdateAttribute { id: attr_id, data },
    );

    Ok(Json(CatalogOpResult::ok()))
}

/// DELETE /api/tenant/stores/:id/catalog/attributes/:aid
pub async fn delete_attribute(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path((store_id, attr_id)): Path<(i64, i64)>,
) -> ApiResult<CatalogOpResult> {
    verify_store(&state, store_id, &identity.tenant_id).await?;

    catalog::delete_attribute_direct(&state.pool, store_id, attr_id)
        .await
        .map_err(internal)?;
    catalog::increment_catalog_version(&state.pool, store_id)
        .await
        .map_err(internal)?;

    push_to_edge_if_online(&state, store_id, CatalogOp::DeleteAttribute { id: attr_id });

    Ok(Json(CatalogOpResult::ok()))
}

/// POST /api/tenant/stores/:id/catalog/attributes/bind
pub async fn bind_attribute(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path(store_id): Path<i64>,
    Json(req): Json<BindAttributeRequest>,
) -> ApiResult<CatalogOpResult> {
    verify_store(&state, store_id, &identity.tenant_id).await?;

    let binding_id = catalog::bind_attribute_direct(
        &state.pool,
        store_id,
        catalog::BindAttributeParams {
            owner_type: req.owner.owner_type(),
            owner_id: req.owner.owner_id(),
            attribute_id: req.attribute_id,
            is_required: req.is_required,
            display_order: req.display_order,
            default_option_ids: req.default_option_ids.clone(),
        },
    )
    .await
    .map_err(internal)?;
    catalog::increment_catalog_version(&state.pool, store_id)
        .await
        .map_err(internal)?;

    push_to_edge_if_online(
        &state,
        store_id,
        CatalogOp::BindAttribute {
            owner: req.owner,
            attribute_id: req.attribute_id,
            is_required: req.is_required,
            display_order: req.display_order,
            default_option_ids: req.default_option_ids,
        },
    );

    Ok(Json(CatalogOpResult::created(binding_id)))
}

/// POST /api/tenant/stores/:id/catalog/attributes/unbind
pub async fn unbind_attribute(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path(store_id): Path<i64>,
    Json(req): Json<UnbindAttributeRequest>,
) -> ApiResult<CatalogOpResult> {
    verify_store(&state, store_id, &identity.tenant_id).await?;

    catalog::unbind_attribute_direct(&state.pool, store_id, req.binding_id)
        .await
        .map_err(internal)?;
    catalog::increment_catalog_version(&state.pool, store_id)
        .await
        .map_err(internal)?;

    push_to_edge_if_online(
        &state,
        store_id,
        CatalogOp::UnbindAttribute {
            binding_id: req.binding_id,
        },
    );

    Ok(Json(CatalogOpResult::ok()))
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

    let (source_id, op_data) = catalog::create_price_rule_direct(&state.pool, store_id, &data)
        .await
        .map_err(internal)?;
    catalog::increment_catalog_version(&state.pool, store_id)
        .await
        .map_err(internal)?;

    push_to_edge_if_online(
        &state,
        store_id,
        CatalogOp::CreatePriceRule {
            id: Some(source_id),
            data,
        },
    );

    Ok(Json(CatalogOpResult::created(source_id).with_data(op_data)))
}

/// PUT /api/tenant/stores/:id/catalog/price-rules/:rid
pub async fn update_price_rule(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path((store_id, rule_id)): Path<(i64, i64)>,
    Json(data): Json<shared::models::price_rule::PriceRuleUpdate>,
) -> ApiResult<CatalogOpResult> {
    verify_store(&state, store_id, &identity.tenant_id).await?;

    catalog::update_price_rule_direct(&state.pool, store_id, rule_id, &data)
        .await
        .map_err(internal)?;
    catalog::increment_catalog_version(&state.pool, store_id)
        .await
        .map_err(internal)?;

    push_to_edge_if_online(
        &state,
        store_id,
        CatalogOp::UpdatePriceRule { id: rule_id, data },
    );

    Ok(Json(CatalogOpResult::ok()))
}

/// DELETE /api/tenant/stores/:id/catalog/price-rules/:rid
pub async fn delete_price_rule(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path((store_id, rule_id)): Path<(i64, i64)>,
) -> ApiResult<CatalogOpResult> {
    verify_store(&state, store_id, &identity.tenant_id).await?;

    catalog::delete_price_rule_direct(&state.pool, store_id, rule_id)
        .await
        .map_err(internal)?;
    catalog::increment_catalog_version(&state.pool, store_id)
        .await
        .map_err(internal)?;

    push_to_edge_if_online(&state, store_id, CatalogOp::DeletePriceRule { id: rule_id });

    Ok(Json(CatalogOpResult::ok()))
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

    let (source_id, op_data) =
        catalog::create_employee_direct(&state.pool, store_id, &identity.tenant_id, &data)
            .await
            .map_err(internal)?;
    catalog::increment_catalog_version(&state.pool, store_id)
        .await
        .map_err(internal)?;

    push_to_edge_if_online(
        &state,
        store_id,
        CatalogOp::CreateEmployee {
            id: Some(source_id),
            data,
        },
    );

    Ok(Json(CatalogOpResult::created(source_id).with_data(op_data)))
}

/// PUT /api/tenant/stores/:id/catalog/employees/:eid
pub async fn update_employee(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path((store_id, employee_id)): Path<(i64, i64)>,
    Json(data): Json<shared::models::employee::EmployeeUpdate>,
) -> ApiResult<CatalogOpResult> {
    verify_store(&state, store_id, &identity.tenant_id).await?;

    let op_data = catalog::update_employee_direct(&state.pool, store_id, employee_id, &data)
        .await
        .map_err(internal)?;
    catalog::increment_catalog_version(&state.pool, store_id)
        .await
        .map_err(internal)?;

    push_to_edge_if_online(
        &state,
        store_id,
        CatalogOp::UpdateEmployee {
            id: employee_id,
            data,
        },
    );

    Ok(Json(CatalogOpResult::ok().with_data(op_data)))
}

/// DELETE /api/tenant/stores/:id/catalog/employees/:eid
pub async fn delete_employee(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path((store_id, employee_id)): Path<(i64, i64)>,
) -> ApiResult<CatalogOpResult> {
    verify_store(&state, store_id, &identity.tenant_id).await?;

    catalog::delete_employee_direct(&state.pool, store_id, employee_id)
        .await
        .map_err(internal)?;
    catalog::increment_catalog_version(&state.pool, store_id)
        .await
        .map_err(internal)?;

    push_to_edge_if_online(
        &state,
        store_id,
        CatalogOp::DeleteEmployee { id: employee_id },
    );

    Ok(Json(CatalogOpResult::ok()))
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

    let (source_id, op_data) =
        catalog::create_zone_direct(&state.pool, store_id, &identity.tenant_id, &data)
            .await
            .map_err(internal)?;
    catalog::increment_catalog_version(&state.pool, store_id)
        .await
        .map_err(internal)?;

    push_to_edge_if_online(
        &state,
        store_id,
        CatalogOp::CreateZone {
            id: Some(source_id),
            data,
        },
    );

    Ok(Json(CatalogOpResult::created(source_id).with_data(op_data)))
}

/// PUT /api/tenant/stores/:id/catalog/zones/:zid
pub async fn update_zone(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path((store_id, zone_id)): Path<(i64, i64)>,
    Json(data): Json<shared::models::zone::ZoneUpdate>,
) -> ApiResult<CatalogOpResult> {
    verify_store(&state, store_id, &identity.tenant_id).await?;

    let op_data = catalog::update_zone_direct(&state.pool, store_id, zone_id, &data)
        .await
        .map_err(internal)?;
    catalog::increment_catalog_version(&state.pool, store_id)
        .await
        .map_err(internal)?;

    push_to_edge_if_online(
        &state,
        store_id,
        CatalogOp::UpdateZone { id: zone_id, data },
    );

    Ok(Json(CatalogOpResult::ok().with_data(op_data)))
}

/// DELETE /api/tenant/stores/:id/catalog/zones/:zid
pub async fn delete_zone(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path((store_id, zone_id)): Path<(i64, i64)>,
) -> ApiResult<CatalogOpResult> {
    verify_store(&state, store_id, &identity.tenant_id).await?;

    catalog::delete_zone_direct(&state.pool, store_id, zone_id)
        .await
        .map_err(internal)?;
    catalog::increment_catalog_version(&state.pool, store_id)
        .await
        .map_err(internal)?;

    push_to_edge_if_online(&state, store_id, CatalogOp::DeleteZone { id: zone_id });

    Ok(Json(CatalogOpResult::ok()))
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

    let (source_id, op_data) =
        catalog::create_table_direct(&state.pool, store_id, &identity.tenant_id, &data)
            .await
            .map_err(internal)?;
    catalog::increment_catalog_version(&state.pool, store_id)
        .await
        .map_err(internal)?;

    push_to_edge_if_online(
        &state,
        store_id,
        CatalogOp::CreateTable {
            id: Some(source_id),
            data,
        },
    );

    Ok(Json(CatalogOpResult::created(source_id).with_data(op_data)))
}

/// PUT /api/tenant/stores/:id/catalog/tables/:tid
pub async fn update_table(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path((store_id, table_id)): Path<(i64, i64)>,
    Json(data): Json<shared::models::dining_table::DiningTableUpdate>,
) -> ApiResult<CatalogOpResult> {
    verify_store(&state, store_id, &identity.tenant_id).await?;

    let op_data = catalog::update_table_direct(&state.pool, store_id, table_id, &data)
        .await
        .map_err(internal)?;
    catalog::increment_catalog_version(&state.pool, store_id)
        .await
        .map_err(internal)?;

    push_to_edge_if_online(
        &state,
        store_id,
        CatalogOp::UpdateTable { id: table_id, data },
    );

    Ok(Json(CatalogOpResult::ok().with_data(op_data)))
}

/// DELETE /api/tenant/stores/:id/catalog/tables/:tid
pub async fn delete_table(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    Path((store_id, table_id)): Path<(i64, i64)>,
) -> ApiResult<CatalogOpResult> {
    verify_store(&state, store_id, &identity.tenant_id).await?;

    catalog::delete_table_direct(&state.pool, store_id, table_id)
        .await
        .map_err(internal)?;
    catalog::increment_catalog_version(&state.pool, store_id)
        .await
        .map_err(internal)?;

    push_to_edge_if_online(&state, store_id, CatalogOp::DeleteTable { id: table_id });

    Ok(Json(CatalogOpResult::ok()))
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

// ── Helpers ──

use super::tenant::verify_store;

fn internal(e: impl std::fmt::Display) -> AppError {
    tracing::error!("Catalog query error: {e}");
    AppError::new(ErrorCode::InternalError)
}

/// Fire-and-forget push CatalogOp to edge if it's currently connected.
///
/// Does NOT block on response. Edge will process the op and update its local SQLite.
/// If edge is offline, the op is simply dropped — edge will catch up via FullSync on reconnect.
fn push_to_edge_if_online(state: &AppState, store_id: i64, op: CatalogOp) {
    let sender = match state.edges.connected.get(&store_id) {
        Some(s) => s.clone(),
        None => return,
    };

    let msg = CloudMessage::Rpc {
        id: format!("push-{}", uuid::Uuid::new_v4()),
        payload: Box::new(CloudRpc::CatalogOp(Box::new(op))),
    };

    let _ = sender.try_send(msg);
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
