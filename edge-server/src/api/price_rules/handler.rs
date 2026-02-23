//! Price Rule API Handlers

use axum::{
    Json,
    extract::{Extension, Path, State},
};

use crate::audit::{AuditAction, create_diff, create_snapshot};
use crate::audit_log;
use crate::auth::CurrentUser;
use crate::core::ServerState;
use crate::db::repository::price_rule;
use crate::utils::validation::{
    MAX_NAME_LEN, MAX_NOTE_LEN, MAX_RECEIPT_NAME_LEN, MAX_SHORT_TEXT_LEN, validate_optional_text,
    validate_required_text,
};
use crate::utils::{AppError, AppResult};
use shared::error::ErrorCode;
use shared::models::price_rule::AdjustmentType;
use shared::models::{PriceRule, PriceRuleCreate, PriceRuleUpdate, ProductScope};

const RESOURCE: &str = "price_rule";

fn validate_create(payload: &PriceRuleCreate) -> AppResult<()> {
    validate_required_text(&payload.name, "name", MAX_NAME_LEN)?;
    validate_required_text(&payload.display_name, "display_name", MAX_NAME_LEN)?;
    validate_required_text(&payload.receipt_name, "receipt_name", MAX_RECEIPT_NAME_LEN)?;
    validate_optional_text(&payload.description, "description", MAX_NOTE_LEN)?;
    validate_optional_text(&payload.zone_scope, "zone_scope", MAX_SHORT_TEXT_LEN)?;
    validate_optional_text(
        &payload.active_start_time,
        "active_start_time",
        MAX_SHORT_TEXT_LEN,
    )?;
    validate_optional_text(
        &payload.active_end_time,
        "active_end_time",
        MAX_SHORT_TEXT_LEN,
    )?;
    Ok(())
}

fn validate_update(payload: &PriceRuleUpdate) -> AppResult<()> {
    if let Some(name) = &payload.name {
        validate_required_text(name, "name", MAX_NAME_LEN)?;
    }
    if let Some(display_name) = &payload.display_name {
        validate_required_text(display_name, "display_name", MAX_NAME_LEN)?;
    }
    if let Some(receipt_name) = &payload.receipt_name {
        validate_required_text(receipt_name, "receipt_name", MAX_RECEIPT_NAME_LEN)?;
    }
    validate_optional_text(&payload.description, "description", MAX_NOTE_LEN)?;
    validate_optional_text(&payload.zone_scope, "zone_scope", MAX_SHORT_TEXT_LEN)?;
    validate_optional_text(
        &payload.active_start_time,
        "active_start_time",
        MAX_SHORT_TEXT_LEN,
    )?;
    validate_optional_text(
        &payload.active_end_time,
        "active_end_time",
        MAX_SHORT_TEXT_LEN,
    )?;
    Ok(())
}

fn validate_adjustment_value(adjustment_type: &AdjustmentType, value: f64) -> Result<(), AppError> {
    if !value.is_finite() {
        return Err(AppError::validation(
            "adjustment_value must be a finite number",
        ));
    }
    if value < 0.0 {
        return Err(AppError::validation(
            "adjustment_value must be non-negative",
        ));
    }
    match adjustment_type {
        AdjustmentType::Percentage => {
            if value > 100.0 {
                return Err(AppError::validation(
                    "Percentage adjustment_value must be between 0 and 100",
                ));
            }
        }
        AdjustmentType::FixedAmount => {
            if value > 1_000_000.0 {
                return Err(AppError::validation(
                    "FixedAmount adjustment_value must not exceed 1,000,000",
                ));
            }
        }
    }
    Ok(())
}

/// GET /api/price-rules - 获取所有价格规则
pub async fn list(State(state): State<ServerState>) -> AppResult<Json<Vec<PriceRule>>> {
    let rules = price_rule::find_all(&state.pool).await?;
    Ok(Json(rules))
}

/// GET /api/price-rules/by-scope/:scope - 按作用域获取价格规则
pub async fn list_by_scope(
    State(state): State<ServerState>,
    Path(scope): Path<String>,
) -> AppResult<Json<Vec<PriceRule>>> {
    let scope = match scope.to_uppercase().as_str() {
        "GLOBAL" => ProductScope::Global,
        "CATEGORY" => ProductScope::Category,
        "TAG" => ProductScope::Tag,
        "PRODUCT" => ProductScope::Product,
        _ => return Err(AppError::validation(format!("Invalid scope: {}", scope))),
    };

    let rules = price_rule::find_by_scope(&state.pool, scope).await?;
    Ok(Json(rules))
}

