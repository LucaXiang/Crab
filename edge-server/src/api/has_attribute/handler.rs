//! AttributeBinding API Handlers - 产品属性绑定

use axum::{
    Json,
    extract::{Path, State},
};
use serde::{Deserialize, Serialize};

use crate::core::ServerState;
use crate::db::models::{Attribute, AttributeBinding};
use crate::db::repository::AttributeRepository;
use crate::utils::{AppError, AppResult};

/// 创建绑定的请求体
#[derive(Debug, Deserialize)]
pub struct CreateBindingRequest {
    pub product_id: String,
    pub attribute_id: String,
    #[serde(default)]
    pub is_required: bool,
    #[serde(default)]
    pub display_order: i32,
    pub default_option_idx: Option<i32>,
}

/// 更新绑定的请求体
#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateBindingRequest {
    pub is_required: Option<bool>,
    pub display_order: Option<i32>,
    pub default_option_idx: Option<i32>,
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
    Json(payload): Json<CreateBindingRequest>,
) -> AppResult<Json<AttributeBinding>> {
    let repo = AttributeRepository::new(state.db.clone());

    let binding = repo
        .link_to_product(
            &payload.product_id,
            &payload.attribute_id,
            payload.is_required,
            payload.display_order,
            payload.default_option_idx,
        )
        .await
        .map_err(|e| AppError::database(e.to_string()))?;

    Ok(Json(binding))
}

/// GET /api/has-attribute/{id} - 获取单个绑定
pub async fn get_by_id(
    State(state): State<ServerState>,
    Path(id): Path<String>,
) -> AppResult<Json<AttributeBinding>> {
    // 通过 ID 查询 has_attribute 边
    let mut result = state
        .db
        .query("SELECT * FROM has_attribute WHERE id = $id")
        .bind((
            "id",
            crate::db::repository::make_thing("has_attribute", &id),
        ))
        .await
        .map_err(|e| AppError::database(e.to_string()))?;

    let bindings: Vec<AttributeBinding> = result
        .take(0)
        .map_err(|e| AppError::database(e.to_string()))?;

    bindings
        .into_iter()
        .next()
        .map(Json)
        .ok_or_else(|| AppError::not_found(format!("Binding {} not found", id)))
}

/// PUT /api/has-attribute/{id} - 更新绑定
pub async fn update(
    State(state): State<ServerState>,
    Path(id): Path<String>,
    Json(payload): Json<UpdateBindingRequest>,
) -> AppResult<Json<AttributeBinding>> {
    let thing = crate::db::repository::make_thing("has_attribute", &id);

    let mut result = state
        .db
        .query("UPDATE $thing MERGE $data RETURN AFTER")
        .bind(("thing", thing))
        .bind(("data", payload))
        .await
        .map_err(|e| AppError::database(e.to_string()))?;

    let bindings: Vec<AttributeBinding> = result
        .take(0)
        .map_err(|e| AppError::database(e.to_string()))?;

    bindings
        .into_iter()
        .next()
        .map(Json)
        .ok_or_else(|| AppError::not_found(format!("Binding {} not found", id)))
}

/// DELETE /api/has-attribute/{id} - 删除绑定
pub async fn delete(
    State(state): State<ServerState>,
    Path(id): Path<String>,
) -> AppResult<Json<bool>> {
    let thing = crate::db::repository::make_thing("has_attribute", &id);

    state
        .db
        .query("DELETE $thing")
        .bind(("thing", thing))
        .await
        .map_err(|e| AppError::database(e.to_string()))?;

    Ok(Json(true))
}

/// GET /api/has-attribute/product/{product_id} - 获取产品的所有属性绑定
pub async fn list_by_product(
    State(state): State<ServerState>,
    Path(product_id): Path<String>,
) -> AppResult<Json<Vec<BindingWithAttribute>>> {
    let repo = AttributeRepository::new(state.db.clone());

    let bindings = repo
        .find_bindings_for_product(&product_id)
        .await
        .map_err(|e| AppError::database(e.to_string()))?;

    let result: Vec<BindingWithAttribute> = bindings
        .into_iter()
        .map(|(binding, attribute)| BindingWithAttribute { binding, attribute })
        .collect();

    Ok(Json(result))
}
