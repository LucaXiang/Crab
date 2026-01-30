//! Price Rule API Handlers

use axum::{
    Json,
    extract::{Path, State},
};

use crate::core::ServerState;
use crate::db::models::{PriceRule, PriceRuleCreate, PriceRuleUpdate, ProductScope};
use crate::db::repository::PriceRuleRepository;
use crate::utils::{AppError, AppResult};

const RESOURCE: &str = "price_rule";

/// GET /api/price-rules - 获取所有价格规则
pub async fn list(State(state): State<ServerState>) -> AppResult<Json<Vec<PriceRule>>> {
    let repo = PriceRuleRepository::new(state.db.clone());
    let rules = repo
        .find_all()
        .await
        ?;
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

    let repo = PriceRuleRepository::new(state.db.clone());
    let rules = repo
        .find_by_scope(scope)
        .await
        ?;
    Ok(Json(rules))
}

/// GET /api/price-rules/for-product/:product_id - 获取适用于商品的价格规则
pub async fn list_for_product(
    State(state): State<ServerState>,
    Path(product_id): Path<String>,
) -> AppResult<Json<Vec<PriceRule>>> {
    let repo = PriceRuleRepository::new(state.db.clone());
    let rules = repo
        .find_for_product(&product_id)
        .await
        ?;
    Ok(Json(rules))
}

/// GET /api/price-rules/:id - 获取单个价格规则
pub async fn get_by_id(
    State(state): State<ServerState>,
    Path(id): Path<String>,
) -> AppResult<Json<PriceRule>> {
    let repo = PriceRuleRepository::new(state.db.clone());
    let rule = repo
        .find_by_id(&id)
        .await
        ?
        .ok_or_else(|| AppError::not_found(format!("Price rule {} not found", id)))?;
    Ok(Json(rule))
}

/// POST /api/price-rules - 创建价格规则
pub async fn create(
    State(state): State<ServerState>,
    Json(payload): Json<PriceRuleCreate>,
) -> AppResult<Json<PriceRule>> {
    let repo = PriceRuleRepository::new(state.db.clone());
    let rule = repo
        .create(payload)
        .await
        ?;

    // 广播同步通知 (使用完整 id 格式，与 rule.id 一致)
    let id = rule.id.as_ref().map(|t| t.to_string()).unwrap_or_default();
    state
        .broadcast_sync(RESOURCE, "created", &id, Some(&rule))
        .await;

    Ok(Json(rule))
}

/// PUT /api/price-rules/:id - 更新价格规则
pub async fn update(
    State(state): State<ServerState>,
    Path(id): Path<String>,
    Json(payload): Json<PriceRuleUpdate>,
) -> AppResult<Json<PriceRule>> {
    let repo = PriceRuleRepository::new(state.db.clone());
    let rule = repo
        .update(&id, payload)
        .await
        ?;

    // 广播同步通知
    state
        .broadcast_sync(RESOURCE, "updated", &id, Some(&rule))
        .await;

    Ok(Json(rule))
}

/// DELETE /api/price-rules/:id - 删除价格规则 (软删除)
pub async fn delete(
    State(state): State<ServerState>,
    Path(id): Path<String>,
) -> AppResult<Json<bool>> {
    let repo = PriceRuleRepository::new(state.db.clone());
    let result = repo
        .delete(&id)
        .await
        ?;

    // 广播同步通知
    if result {
        state
            .broadcast_sync::<()>(RESOURCE, "deleted", &id, None)
            .await;
    }

    Ok(Json(result))
}
