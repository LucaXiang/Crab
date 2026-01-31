//! Price Rule API Handlers

use axum::{
    Json,
    extract::{Extension, Path, State},
};

use crate::audit::AuditAction;
use crate::audit_log;
use crate::auth::CurrentUser;
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
    Extension(current_user): Extension<CurrentUser>,
    Json(payload): Json<PriceRuleCreate>,
) -> AppResult<Json<PriceRule>> {
    let repo = PriceRuleRepository::new(state.db.clone());
    let rule = repo
        .create(payload)
        .await
        ?;

    let id = rule.id.as_ref().map(|t| t.to_string()).unwrap_or_default();

    audit_log!(
        state.audit_service,
        AuditAction::PriceRuleCreated,
        "price_rule", &id,
        operator_id = Some(current_user.id.clone()),
        operator_name = Some(current_user.display_name.clone()),
        details = serde_json::json!({"name": &rule.name})
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
    Path(id): Path<String>,
    Json(payload): Json<PriceRuleUpdate>,
) -> AppResult<Json<PriceRule>> {
    let repo = PriceRuleRepository::new(state.db.clone());
    let rule = repo
        .update(&id, payload)
        .await
        ?;

    audit_log!(
        state.audit_service,
        AuditAction::PriceRuleUpdated,
        "price_rule", &id,
        operator_id = Some(current_user.id.clone()),
        operator_name = Some(current_user.display_name.clone()),
        details = serde_json::json!({"name": &rule.name})
    );

    state
        .broadcast_sync(RESOURCE, "updated", &id, Some(&rule))
        .await;

    Ok(Json(rule))
}

/// DELETE /api/price-rules/:id - 删除价格规则 (软删除)
pub async fn delete(
    State(state): State<ServerState>,
    Extension(current_user): Extension<CurrentUser>,
    Path(id): Path<String>,
) -> AppResult<Json<bool>> {
    let repo = PriceRuleRepository::new(state.db.clone());
    let name_for_audit = repo.find_by_id(&id).await.ok().flatten()
        .map(|r| r.name.clone()).unwrap_or_default();
    let result = repo
        .delete(&id)
        .await
        ?;

    if result {
        audit_log!(
            state.audit_service,
            AuditAction::PriceRuleDeleted,
            "price_rule", &id,
            operator_id = Some(current_user.id.clone()),
            operator_name = Some(current_user.display_name.clone()),
            details = serde_json::json!({"name": name_for_audit})
        );

        state
            .broadcast_sync::<()>(RESOURCE, "deleted", &id, None)
            .await;
    }

    Ok(Json(result))
}
