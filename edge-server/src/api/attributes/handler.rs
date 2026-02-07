//! Attribute API Handlers

use axum::{
    Json,
    extract::{Extension, Path, State},
};

use crate::audit::{create_diff, create_snapshot, AuditAction};
use crate::audit_log;
use crate::auth::CurrentUser;
use crate::core::ServerState;
use crate::db::repository::attribute;
use crate::utils::{AppError, AppResult};
use shared::models::{Attribute, AttributeCreate, AttributeOptionInput, AttributeUpdate};

const RESOURCE: &str = "attribute";

/// GET /api/attributes - 获取所有属性
pub async fn list(State(state): State<ServerState>) -> AppResult<Json<Vec<Attribute>>> {
    let attrs = attribute::find_all(&state.pool).await?;
    Ok(Json(attrs))
}

/// GET /api/attributes/:id - 获取单个属性
pub async fn get_by_id(
    State(state): State<ServerState>,
    Path(id): Path<i64>,
) -> AppResult<Json<Attribute>> {
    let attr = attribute::find_by_id(&state.pool, id)
        .await?
        .ok_or_else(|| AppError::not_found(format!("Attribute {} not found", id)))?;
    Ok(Json(attr))
}

/// POST /api/attributes - 创建属性
pub async fn create(
    State(state): State<ServerState>,
    Extension(current_user): Extension<CurrentUser>,
    Json(payload): Json<AttributeCreate>,
) -> AppResult<Json<Attribute>> {
    let attr = attribute::create(&state.pool, payload).await?;

    let id = attr.id.to_string();

    audit_log!(
        state.audit_service,
        AuditAction::AttributeCreated,
        "attribute", &id,
        operator_id = Some(current_user.id.clone()),
        operator_name = Some(current_user.display_name.clone()),
        details = create_snapshot(&attr, "attribute")
    );

    state
        .broadcast_sync(RESOURCE, "created", &id, Some(&attr))
        .await;

    Ok(Json(attr))
}

/// PUT /api/attributes/:id - 更新属性
pub async fn update(
    State(state): State<ServerState>,
    Extension(current_user): Extension<CurrentUser>,
    Path(id): Path<i64>,
    Json(payload): Json<AttributeUpdate>,
) -> AppResult<Json<Attribute>> {
    // 查询旧值（用于审计 diff）
    let old_attr = attribute::find_by_id(&state.pool, id)
        .await?
        .ok_or_else(|| AppError::not_found(format!("Attribute {}", id)))?;

    let attr = attribute::update(&state.pool, id, payload).await?;

    let id_str = id.to_string();

    audit_log!(
        state.audit_service,
        AuditAction::AttributeUpdated,
        "attribute", &id_str,
        operator_id = Some(current_user.id.clone()),
        operator_name = Some(current_user.display_name.clone()),
        details = create_diff(&old_attr, &attr, "attribute")
    );

    state
        .broadcast_sync(RESOURCE, "updated", &id_str, Some(&attr))
        .await;

    // 刷新引用此属性的产品缓存
    if let Err(e) = state.catalog_service.refresh_products_with_attribute(&id_str).await {
        tracing::warn!("Failed to refresh product cache after attribute update: {e}");
    }

    Ok(Json(attr))
}

/// DELETE /api/attributes/:id - 删除属性
pub async fn delete(
    State(state): State<ServerState>,
    Extension(current_user): Extension<CurrentUser>,
    Path(id): Path<i64>,
) -> AppResult<Json<bool>> {
    let name_for_audit = attribute::find_by_id(&state.pool, id)
        .await
        .ok()
        .flatten()
        .map(|a| a.name.clone())
        .unwrap_or_default();
    let result = attribute::delete(&state.pool, id).await?;

    let id_str = id.to_string();

    if result {
        audit_log!(
            state.audit_service,
            AuditAction::AttributeDeleted,
            "attribute", &id_str,
            operator_id = Some(current_user.id.clone()),
            operator_name = Some(current_user.display_name.clone()),
            details = serde_json::json!({"name": name_for_audit})
        );

        state
            .broadcast_sync::<()>(RESOURCE, "deleted", &id_str, None)
            .await;

        // 刷新引用此属性的产品缓存
        if let Err(e) = state.catalog_service.refresh_products_with_attribute(&id_str).await {
            tracing::warn!("Failed to refresh product cache after attribute delete: {e}");
        }
    }

    Ok(Json(result))
}

