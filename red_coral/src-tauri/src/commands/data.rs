//! 数据 API Commands
//!
//! 通过 ClientBridge -> CrabClient -> EdgeServer REST API
//! 所有响应使用 ApiResponse<T> 格式包装

use std::sync::Arc;
use tauri::State;

use crate::core::{
    ApiResponse, ClientBridge, DeleteData,
};
use shared::models::{
    // Attributes
    Attribute,
    AttributeBinding,
    AttributeBindingFull,
    AttributeCreate,
    AttributeOptionInput,
    AttributeUpdate,
    // Categories
    Category,
    CategoryCreate,
    CategoryUpdate,
    // Print Destinations
    PrintDestination,
    PrintDestinationCreate,
    PrintDestinationUpdate,
    // Products
    ProductCreate,
    ProductFull,
    ProductUpdate,
    // Tags
    Tag,
    TagCreate,
    TagUpdate,
};

// ============ Tags ============

#[tauri::command]
pub async fn list_tags(
    bridge: State<'_, Arc<ClientBridge>>,
) -> Result<ApiResponse<Vec<Tag>>, String> {
    match bridge.get::<Vec<Tag>>("/api/tags").await {
        Ok(tags) => Ok(ApiResponse::success(tags)),
        Err(e) => Ok(ApiResponse::from_bridge_error(e)),
    }
}

#[tauri::command]
pub async fn get_tag(
    bridge: State<'_, Arc<ClientBridge>>,
    id: i64,
) -> Result<ApiResponse<Tag>, String> {
    match bridge
        .get::<Tag>(&format!("/api/tags/{}", id))
        .await
    {
        Ok(tag) => Ok(ApiResponse::success(tag)),
        Err(e) => Ok(ApiResponse::from_bridge_error(e)),
    }
}

#[tauri::command]
pub async fn create_tag(
    bridge: State<'_, Arc<ClientBridge>>,
    data: TagCreate,
) -> Result<ApiResponse<Tag>, String> {
    match bridge.post::<Tag, _>("/api/tags", &data).await {
        Ok(tag) => Ok(ApiResponse::success(tag)),
        Err(e) => Ok(ApiResponse::from_bridge_error(e)),
    }
}

#[tauri::command]
pub async fn update_tag(
    bridge: State<'_, Arc<ClientBridge>>,
    id: i64,
    data: TagUpdate,
) -> Result<ApiResponse<Tag>, String> {
    match bridge
        .put::<Tag, _>(&format!("/api/tags/{}", id), &data)
        .await
    {
        Ok(tag) => Ok(ApiResponse::success(tag)),
        Err(e) => Ok(ApiResponse::from_bridge_error(e)),
    }
}

#[tauri::command]
pub async fn delete_tag(
    bridge: State<'_, Arc<ClientBridge>>,
    id: i64,
) -> Result<ApiResponse<DeleteData>, String> {
    match bridge
        .delete::<bool>(&format!("/api/tags/{}", id))
        .await
    {
        Ok(deleted) => Ok(ApiResponse::success(DeleteData { deleted })),
        Err(e) => Ok(ApiResponse::from_bridge_error(e)),
    }
}

// ============ Categories ============

#[tauri::command]
pub async fn list_categories(
    bridge: State<'_, Arc<ClientBridge>>,
) -> Result<ApiResponse<Vec<Category>>, String> {
    match bridge.get::<Vec<Category>>("/api/categories").await {
        Ok(categories) => Ok(ApiResponse::success(categories)),
        Err(e) => Ok(ApiResponse::from_bridge_error(e)),
    }
}

#[tauri::command]
pub async fn get_category(
    bridge: State<'_, Arc<ClientBridge>>,
    id: i64,
) -> Result<ApiResponse<Category>, String> {
    match bridge
        .get::<Category>(&format!("/api/categories/{}", id))
        .await
    {
        Ok(cat) => Ok(ApiResponse::success(cat)),
        Err(e) => Ok(ApiResponse::from_bridge_error(e)),
    }
}