/// GET /api/price-rules/for-product/:product_id - 获取适用于商品的价格规则
pub async fn list_for_product(
    State(state): State<ServerState>,
    Path(product_id): Path<i64>,
) -> AppResult<Json<Vec<PriceRule>>> {
    // 获取所有规则后在内存中筛选适用于该商品的规则
    let all_rules = price_rule::find_all(&state.pool).await?;
    let rules: Vec<PriceRule> = all_rules
        .into_iter()
        .filter(|r| {
            r.product_scope == ProductScope::Global
                || (r.product_scope == ProductScope::Product && r.target_id == Some(product_id))
        })
        .collect();
    Ok(Json(rules))
}

/// GET /api/price-rules/:id - 获取单个价格规则
pub async fn get_by_id(
    State(state): State<ServerState>,
    Path(id): Path<i64>,
) -> AppResult<Json<PriceRule>> {
    let rule = price_rule::find_by_id(&state.pool, id)
        .await?
        .ok_or_else(|| {
            AppError::with_message(
                ErrorCode::PriceRuleNotFound,
                format!("Price rule {} not found", id),
            )
        })?;
    Ok(Json(rule))
}

/// POST /api/price-rules - 创建价格规则
pub async fn create(
    State(state): State<ServerState>,
    Extension(current_user): Extension<CurrentUser>,
    Json(payload): Json<PriceRuleCreate>,
) -> AppResult<Json<PriceRule>> {
    validate_create(&payload)?;
    validate_adjustment_value(&payload.adjustment_type, payload.adjustment_value)?;
    let rule = price_rule::create(&state.pool, None, payload).await?;

    let id = rule.id.to_string();

    audit_log!(
        state.audit_service,
        AuditAction::PriceRuleCreated,
        "price_rule",
        &id,
        operator_id = Some(current_user.id),
        operator_name = Some(current_user.display_name.clone()),
        details = create_snapshot(&rule, "price_rule")
    );

    state
        .broadcast_sync(RESOURCE, "created", &id, Some(&rule))
        .await;

    Ok(Json(rule))
}

/// PUT /api/price-rules/:id - 更新价格规则
pub async fn update(
    State(state): State<ServerState>,
    Extension(current_user): Extension<CurrentUser>,
    Path(id): Path<i64>,
    Json(payload): Json<PriceRuleUpdate>,
) -> AppResult<Json<PriceRule>> {
    validate_update(&payload)?;

    // 查询旧值（用于审计 diff + 部分更新验证）
    let old_rule = price_rule::find_by_id(&state.pool, id)
        .await?
        .ok_or_else(|| {
            AppError::with_message(
                ErrorCode::PriceRuleNotFound,
                format!("Price rule {} not found", id),
            )
        })?;

    // 验证 adjustment_value（部分更新时用旧值补齐）
    let adj_type = payload
        .adjustment_type
        .as_ref()
        .unwrap_or(&old_rule.adjustment_type);
    let adj_value = payload
        .adjustment_value
        .unwrap_or(old_rule.adjustment_value);
    validate_adjustment_value(adj_type, adj_value)?;

    let rule = price_rule::update(&state.pool, id, payload).await?;

    let id_str = id.to_string();

    audit_log!(
        state.audit_service,
        AuditAction::PriceRuleUpdated,
        "price_rule",
        &id_str,
        operator_id = Some(current_user.id),
        operator_name = Some(current_user.display_name.clone()),
        details = create_diff(&old_rule, &rule, "price_rule")
    );

    state
        .broadcast_sync(RESOURCE, "updated", &id_str, Some(&rule))
        .await;

    Ok(Json(rule))
}

/// DELETE /api/price-rules/:id - 删除价格规则 (软删除)
pub async fn delete(
    State(state): State<ServerState>,
    Extension(current_user): Extension<CurrentUser>,
    Path(id): Path<i64>,
) -> AppResult<Json<bool>> {
    let name_for_audit = price_rule::find_by_id(&state.pool, id)
        .await
        .ok()
        .flatten()
        .map(|r| r.name.clone())
        .unwrap_or_default();
    let result = price_rule::delete(&state.pool, id).await?;

    let id_str = id.to_string();

    if result {
        audit_log!(
            state.audit_service,
            AuditAction::PriceRuleDeleted,
            "price_rule",
            &id_str,
            operator_id = Some(current_user.id),
            operator_name = Some(current_user.display_name.clone()),
            details = serde_json::json!({"name": name_for_audit})
        );

        state
            .broadcast_sync::<()>(RESOURCE, "deleted", &id_str, None)
            .await;
    }

    Ok(Json(result))
}
