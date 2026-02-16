//! AttributeBinding API Handlers - 产品属性绑定

use axum::{
    Extension, Json,
    extract::{Path, State},
};
use serde::{Deserialize, Serialize};

use crate::audit::AuditAction;
use crate::audit_log;
use crate::auth::CurrentUser;
use crate::core::ServerState;
use crate::db::repository::attribute;
use crate::utils::{AppError, AppResult};
use shared::models::{Attribute, AttributeBinding};

/// 创建绑定的请求体
#[derive(Debug, Deserialize)]
pub struct CreateBindingRequest {
    pub product_id: i64,
    pub attribute_id: i64,
    #[serde(default)]
    pub is_required: bool,
    #[serde(default)]
    pub display_order: i32,
    pub default_option_ids: Option<Vec<i32>>,
}

/// 更新绑定的请求体
#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateBindingRequest {
    pub is_required: Option<bool>,
    pub display_order: Option<i32>,
    pub default_option_ids: Option<Vec<i32>>,
}

/// 绑定响应 (包含属性详情)
#[derive(Debug, Serialize)]
pub struct BindingWithAttribute {
    pub binding: AttributeBinding,
    pub attribute: Attribute,
}

/// POST /api/has-attribute - 创建产品属性绑定
pub async fn create(
    State(state): State<ServerState>,
    Extension(current_user): Extension<CurrentUser>,
    Json(payload): Json<CreateBindingRequest>,
) -> AppResult<Json<AttributeBinding>> {
    // 查询产品的 category_id，检查分类是否已绑定此属性
    let category_id: Option<i64> = sqlx::query_scalar!(
        "SELECT category_id FROM product WHERE id = ?",
        payload.product_id,
    )
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| AppError::database(e.to_string()))?;

    if let Some(cat_id) = category_id
        && attribute::has_binding(&state.pool, "category", cat_id, payload.attribute_id).await?
    {
        return Err(AppError::validation(
            "This attribute is already inherited from the category, cannot add duplicate binding"
                .to_string(),
        ));
    }

    let binding = attribute::link(
        &state.pool,
        "product",
        payload.product_id,
        payload.attribute_id,
        payload.is_required,
        payload.display_order,
        payload.default_option_ids,
    )
    .await?;

    // 查询属性名用于审计
    let attr_name = attribute::find_by_id(&state.pool, payload.attribute_id)
        .await
        .ok()
        .flatten()
        .map(|a| a.name)
        .unwrap_or_default();

    let binding_id = binding.id.to_string();
    audit_log!(
        state.audit_service,
        AuditAction::ProductUpdated,
        "attribute_binding",
        &binding_id,
        operator_id = Some(current_user.id),
        operator_name = Some(current_user.display_name.clone()),
        details = serde_json::json!({
            "op": "bind_attribute",
            "product_id": payload.product_id,
            "attribute_id": payload.attribute_id,
            "attribute_name": attr_name,
            "is_required": payload.is_required,
        })
    );

    // Refresh product cache (attribute bindings changed)
    if let Err(e) = state
        .catalog_service
        .refresh_product_cache(payload.product_id)
        .await
    {
        tracing::warn!(
            "Failed to refresh product cache for {}: {}",
            payload.product_id,
            e
        );
    }

    Ok(Json(binding))
}

/// GET /api/has-attribute/{id} - 获取单个绑定
pub async fn get_by_id(
    State(state): State<ServerState>,
    Path(id): Path<i64>,
) -> AppResult<Json<AttributeBinding>> {
    attribute::find_binding_by_id(&state.pool, id)
        .await?
        .map(Json)
        .ok_or_else(|| AppError::not_found(format!("Binding {} not found", id)))
}

/// PUT /api/has-attribute/{id} - 更新绑定
pub async fn update(
    State(state): State<ServerState>,
    Extension(current_user): Extension<CurrentUser>,
    Path(id): Path<i64>,
    Json(payload): Json<UpdateBindingRequest>,
) -> AppResult<Json<AttributeBinding>> {
    let binding = attribute::update_binding(
        &state.pool,
        id,
        payload.is_required,
        payload.display_order,
        payload.default_option_ids,
    )
    .await?;

    // 查询属性名用于审计
    let attr_name = attribute::find_by_id(&state.pool, binding.attribute_id)
        .await
        .ok()
        .flatten()
        .map(|a| a.name)
        .unwrap_or_default();

    let id_str = id.to_string();
    audit_log!(
        state.audit_service,
        AuditAction::ProductUpdated,
        "attribute_binding",
        &id_str,
        operator_id = Some(current_user.id),
        operator_name = Some(current_user.display_name.clone()),
        details = serde_json::json!({
            "op": "update_binding",
            "attribute_name": attr_name,
            "is_required": payload.is_required,
            "display_order": payload.display_order,
        })
    );

    Ok(Json(binding))
}

/// DELETE /api/has-attribute/{id} - 删除绑定
pub async fn delete(
    State(state): State<ServerState>,
    Extension(current_user): Extension<CurrentUser>,
    Path(id): Path<i64>,
) -> AppResult<Json<bool>> {
    // Get the binding before deleting (for cache refresh and audit)
    let binding = attribute::find_binding_by_id(&state.pool, id).await?;
    let owner_id = binding.as_ref().map(|b| b.owner_id);
    let owner_type = binding.as_ref().map(|b| b.owner_type.clone());

    // 查询属性名用于审计
    let attribute_id = binding.as_ref().map(|b| b.attribute_id);
    let attr_name = if let Some(aid) = attribute_id {
        attribute::find_by_id(&state.pool, aid)
            .await
            .ok()
            .flatten()
            .map(|a| a.name)
    } else {
        None
    };

    attribute::delete_binding(&state.pool, id).await?;

    let id_str = id.to_string();
    audit_log!(
        state.audit_service,
        AuditAction::ProductUpdated,
        "attribute_binding",
        &id_str,
        operator_id = Some(current_user.id),
        operator_name = Some(current_user.display_name.clone()),
        details = serde_json::json!({
            "op": "unbind_attribute",
            "owner_id": owner_id,
            "owner_type": &owner_type,
            "attribute_name": attr_name,
        })
    );

    // Refresh product cache if the binding was for a product
    if let (Some(oid), Some(otype)) = (owner_id, owner_type)
        && otype == "product"
        && let Err(e) = state.catalog_service.refresh_product_cache(oid).await
    {
        tracing::warn!("Failed to refresh product cache for {}: {}", oid, e);
    }

    Ok(Json(true))
}

/// GET /api/has-attribute/product/{product_id} - 获取产品的所有属性绑定
pub async fn list_by_product(
    State(state): State<ServerState>,
    Path(product_id): Path<i64>,
) -> AppResult<Json<Vec<BindingWithAttribute>>> {
    let bindings = attribute::find_bindings_for_owner(&state.pool, "product", product_id).await?;

    let result: Vec<BindingWithAttribute> = bindings
        .into_iter()
        .map(|(binding, attribute)| BindingWithAttribute { binding, attribute })
        .collect();

    Ok(Json(result))
}