#[tauri::command]
pub async fn create_category(
    bridge: State<'_, Arc<ClientBridge>>,
    data: CategoryCreate,
) -> Result<ApiResponse<Category>, String> {
    match bridge.post::<Category, _>("/api/categories", &data).await {
        Ok(cat) => Ok(ApiResponse::success(cat)),
        Err(e) => Ok(ApiResponse::from_bridge_error(e)),
    }
}

#[tauri::command]
pub async fn update_category(
    bridge: State<'_, Arc<ClientBridge>>,
    id: i64,
    data: CategoryUpdate,
) -> Result<ApiResponse<Category>, String> {
    match bridge
        .put::<Category, _>(&format!("/api/categories/{}", id), &data)
        .await
    {
        Ok(cat) => Ok(ApiResponse::success(cat)),
        Err(e) => Ok(ApiResponse::from_bridge_error(e)),
    }
}

#[tauri::command]
pub async fn delete_category(
    bridge: State<'_, Arc<ClientBridge>>,
    id: i64,
) -> Result<ApiResponse<DeleteData>, String> {
    match bridge
        .delete::<bool>(&format!("/api/categories/{}", id))
        .await
    {
        Ok(deleted) => Ok(ApiResponse::success(DeleteData { deleted })),
        Err(e) => Ok(ApiResponse::from_bridge_error(e)),
    }
}

// ============ Products ============

#[tauri::command]
pub async fn list_products(
    bridge: State<'_, Arc<ClientBridge>>,
) -> Result<ApiResponse<Vec<ProductFull>>, String> {
    match bridge.get::<Vec<ProductFull>>("/api/products").await {
        Ok(products) => Ok(ApiResponse::success(products)),
        Err(e) => Ok(ApiResponse::from_bridge_error(e)),
    }
}

#[tauri::command]
pub async fn get_product(
    bridge: State<'_, Arc<ClientBridge>>,
    id: i64,
) -> Result<ApiResponse<ProductFull>, String> {
    match bridge
        .get::<ProductFull>(&format!("/api/products/{}", id))
        .await
    {
        Ok(prod) => Ok(ApiResponse::success(prod)),
        Err(e) => Ok(ApiResponse::from_bridge_error(e)),
    }
}

/// 获取商品完整信息 (含规格、属性、标签)
#[tauri::command]
pub async fn get_product_full(
    bridge: State<'_, Arc<ClientBridge>>,
    id: i64,
) -> Result<ApiResponse<ProductFull>, String> {
    match bridge
        .get::<ProductFull>(&format!("/api/products/{}", id))
        .await
    {
        Ok(product) => Ok(ApiResponse::success(product)),
        Err(e) => Ok(ApiResponse::from_bridge_error(e)),
    }
}

#[tauri::command]
pub async fn create_product(
    bridge: State<'_, Arc<ClientBridge>>,
    data: ProductCreate,
) -> Result<ApiResponse<ProductFull>, String> {
    match bridge.post::<ProductFull, _>("/api/products", &data).await {
        Ok(prod) => Ok(ApiResponse::success(prod)),
        Err(e) => Ok(ApiResponse::from_bridge_error(e)),
    }
}

#[tauri::command]
pub async fn update_product(
    bridge: State<'_, Arc<ClientBridge>>,
    id: i64,
    data: ProductUpdate,
) -> Result<ApiResponse<ProductFull>, String> {
    match bridge
        .put::<ProductFull, _>(&format!("/api/products/{}", id), &data)
        .await
    {
        Ok(prod) => Ok(ApiResponse::success(prod)),
        Err(e) => Ok(ApiResponse::from_bridge_error(e)),
    }
}

#[tauri::command]
pub async fn delete_product(
    bridge: State<'_, Arc<ClientBridge>>,
    id: i64,
) -> Result<ApiResponse<DeleteData>, String> {
    match bridge
        .delete::<bool>(&format!("/api/products/{}", id))
        .await
    {
        Ok(deleted) => Ok(ApiResponse::success(DeleteData { deleted })),
        Err(e) => Ok(ApiResponse::from_bridge_error(e)),
    }
}

