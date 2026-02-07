//! Price Rule API Handlers

use axum::{
    Json,
    extract::{Extension, Path, State},
};

use crate::audit::{create_diff, create_snapshot, AuditAction};
use crate::audit_log;
use crate::auth::CurrentUser;
use crate::core::ServerState;
use crate::db::repository::price_rule;
use crate::utils::{AppError, AppResult};
use shared::models::{PriceRule, PriceRuleCreate, PriceRuleUpdate, ProductScope};

const RESOURCE: &str = "price_rule";

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
                || (r.product_scope == ProductScope::Product
                    && r.target_id == Some(product_id))
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
        .ok_or_else(|| AppError::not_found(format!("Price rule {} not found", id)))?;
    Ok(Json(rule))
}

/// POST /api/price-rules - 创建价格规则
pub async fn create(
    State(state): State<ServerState>,
    Extension(current_user): Extension<CurrentUser>,
    Json(payload): Json<PriceRuleCreate>,
) -> AppResult<Json<PriceRule>> {
    let rule = price_rule::create(&state.pool, payload).await?;

    let id = rule.id.to_string();

    audit_log!(
        state.audit_service,
        AuditAction::PriceRuleCreated,
        "price_rule", &id,
        operator_id = Some(current_user.id.clone()),
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
    // 查询旧值（用于审计 diff）
    let old_rule = price_rule::find_by_id(&state.pool, id)
        .await?
        .ok_or_else(|| AppError::not_found(format!("Price rule {}", id)))?;

    let rule = price_rule::update(&state.pool, id, payload).await?;

    let id_str = id.to_string();

    audit_log!(
        state.audit_service,
        AuditAction::PriceRuleUpdated,
        "price_rule", &id_str,
        operator_id = Some(current_user.id.clone()),
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
    let name_for_audit = price_rule::find_by_id(&state.pool, id).await.ok().flatten()
        .map(|r| r.name.clone()).unwrap_or_default();
    let result = price_rule::delete(&state.pool, id).await?;

    let id_str = id.to_string();

    if result {
        audit_log!(
            state.audit_service,
            AuditAction::PriceRuleDeleted,
            "price_rule", &id_str,
            operator_id = Some(current_user.id.clone()),
            operator_name = Some(current_user.display_name.clone()),
            details = serde_json::json!({"name": name_for_audit})
        );

        state
            .broadcast_sync::<()>(RESOURCE, "deleted", &id_str, None)
            .await;
    }

    Ok(Json(result))
}
