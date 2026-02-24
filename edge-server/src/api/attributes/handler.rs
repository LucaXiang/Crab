//! Attribute API Handlers

use axum::{
    Json,
    extract::{Extension, Path, State},
};

use crate::audit::{AuditAction, create_diff, create_snapshot};
use crate::audit_log;
use crate::auth::CurrentUser;
use crate::core::ServerState;
use crate::db::repository::attribute;
use crate::utils::validation::{
    MAX_NAME_LEN, MAX_RECEIPT_NAME_LEN, validate_optional_text, validate_required_text,
};
use crate::utils::{AppError, AppResult};
use shared::error::ErrorCode;
use shared::message::SyncChangeType;
use shared::models::{Attribute, AttributeCreate, AttributeOptionInput, AttributeUpdate};

use shared::cloud::SyncResource;
const RESOURCE: SyncResource = SyncResource::Attribute;

/// Refresh product cache for all products using this attribute and broadcast sync for each
async fn refresh_and_broadcast_products(state: &ServerState, attribute_id: i64) {
    match state
        .catalog_service
        .refresh_products_with_attribute(attribute_id)
        .await
    {
        Ok(product_ids) => {
            for pid in product_ids {
                let product = state.catalog_service.get_product(pid);
                let pid_str = pid.to_string();
                state
                    .broadcast_sync(
                        SyncResource::Product,
                        SyncChangeType::Updated,
                        &pid_str,
                        product.as_ref(),
                        false,
                    )
                    .await;
            }
        }
        Err(e) => {
            tracing::warn!(
                "Failed to refresh product cache after attribute {} update: {e}",
                attribute_id
            );
        }
    }
}

fn validate_create(payload: &AttributeCreate) -> AppResult<()> {
    validate_required_text(&payload.name, "name", MAX_NAME_LEN)?;
    validate_optional_text(&payload.receipt_name, "receipt_name", MAX_RECEIPT_NAME_LEN)?;
    validate_optional_text(
        &payload.kitchen_print_name,
        "kitchen_print_name",
        MAX_RECEIPT_NAME_LEN,
    )?;
    Ok(())
}

fn validate_update(payload: &AttributeUpdate) -> AppResult<()> {
    if let Some(name) = &payload.name {
        validate_required_text(name, "name", MAX_NAME_LEN)?;
    }
    validate_optional_text(&payload.receipt_name, "receipt_name", MAX_RECEIPT_NAME_LEN)?;
    validate_optional_text(
        &payload.kitchen_print_name,
        "kitchen_print_name",
        MAX_RECEIPT_NAME_LEN,
    )?;
    Ok(())
}

/// Validate an option input before saving
fn validate_option(opt: &AttributeOptionInput) -> AppResult<()> {
    validate_required_text(&opt.name, "option name", MAX_NAME_LEN)?;
    validate_optional_text(
        &opt.receipt_name,
        "option receipt_name",
        MAX_RECEIPT_NAME_LEN,
    )?;
    validate_optional_text(
        &opt.kitchen_print_name,
        "option kitchen_print_name",
        MAX_RECEIPT_NAME_LEN,
    )?;
    if !opt.price_modifier.is_finite() {
        return Err(AppError::validation(
            "price_modifier must be a finite number".to_string(),
        ));
    }
    if let Some(mq) = opt.max_quantity
        && mq < 1
    {
        return Err(AppError::validation(format!(
            "max_quantity must be at least 1, got {}",
            mq
        )));
    }
    Ok(())
}

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
        .ok_or_else(|| {
            AppError::with_message(
                ErrorCode::AttributeNotFound,
                format!("Attribute {} not found", id),
            )
        })?;
    Ok(Json(attr))
}

/// POST /api/attributes - 创建属性
pub async fn create(
    State(state): State<ServerState>,
    Extension(current_user): Extension<CurrentUser>,
    Json(payload): Json<AttributeCreate>,
) -> AppResult<Json<Attribute>> {
    validate_create(&payload)?;

    let attr = attribute::create(&state.pool, None, payload).await?;

    let id = attr.id.to_string();

    audit_log!(
        state.audit_service,
        AuditAction::AttributeCreated,
        "attribute",
        &id,
        operator_id = Some(current_user.id),
        operator_name = Some(current_user.display_name.clone()),
        details = create_snapshot(&attr, "attribute")
    );

    state
        .broadcast_sync(RESOURCE, SyncChangeType::Created, &id, Some(&attr), false)
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
    validate_update(&payload)?;

    // 查询旧值（用于审计 diff）
    let old_attr = attribute::find_by_id(&state.pool, id)
        .await?
        .ok_or_else(|| {
            AppError::with_message(
                ErrorCode::AttributeNotFound,
                format!("Attribute {} not found", id),
            )
        })?;

    let attr = attribute::update(&state.pool, id, payload).await?;

    let id_str = id.to_string();

    audit_log!(
        state.audit_service,
        AuditAction::AttributeUpdated,
        "attribute",
        &id_str,
        operator_id = Some(current_user.id),
        operator_name = Some(current_user.display_name.clone()),
        details = create_diff(&old_attr, &attr, "attribute")
    );

    state
        .broadcast_sync(
            RESOURCE,
            SyncChangeType::Updated,
            &id_str,
            Some(&attr),
            false,
        )
        .await;

    // 刷新引用此属性的产品缓存并广播 sync
    refresh_and_broadcast_products(&state, id).await;

    Ok(Json(attr))
}