// ============ Attributes ============

#[tauri::command]
pub async fn list_attributes(
    bridge: State<'_, Arc<ClientBridge>>,
) -> Result<ApiResponse<Vec<Attribute>>, String> {
    match bridge.get::<Vec<Attribute>>("/api/attributes").await {
        Ok(templates) => Ok(ApiResponse::success(templates)),
        Err(e) => Ok(ApiResponse::from_bridge_error(e)),
    }
}

#[tauri::command]
pub async fn get_attribute(
    bridge: State<'_, Arc<ClientBridge>>,
    id: i64,
) -> Result<ApiResponse<Attribute>, String> {
    match bridge
        .get::<Attribute>(&format!("/api/attributes/{}", id))
        .await
    {
        Ok(template) => Ok(ApiResponse::success(template)),
        Err(e) => Ok(ApiResponse::from_bridge_error(e)),
    }
}

#[tauri::command]
pub async fn create_attribute(
    bridge: State<'_, Arc<ClientBridge>>,
    data: AttributeCreate,
) -> Result<ApiResponse<Attribute>, String> {
    match bridge.post::<Attribute, _>("/api/attributes", &data).await {
        Ok(template) => Ok(ApiResponse::success(template)),
        Err(e) => Ok(ApiResponse::from_bridge_error(e)),
    }
}

#[tauri::command]
pub async fn update_attribute(
    bridge: State<'_, Arc<ClientBridge>>,
    id: i64,
    data: AttributeUpdate,
) -> Result<ApiResponse<Attribute>, String> {
    match bridge
        .put::<Attribute, _>(&format!("/api/attributes/{}", id), &data)
        .await
    {
        Ok(template) => Ok(ApiResponse::success(template)),
        Err(e) => Ok(ApiResponse::from_bridge_error(e)),
    }
}

#[tauri::command]
pub async fn delete_attribute(
    bridge: State<'_, Arc<ClientBridge>>,
    id: i64,
) -> Result<ApiResponse<DeleteData>, String> {
    match bridge
        .delete::<bool>(&format!("/api/attributes/{}", id))
        .await
    {
        Ok(deleted) => Ok(ApiResponse::success(DeleteData { deleted })),
        Err(e) => Ok(ApiResponse::from_bridge_error(e)),
    }
}

// ============ Attribute Options ============

#[tauri::command]
pub async fn add_attribute_option(
    bridge: State<'_, Arc<ClientBridge>>,
    attribute_id: i64,
    data: AttributeOptionInput,
) -> Result<ApiResponse<Attribute>, String> {
    match bridge
        .post::<Attribute, _>(
            &format!("/api/attributes/{}/options", attribute_id),
            &data,
        )
        .await
    {
        Ok(template) => Ok(ApiResponse::success(template)),
        Err(e) => Ok(ApiResponse::from_bridge_error(e)),
    }
}

#[tauri::command]
pub async fn update_attribute_option(
    bridge: State<'_, Arc<ClientBridge>>,
    attribute_id: i64,
    index: usize,
    data: AttributeOptionInput,
) -> Result<ApiResponse<Attribute>, String> {
    match bridge
        .put::<Attribute, _>(
            &format!(
                "/api/attributes/{}/options/{}",
                attribute_id,
                index
            ),
            &data,
        )
        .await
    {
        Ok(template) => Ok(ApiResponse::success(template)),
        Err(e) => Ok(ApiResponse::from_bridge_error(e)),
    }
}

#[tauri::command]
pub async fn delete_attribute_option(
    bridge: State<'_, Arc<ClientBridge>>,
    attribute_id: i64,
    index: usize,
) -> Result<ApiResponse<Attribute>, String> {
    match bridge
        .delete::<Attribute>(&format!(
            "/api/attributes/{}/options/{}",
            attribute_id,
            index
        ))
        .await
    {
        Ok(template) => Ok(ApiResponse::success(template)),
        Err(e) => Ok(ApiResponse::from_bridge_error(e)),
    }
}