/// POST /api/attributes/:id/options - 添加选项
pub async fn add_option(
    State(state): State<ServerState>,
    Extension(current_user): Extension<CurrentUser>,
    Path(id): Path<i64>,
    Json(option): Json<AttributeOptionInput>,
) -> AppResult<Json<Attribute>> {
    // 读取当前属性，将新选项追加到现有选项列表后，整体替换
    let current = attribute::find_by_id(&state.pool, id)
        .await?
        .ok_or_else(|| AppError::not_found(format!("Attribute {} not found", id)))?;

    let mut options: Vec<AttributeOptionInput> = current
        .options
        .iter()
        .map(|o| AttributeOptionInput {
            name: o.name.clone(),
            price_modifier: o.price_modifier,
            display_order: o.display_order,
            receipt_name: o.receipt_name.clone(),
            kitchen_print_name: o.kitchen_print_name.clone(),
            enable_quantity: o.enable_quantity,
            max_quantity: o.max_quantity,
        })
        .collect();

    options.push(option.clone());

    let update_data = AttributeUpdate {
        options: Some(options),
        ..Default::default()
    };

    let attr = attribute::update(&state.pool, id, update_data).await?;

    let id_str = id.to_string();

    audit_log!(
        state.audit_service,
        AuditAction::AttributeUpdated,
        "attribute", &id_str,
        operator_id = Some(current_user.id.clone()),
        operator_name = Some(current_user.display_name.clone()),
        details = serde_json::json!({"op": "add_option", "option_name": &option.name})
    );

    // 广播同步通知
    state
        .broadcast_sync(RESOURCE, "updated", &id_str, Some(&attr))
        .await;

    // 刷新引用此属性的产品缓存
    if let Err(e) = state.catalog_service.refresh_products_with_attribute(&id_str).await {
        tracing::warn!("Failed to refresh product cache after option add: {e}");
    }

    Ok(Json(attr))
}

/// PUT /api/attributes/:id/options/:idx - 更新选项
pub async fn update_option(
    State(state): State<ServerState>,
    Extension(current_user): Extension<CurrentUser>,
    Path((id, idx)): Path<(i64, usize)>,
    Json(option): Json<AttributeOptionInput>,
) -> AppResult<Json<Attribute>> {
    let current = attribute::find_by_id(&state.pool, id)
        .await?
        .ok_or_else(|| AppError::not_found(format!("Attribute {} not found", id)))?;

    let mut options: Vec<AttributeOptionInput> = current
        .options
        .iter()
        .map(|o| AttributeOptionInput {
            name: o.name.clone(),
            price_modifier: o.price_modifier,
            display_order: o.display_order,
            receipt_name: o.receipt_name.clone(),
            kitchen_print_name: o.kitchen_print_name.clone(),
            enable_quantity: o.enable_quantity,
            max_quantity: o.max_quantity,
        })
        .collect();

    if idx >= options.len() {
        return Err(AppError::validation(format!(
            "Option index {} out of range (total: {})",
            idx,
            options.len()
        )));
    }

    let option_name = option.name.clone();
    options[idx] = option;

    let update_data = AttributeUpdate {
        options: Some(options),
        ..Default::default()
    };

    let attr = attribute::update(&state.pool, id, update_data).await?;

    let id_str = id.to_string();

    audit_log!(
        state.audit_service,
        AuditAction::AttributeUpdated,
        "attribute", &id_str,
        operator_id = Some(current_user.id.clone()),
        operator_name = Some(current_user.display_name.clone()),
        details = serde_json::json!({"op": "update_option", "index": idx, "option_name": option_name})
    );

    // 广播同步通知
    state
        .broadcast_sync(RESOURCE, "updated", &id_str, Some(&attr))
        .await;

    // 刷新引用此属性的产品缓存
    if let Err(e) = state.catalog_service.refresh_products_with_attribute(&id_str).await {
        tracing::warn!("Failed to refresh product cache after option update: {e}");
    }

    Ok(Json(attr))
}

/// DELETE /api/attributes/:id/options/:idx - 删除选项
pub async fn remove_option(
    State(state): State<ServerState>,
    Extension(current_user): Extension<CurrentUser>,
    Path((id, idx)): Path<(i64, usize)>,
) -> AppResult<Json<Attribute>> {
    let current = attribute::find_by_id(&state.pool, id)
        .await?
        .ok_or_else(|| AppError::not_found(format!("Attribute {} not found", id)))?;

    let mut options: Vec<AttributeOptionInput> = current
        .options
        .iter()
        .map(|o| AttributeOptionInput {
            name: o.name.clone(),
            price_modifier: o.price_modifier,
            display_order: o.display_order,
            receipt_name: o.receipt_name.clone(),
            kitchen_print_name: o.kitchen_print_name.clone(),
            enable_quantity: o.enable_quantity,
            max_quantity: o.max_quantity,
        })
        .collect();

    if idx >= options.len() {
        return Err(AppError::validation(format!(
            "Option index {} out of range (total: {})",
            idx,
            options.len()
        )));
    }

    options.remove(idx);

    let update_data = AttributeUpdate {
        options: Some(options),
        ..Default::default()
    };

    let attr = attribute::update(&state.pool, id, update_data).await?;

    let id_str = id.to_string();

    audit_log!(
        state.audit_service,
        AuditAction::AttributeUpdated,
        "attribute", &id_str,
        operator_id = Some(current_user.id.clone()),
        operator_name = Some(current_user.display_name.clone()),
        details = serde_json::json!({"op": "remove_option", "index": idx})
    );

    // 广播同步通知
    state
        .broadcast_sync(RESOURCE, "updated", &id_str, Some(&attr))
        .await;

    // 刷新引用此属性的产品缓存
    if let Err(e) = state.catalog_service.refresh_products_with_attribute(&id_str).await {
        tracing::warn!("Failed to refresh product cache after option remove: {e}");
    }

    Ok(Json(attr))
}