/// DELETE /api/attributes/:id - 删除属性
pub async fn delete(
    State(state): State<ServerState>,
    Extension(current_user): Extension<CurrentUser>,
    Path(id): Path<i64>,
) -> AppResult<Json<bool>> {
    // 检查是否有商品/分类正在使用此属性
    let binding_count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM attribute_binding WHERE attribute_id = ?",
        id
    )
    .fetch_one(&state.pool)
    .await
    .unwrap_or(0);

    if binding_count > 0 {
        return Err(AppError::with_message(
            ErrorCode::AttributeInUse,
            format!(
                "Cannot delete attribute: {} product/category binding(s) exist",
                binding_count
            ),
        ));
    }

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
            "attribute",
            &id_str,
            operator_id = Some(current_user.id),
            operator_name = Some(current_user.display_name.clone()),
            details = serde_json::json!({"name": name_for_audit})
        );

        state
            .broadcast_sync::<()>(RESOURCE, SyncChangeType::Deleted, &id_str, None, false)
            .await;

        // 刷新引用此属性的产品缓存
        refresh_and_broadcast_products(&state, id).await;
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
    validate_option(&option)?;

    // 读取当前属性，将新选项追加到现有选项列表后，整体替换
    let current = attribute::find_by_id(&state.pool, id)
        .await?
        .ok_or_else(|| {
            AppError::with_message(
                ErrorCode::AttributeNotFound,
                format!("Attribute {} not found", id),
            )
        })?;

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
        "attribute",
        &id_str,
        operator_id = Some(current_user.id),
        operator_name = Some(current_user.display_name.clone()),
        details = serde_json::json!({"op": "add_option", "option_name": &option.name})
    );

    // 广播同步通知
    state
        .broadcast_sync(
            RESOURCE,
            SyncChangeType::Updated,
            &id_str,
            Some(&attr),
            false,
        )
        .await;

    // 刷新引用此属性的产品缓存
    refresh_and_broadcast_products(&state, id).await;

    Ok(Json(attr))
}

/// PUT /api/attributes/:id/options/:idx - 更新选项
pub async fn update_option(
    State(state): State<ServerState>,
    Extension(current_user): Extension<CurrentUser>,
    Path((id, idx)): Path<(i64, usize)>,
    Json(option): Json<AttributeOptionInput>,
) -> AppResult<Json<Attribute>> {
    validate_option(&option)?;

    let current = attribute::find_by_id(&state.pool, id)
        .await?
        .ok_or_else(|| {
            AppError::with_message(
                ErrorCode::AttributeNotFound,
                format!("Attribute {} not found", id),
            )
        })?;

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
        "attribute",
        &id_str,
        operator_id = Some(current_user.id),
        operator_name = Some(current_user.display_name.clone()),
        details =
            serde_json::json!({"op": "update_option", "index": idx, "option_name": option_name})
    );

    // 广播同步通知
    state
        .broadcast_sync(
            RESOURCE,
            SyncChangeType::Updated,
            &id_str,
            Some(&attr),
            false,
        )
        .await;

    // 刷新引用此属性的产品缓存
    refresh_and_broadcast_products(&state, id).await;

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
        .ok_or_else(|| {
            AppError::with_message(
                ErrorCode::AttributeNotFound,
                format!("Attribute {} not found", id),
            )
        })?;

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
        "attribute",
        &id_str,
        operator_id = Some(current_user.id),
        operator_name = Some(current_user.display_name.clone()),
        details = serde_json::json!({"op": "remove_option", "index": idx})
    );

    // 广播同步通知
    state
        .broadcast_sync(
            RESOURCE,
            SyncChangeType::Updated,
            &id_str,
            Some(&attr),
            false,
        )
        .await;

    // 刷新引用此属性的产品缓存
    refresh_and_broadcast_products(&state, id).await;

    Ok(Json(attr))
}