// ============ Product Attributes (Bindings) ============

#[tauri::command]
pub async fn list_product_attributes(
    bridge: State<'_, Arc<ClientBridge>>,
    product_id: i64,
) -> Result<ApiResponse<Vec<AttributeBindingFull>>, String> {
    match bridge
        .get::<Vec<AttributeBindingFull>>(&format!("/api/products/{}/attributes", product_id))
        .await
    {
        Ok(attrs) => Ok(ApiResponse::success(attrs)),
        Err(e) => Ok(ApiResponse::from_bridge_error(e)),
    }
}

#[tauri::command]
pub async fn bind_product_attribute(
    bridge: State<'_, Arc<ClientBridge>>,
    data: serde_json::Value,
) -> Result<ApiResponse<AttributeBinding>, String> {
    match bridge
        .post::<AttributeBinding, _>("/api/has-attribute", &data)
        .await
    {
        Ok(result) => Ok(ApiResponse::success(result)),
        Err(e) => Ok(ApiResponse::from_bridge_error(e)),
    }
}

#[tauri::command]
pub async fn unbind_product_attribute(
    bridge: State<'_, Arc<ClientBridge>>,
    id: i64,
) -> Result<ApiResponse<DeleteData>, String> {
    match bridge
        .delete::<bool>(&format!("/api/has-attribute/{}", id))
        .await
    {
        Ok(deleted) => Ok(ApiResponse::success(DeleteData { deleted })),
        Err(e) => Ok(ApiResponse::from_bridge_error(e)),
    }
}

// ============ Category Attributes (Bindings) ============

/// List attributes for a category
#[tauri::command]
pub async fn list_category_attributes(
    bridge: State<'_, Arc<ClientBridge>>,
    category_id: i64,
) -> Result<ApiResponse<Vec<Attribute>>, String> {
    match bridge
        .get::<Vec<Attribute>>(&format!("/api/categories/{}/attributes", category_id))
        .await
    {
        Ok(templates) => Ok(ApiResponse::success(templates)),
        Err(e) => Ok(ApiResponse::from_bridge_error(e)),
    }
}

/// Payload for binding attribute to category
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct BindCategoryAttributeData {
    pub category_id: i64,
    pub attribute_id: i64,
    pub is_required: Option<bool>,
    pub display_order: Option<i32>,
    pub default_option_ids: Option<Vec<i32>>,
}

/// Bind attribute to category
#[tauri::command]
pub async fn bind_category_attribute(
    bridge: State<'_, Arc<ClientBridge>>,
    data: BindCategoryAttributeData,
) -> Result<ApiResponse<AttributeBinding>, String> {
    // Build payload for API
    let payload = serde_json::json!({
        "is_required": data.is_required,
        "display_order": data.display_order,
        "default_option_ids": data.default_option_ids,
    });
    match bridge
        .post::<AttributeBinding, _>(
            &format!(
                "/api/categories/{}/attributes/{}",
                data.category_id, data.attribute_id
            ),
            &payload,
        )
        .await
    {
        Ok(binding) => Ok(ApiResponse::success(binding)),
        Err(e) => Ok(ApiResponse::from_bridge_error(e)),
    }
}

/// Unbind attribute from category
#[tauri::command]
pub async fn unbind_category_attribute(
    bridge: State<'_, Arc<ClientBridge>>,
    category_id: i64,
    attribute_id: i64,
) -> Result<ApiResponse<DeleteData>, String> {
    match bridge
        .delete::<bool>(&format!(
            "/api/categories/{}/attributes/{}",
            category_id, attribute_id
        ))
        .await
    {
        Ok(deleted) => Ok(ApiResponse::success(DeleteData { deleted })),
        Err(e) => Ok(ApiResponse::from_bridge_error(e)),
    }
}

