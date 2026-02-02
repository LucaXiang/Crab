//! AttributeBinding API Handlers - 产品属性绑定

use axum::{
    Json,
    extract::{Path, State},
};
use serde::{Deserialize, Serialize};
use surrealdb::RecordId;

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
    pub default_option_indices: Option<Vec<i32>>,
}

/// 更新绑定的请求体
#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateBindingRequest {
    pub is_required: Option<bool>,
    pub display_order: Option<i32>,
    pub default_option_indices: Option<Vec<i32>>,
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

    // Check if the product's category already has this attribute bound
    let product_thing: RecordId = payload.product_id
        .parse()
        .map_err(|_| AppError::validation(format!("Invalid product ID: {}", payload.product_id)))?;
    let attr_thing: RecordId = payload.attribute_id
        .parse()
        .map_err(|_| AppError::validation(format!("Invalid attribute ID: {}", payload.attribute_id)))?;

    // Get product's category
    let mut cat_result = state
        .db
        .query("SELECT category FROM product WHERE id = $prod")
        .bind(("prod", product_thing))
        .await
        .map_err(crate::db::repository::surreal_err_to_app)?;

    #[derive(Debug, serde::Deserialize)]
    struct CatRow {
        category: RecordId,
    }
    let cat_rows: Vec<CatRow> = cat_result
        .take(0)
        .map_err(crate::db::repository::surreal_err_to_app)?;

    if let Some(cat_row) = cat_rows.first() {
        // Check if category has this attribute bound
        let mut check_result = state
            .db
            .query("SELECT count() as cnt FROM has_attribute WHERE in = $cat AND out = $attr GROUP ALL")
            .bind(("cat", cat_row.category.clone()))
            .bind(("attr", attr_thing))
            .await
            .map_err(crate::db::repository::surreal_err_to_app)?;

        #[derive(Debug, serde::Deserialize)]
        struct CountRow {
            cnt: i64,
        }
        let count_rows: Vec<CountRow> = check_result
            .take(0)
            .map_err(crate::db::repository::surreal_err_to_app)?;

        if count_rows.first().map(|r| r.cnt).unwrap_or(0) > 0 {
            return Err(AppError::validation(
                "该属性已通过分类继承绑定到此产品，不能重复添加".to_string(),
            ));
        }
    }

    let binding = repo
        .link_to_product(
            &payload.product_id,
            &payload.attribute_id,
            payload.is_required,
            payload.display_order,
            payload.default_option_indices,
        )
        .await?;

    // Refresh product cache (attribute bindings changed)
    if let Err(e) = state.catalog_service.refresh_product_cache(&payload.product_id).await {
        tracing::warn!("Failed to refresh product cache for {}: {}", payload.product_id, e);
    }

    Ok(Json(binding))
}

/// GET /api/has-attribute/{id} - 获取单个绑定
pub async fn get_by_id(
    State(state): State<ServerState>,
    Path(id): Path<String>,
) -> AppResult<Json<AttributeBinding>> {
    // 通过 ID 查询 has_attribute 边
    let thing: RecordId = id
        .parse()
        .map_err(|_| AppError::validation(format!("Invalid ID: {}", id)))?;
    let mut result = state
        .db
        .query("SELECT * FROM has_attribute WHERE id = $id")
        .bind(("id", thing))
        .await
        .map_err(crate::db::repository::surreal_err_to_app)?;

    let bindings: Vec<AttributeBinding> = result
        .take(0)
        .map_err(crate::db::repository::surreal_err_to_app)?;

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
    let thing: RecordId = id
        .parse()
        .map_err(|_| AppError::validation(format!("Invalid ID: {}", id)))?;

    let mut result = state
        .db
        .query("UPDATE $thing MERGE $data RETURN AFTER")
        .bind(("thing", thing))
        .bind(("data", payload))
        .await
        .map_err(crate::db::repository::surreal_err_to_app)?;

    let bindings: Vec<AttributeBinding> = result
        .take(0)
        .map_err(crate::db::repository::surreal_err_to_app)?;

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
    let thing: RecordId = id
        .parse()
        .map_err(|_| AppError::validation(format!("Invalid ID: {}", id)))?;

    // Get the product ID before deleting (for cache refresh)
    let mut pre_result = state
        .db
        .query("SELECT in FROM has_attribute WHERE id = $id")
        .bind(("id", thing.clone()))
        .await
        .map_err(crate::db::repository::surreal_err_to_app)?;

    #[derive(Debug, serde::Deserialize)]
    struct InRow {
        #[serde(rename = "in", with = "crate::db::models::serde_helpers::record_id")]
        from: RecordId,
    }
    let in_rows: Vec<InRow> = pre_result
        .take(0)
        .map_err(crate::db::repository::surreal_err_to_app)?;
    let product_id = in_rows.first().map(|r| r.from.to_string());

    state
        .db
        .query("DELETE $thing")
        .bind(("thing", thing))
        .await
        .map_err(crate::db::repository::surreal_err_to_app)?;

    // Refresh product cache if the binding was for a product
    if let Some(pid) = product_id
        && pid.starts_with("product:")
            && let Err(e) = state.catalog_service.refresh_product_cache(&pid).await {
                tracing::warn!("Failed to refresh product cache for {}: {}", pid, e);
            }

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
        .await?;

    let result: Vec<BindingWithAttribute> = bindings
        .into_iter()
        .map(|(binding, attribute)| BindingWithAttribute { binding, attribute })
        .collect();

    Ok(Json(result))
}