/// Payload for batch sort order update
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct CategorySortOrderUpdate {
    pub id: i64,
    pub sort_order: i32,
}

/// Response for batch update operation
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct BatchUpdateResponse {
    pub updated: usize,
}

/// Batch update category sort order
#[tauri::command]
pub async fn batch_update_category_sort_order(
    bridge: State<'_, Arc<ClientBridge>>,
    updates: Vec<CategorySortOrderUpdate>,
) -> Result<ApiResponse<BatchUpdateResponse>, String> {
    match bridge
        .put::<BatchUpdateResponse, _>("/api/categories/sort-order", &updates)
        .await
    {
        Ok(result) => Ok(ApiResponse::success(result)),
        Err(e) => Ok(ApiResponse::from_bridge_error(e)),
    }
}

/// Payload for batch product sort order update
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ProductSortOrderUpdate {
    pub id: i64,
    pub sort_order: i32,
}

/// Batch update product sort order
#[tauri::command]
pub async fn batch_update_product_sort_order(
    bridge: State<'_, Arc<ClientBridge>>,
    updates: Vec<ProductSortOrderUpdate>,
) -> Result<ApiResponse<BatchUpdateResponse>, String> {
    match bridge
        .put::<BatchUpdateResponse, _>("/api/products/sort-order", &updates)
        .await
    {
        Ok(result) => Ok(ApiResponse::success(result)),
        Err(e) => Ok(ApiResponse::from_bridge_error(e)),
    }
}

// ============ Print Destinations ============

#[tauri::command]
pub async fn list_print_destinations(
    bridge: State<'_, Arc<ClientBridge>>,
) -> Result<ApiResponse<Vec<PrintDestination>>, String> {
    match bridge
        .get::<Vec<PrintDestination>>("/api/print-destinations")
        .await
    {
        Ok(print_destinations) => Ok(ApiResponse::success(print_destinations)),
        Err(e) => Ok(ApiResponse::from_bridge_error(e)),
    }
}

#[tauri::command]
pub async fn get_print_destination(
    bridge: State<'_, Arc<ClientBridge>>,
    id: i64,
) -> Result<ApiResponse<PrintDestination>, String> {
    match bridge
        .get::<PrintDestination>(&format!("/api/print-destinations/{}", id))
        .await
    {
        Ok(print_destination) => Ok(ApiResponse::success(print_destination)),
        Err(e) => Ok(ApiResponse::from_bridge_error(e)),
    }
}

#[tauri::command]
pub async fn create_print_destination(
    bridge: State<'_, Arc<ClientBridge>>,
    data: PrintDestinationCreate,
) -> Result<ApiResponse<PrintDestination>, String> {
    match bridge
        .post::<PrintDestination, _>("/api/print-destinations", &data)
        .await
    {
        Ok(print_destination) => Ok(ApiResponse::success(print_destination)),
        Err(e) => Ok(ApiResponse::from_bridge_error(e)),
    }
}

#[tauri::command]
pub async fn update_print_destination(
    bridge: State<'_, Arc<ClientBridge>>,
    id: i64,
    data: PrintDestinationUpdate,
) -> Result<ApiResponse<PrintDestination>, String> {
    match bridge
        .put::<PrintDestination, _>(&format!("/api/print-destinations/{}", id), &data)
        .await
    {
        Ok(print_destination) => Ok(ApiResponse::success(print_destination)),
        Err(e) => Ok(ApiResponse::from_bridge_error(e)),
    }
}

#[tauri::command]
pub async fn delete_print_destination(
    bridge: State<'_, Arc<ClientBridge>>,
    id: i64,
) -> Result<ApiResponse<DeleteData>, String> {
    match bridge
        .delete::<bool>(&format!("/api/print-destinations/{}", id))
        .await
    {
        Ok(deleted) => Ok(ApiResponse::success(DeleteData { deleted })),
        Err(e) => Ok(ApiResponse::from_bridge_error(e)),
    }
}
