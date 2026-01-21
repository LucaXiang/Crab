//! 数据 API Commands
//!
//! 通过 ClientBridge -> CrabClient -> EdgeServer REST API
//! 所有响应使用 ApiResponse<T> 格式包装

use std::sync::Arc;
use tauri::State;
use tokio::sync::RwLock;

use crate::core::response::ErrorCode;
use crate::core::{
    ApiResponse, AttributeData, AttributeListData, CategoryData, CategoryListData, ClientBridge,
    DeleteData, PrinterData, PrinterListData, ProductData, ProductFullData, ProductListData,
    TagListData,
};
use shared::models::{
    // Attributes
    Attribute,
    AttributeCreate,
    AttributeOption,
    AttributeUpdate,
    // Categories
    Category,
    CategoryCreate,
    CategoryUpdate,
    HasAttribute,
    // Kitchen Printers
    KitchenPrinter,
    KitchenPrinterCreate,
    KitchenPrinterUpdate,
    // Products
    Product,
    ProductCreate,
    ProductFull,
    ProductUpdate,
    // Tags
    Tag,
    TagCreate,
    TagUpdate,
};
use urlencoding::encode;

// ============ Tags ============

#[tauri::command(rename_all = "snake_case")]
pub async fn list_tags(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
) -> Result<ApiResponse<TagListData>, String> {
    let bridge = bridge.read().await;
    match bridge.get::<Vec<Tag>>("/api/tags").await {
        Ok(tags) => Ok(ApiResponse::success(TagListData { tags })),
        Err(e) => Ok(ApiResponse::error_with_code(ErrorCode::DatabaseError, e.to_string())),
    }
}

#[tauri::command(rename_all = "snake_case")]
pub async fn get_tag(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    id: String,
) -> Result<ApiResponse<Tag>, String> {
    let bridge = bridge.read().await;
    match bridge
        .get::<Tag>(&format!("/api/tags/{}", encode(&id)))
        .await
    {
        Ok(tag) => Ok(ApiResponse::success(tag)),
        Err(e) => Ok(ApiResponse::error_with_code(ErrorCode::NotFound, e.to_string())),
    }
}

#[tauri::command(rename_all = "snake_case")]
pub async fn create_tag(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    data: TagCreate,
) -> Result<ApiResponse<Tag>, String> {
    let bridge = bridge.read().await;
    match bridge.post::<Tag, _>("/api/tags", &data).await {
        Ok(tag) => Ok(ApiResponse::success(tag)),
        Err(e) => Ok(ApiResponse::error_with_code(ErrorCode::DatabaseError, e.to_string())),
    }
}

#[tauri::command(rename_all = "snake_case")]
pub async fn update_tag(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    id: String,
    data: TagUpdate,
) -> Result<ApiResponse<Tag>, String> {
    let bridge = bridge.read().await;
    match bridge
        .put::<Tag, _>(&format!("/api/tags/{}", encode(&id)), &data)
        .await
    {
        Ok(tag) => Ok(ApiResponse::success(tag)),
        Err(e) => Ok(ApiResponse::error_with_code(ErrorCode::DatabaseError, e.to_string())),
    }
}

#[tauri::command(rename_all = "snake_case")]
pub async fn delete_tag(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    id: String,
) -> Result<ApiResponse<DeleteData>, String> {
    let bridge = bridge.read().await;
    match bridge
        .delete::<bool>(&format!("/api/tags/{}", encode(&id)))
        .await
    {
        Ok(deleted) => Ok(ApiResponse::success(DeleteData { deleted })),
        Err(e) => Ok(ApiResponse::error_with_code(ErrorCode::DatabaseError, e.to_string())),
    }
}

// ============ Categories ============

#[tauri::command(rename_all = "snake_case")]
pub async fn list_categories(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
) -> Result<ApiResponse<CategoryListData>, String> {
    let bridge = bridge.read().await;
    match bridge.get::<Vec<Category>>("/api/categories").await {
        Ok(categories) => Ok(ApiResponse::success(CategoryListData { categories })),
        Err(e) => Ok(ApiResponse::error_with_code(ErrorCode::DatabaseError, e.to_string())),
    }
}

#[tauri::command(rename_all = "snake_case")]
pub async fn get_category(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    id: String,
) -> Result<ApiResponse<CategoryData>, String> {
    let bridge = bridge.read().await;
    match bridge
        .get::<Category>(&format!("/api/categories/{}", encode(&id)))
        .await
    {
        Ok(cat) => Ok(ApiResponse::success(CategoryData { category: cat })),
        Err(e) => Ok(ApiResponse::error_with_code(ErrorCode::CategoryNotFound, e.to_string())),
    }
}

#[tauri::command(rename_all = "snake_case")]
pub async fn create_category(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    data: CategoryCreate,
) -> Result<ApiResponse<CategoryData>, String> {
    let bridge = bridge.read().await;
    match bridge.post::<Category, _>("/api/categories", &data).await {
        Ok(cat) => Ok(ApiResponse::success(CategoryData { category: cat })),
        Err(e) => Ok(ApiResponse::error_with_code(ErrorCode::DatabaseError, e.to_string())),
    }
}

#[tauri::command(rename_all = "snake_case")]
pub async fn update_category(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    id: String,
    data: CategoryUpdate,
) -> Result<ApiResponse<CategoryData>, String> {
    let bridge = bridge.read().await;
    match bridge
        .put::<Category, _>(&format!("/api/categories/{}", encode(&id)), &data)
        .await
    {
        Ok(cat) => Ok(ApiResponse::success(CategoryData { category: cat })),
        Err(e) => Ok(ApiResponse::error_with_code(ErrorCode::DatabaseError, e.to_string())),
    }
}

#[tauri::command(rename_all = "snake_case")]
pub async fn delete_category(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    id: String,
) -> Result<ApiResponse<DeleteData>, String> {
    let bridge = bridge.read().await;
    match bridge
        .delete::<bool>(&format!("/api/categories/{}", encode(&id)))
        .await
    {
        Ok(deleted) => Ok(ApiResponse::success(DeleteData { deleted })),
        Err(e) => Ok(ApiResponse::error_with_code(ErrorCode::DatabaseError, e.to_string())),
    }
}

// ============ Products ============

#[tauri::command(rename_all = "snake_case")]
pub async fn list_products(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
) -> Result<ApiResponse<ProductListData>, String> {
    let bridge = bridge.read().await;
    match bridge.get::<Vec<Product>>("/api/products").await {
        Ok(products) => Ok(ApiResponse::success(ProductListData { products })),
        Err(e) => Ok(ApiResponse::error_with_code(ErrorCode::DatabaseError, e.to_string())),
    }
}

#[tauri::command(rename_all = "snake_case")]
pub async fn get_product(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    id: String,
) -> Result<ApiResponse<ProductData>, String> {
    let bridge = bridge.read().await;
    match bridge
        .get::<Product>(&format!("/api/products/{}", encode(&id)))
        .await
    {
        Ok(prod) => Ok(ApiResponse::success(ProductData { product: prod })),
        Err(e) => Ok(ApiResponse::error_with_code(ErrorCode::ProductNotFound, e.to_string())),
    }
}

/// 获取商品完整信息 (含规格、属性、标签)
#[tauri::command(rename_all = "snake_case")]
pub async fn get_product_full(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    id: String,
) -> Result<ApiResponse<ProductFullData>, String> {
    let bridge = bridge.read().await;
    match bridge
        .get::<ProductFull>(&format!("/api/products/{}/full", encode(&id)))
        .await
    {
        Ok(product) => Ok(ApiResponse::success(ProductFullData { product })),
        Err(e) => Ok(ApiResponse::error_with_code(ErrorCode::ProductNotFound, e.to_string())),
    }
}

#[tauri::command(rename_all = "snake_case")]
pub async fn create_product(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    data: ProductCreate,
) -> Result<ApiResponse<ProductData>, String> {
    let bridge = bridge.read().await;
    match bridge.post::<Product, _>("/api/products", &data).await {
        Ok(prod) => Ok(ApiResponse::success(ProductData { product: prod })),
        Err(e) => Ok(ApiResponse::error_with_code(ErrorCode::DatabaseError, e.to_string())),
    }
}

#[tauri::command(rename_all = "snake_case")]
pub async fn update_product(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    id: String,
    data: ProductUpdate,
) -> Result<ApiResponse<ProductData>, String> {
    let bridge = bridge.read().await;
    match bridge
        .put::<Product, _>(&format!("/api/products/{}", encode(&id)), &data)
        .await
    {
        Ok(prod) => Ok(ApiResponse::success(ProductData { product: prod })),
        Err(e) => Ok(ApiResponse::error_with_code(ErrorCode::DatabaseError, e.to_string())),
    }
}

#[tauri::command(rename_all = "snake_case")]
pub async fn delete_product(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    id: String,
) -> Result<ApiResponse<DeleteData>, String> {
    let bridge = bridge.read().await;
    match bridge
        .delete::<bool>(&format!("/api/products/{}", encode(&id)))
        .await
    {
        Ok(deleted) => Ok(ApiResponse::success(DeleteData { deleted })),
        Err(e) => Ok(ApiResponse::error_with_code(ErrorCode::DatabaseError, e.to_string())),
    }
}

// ============ Attributes ============

#[tauri::command(rename_all = "snake_case")]
pub async fn list_attributes(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
) -> Result<ApiResponse<AttributeListData>, String> {
    let bridge = bridge.read().await;
    match bridge.get::<Vec<Attribute>>("/api/attributes").await {
        Ok(templates) => Ok(ApiResponse::success(AttributeListData { templates })),
        Err(e) => Ok(ApiResponse::error_with_code(ErrorCode::DatabaseError, e.to_string())),
    }
}

#[tauri::command(rename_all = "snake_case")]
pub async fn get_attribute(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    id: String,
) -> Result<ApiResponse<AttributeData>, String> {
    let bridge = bridge.read().await;
    match bridge
        .get::<Attribute>(&format!("/api/attributes/{}", encode(&id)))
        .await
    {
        Ok(template) => Ok(ApiResponse::success(AttributeData { template })),
        Err(e) => Ok(ApiResponse::error_with_code(ErrorCode::AttributeNotFound, e.to_string())),
    }
}

#[tauri::command(rename_all = "snake_case")]
pub async fn create_attribute(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    data: AttributeCreate,
) -> Result<ApiResponse<AttributeData>, String> {
    let bridge = bridge.read().await;
    match bridge.post::<Attribute, _>("/api/attributes", &data).await {
        Ok(template) => Ok(ApiResponse::success(AttributeData { template })),
        Err(e) => Ok(ApiResponse::error_with_code(ErrorCode::DatabaseError, e.to_string())),
    }
}

#[tauri::command(rename_all = "snake_case")]
pub async fn update_attribute(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    id: String,
    data: AttributeUpdate,
) -> Result<ApiResponse<AttributeData>, String> {
    let bridge = bridge.read().await;
    match bridge
        .put::<Attribute, _>(&format!("/api/attributes/{}", encode(&id)), &data)
        .await
    {
        Ok(template) => Ok(ApiResponse::success(AttributeData { template })),
        Err(e) => Ok(ApiResponse::error_with_code(ErrorCode::DatabaseError, e.to_string())),
    }
}

#[tauri::command(rename_all = "snake_case")]
pub async fn delete_attribute(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    id: String,
) -> Result<ApiResponse<DeleteData>, String> {
    let bridge = bridge.read().await;
    match bridge
        .delete::<bool>(&format!("/api/attributes/{}", encode(&id)))
        .await
    {
        Ok(deleted) => Ok(ApiResponse::success(DeleteData { deleted })),
        Err(e) => Ok(ApiResponse::error_with_code(ErrorCode::DatabaseError, e.to_string())),
    }
}

// ============ Attribute Options ============

#[tauri::command(rename_all = "snake_case")]
pub async fn add_attribute_option(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    attribute_id: String,
    data: AttributeOption,
) -> Result<ApiResponse<AttributeData>, String> {
    let bridge = bridge.read().await;
    match bridge
        .post::<Attribute, _>(&format!("/api/attributes/{}/options", encode(&attribute_id)), &data)
        .await
    {
        Ok(template) => Ok(ApiResponse::success(AttributeData { template })),
        Err(e) => Ok(ApiResponse::error_with_code(ErrorCode::DatabaseError, e.to_string())),
    }
}

#[tauri::command(rename_all = "snake_case")]
pub async fn update_attribute_option(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    attribute_id: String,
    index: usize,
    data: AttributeOption,
) -> Result<ApiResponse<AttributeData>, String> {
    let bridge = bridge.read().await;
    match bridge
        .put::<Attribute, _>(&format!("/api/attributes/{}/options/{}", encode(&attribute_id), index), &data)
        .await
    {
        Ok(template) => Ok(ApiResponse::success(AttributeData { template })),
        Err(e) => Ok(ApiResponse::error_with_code(ErrorCode::DatabaseError, e.to_string())),
    }
}

#[tauri::command(rename_all = "snake_case")]
pub async fn delete_attribute_option(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    attribute_id: String,
    index: usize,
) -> Result<ApiResponse<AttributeData>, String> {
    let bridge = bridge.read().await;
    match bridge
        .delete::<Attribute>(&format!("/api/attributes/{}/options/{}", encode(&attribute_id), index))
        .await
    {
        Ok(template) => Ok(ApiResponse::success(AttributeData { template })),
        Err(e) => Ok(ApiResponse::error_with_code(ErrorCode::DatabaseError, e.to_string())),
    }
}

// ============ Kitchen Printers ============

#[tauri::command(rename_all = "snake_case")]
pub async fn list_kitchen_printers(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
) -> Result<ApiResponse<PrinterListData>, String> {
    let bridge = bridge.read().await;
    match bridge
        .get::<Vec<KitchenPrinter>>("/api/kitchen-printers")
        .await
    {
        Ok(printers) => Ok(ApiResponse::success(PrinterListData { printers })),
        Err(e) => Ok(ApiResponse::error_with_code(ErrorCode::DatabaseError, e.to_string())),
    }
}

#[tauri::command(rename_all = "snake_case")]
pub async fn get_kitchen_printer(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    id: String,
) -> Result<ApiResponse<PrinterData>, String> {
    let bridge = bridge.read().await;
    match bridge
        .get::<KitchenPrinter>(&format!("/api/kitchen-printers/{}", encode(&id)))
        .await
    {
        Ok(p) => Ok(ApiResponse::success(PrinterData { printer: p })),
        Err(e) => Ok(ApiResponse::error_with_code(ErrorCode::PrinterNotAvailable, e.to_string())),
    }
}

#[tauri::command(rename_all = "snake_case")]
pub async fn create_kitchen_printer(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    data: KitchenPrinterCreate,
) -> Result<ApiResponse<PrinterData>, String> {
    let bridge = bridge.read().await;
    match bridge
        .post::<KitchenPrinter, _>("/api/kitchen-printers", &data)
        .await
    {
        Ok(p) => Ok(ApiResponse::success(PrinterData { printer: p })),
        Err(e) => Ok(ApiResponse::error_with_code(ErrorCode::DatabaseError, e.to_string())),
    }
}

#[tauri::command(rename_all = "snake_case")]
pub async fn update_kitchen_printer(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    id: String,
    data: KitchenPrinterUpdate,
) -> Result<ApiResponse<PrinterData>, String> {
    let bridge = bridge.read().await;
    match bridge
        .put::<KitchenPrinter, _>(&format!("/api/kitchen-printers/{}", encode(&id)), &data)
        .await
    {
        Ok(p) => Ok(ApiResponse::success(PrinterData { printer: p })),
        Err(e) => Ok(ApiResponse::error_with_code(ErrorCode::DatabaseError, e.to_string())),
    }
}

#[tauri::command(rename_all = "snake_case")]
pub async fn delete_kitchen_printer(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    id: String,
) -> Result<ApiResponse<DeleteData>, String> {
    let bridge = bridge.read().await;
    match bridge
        .delete::<bool>(&format!("/api/kitchen-printers/{}", encode(&id)))
        .await
    {
        Ok(deleted) => Ok(ApiResponse::success(DeleteData { deleted })),
        Err(e) => Ok(ApiResponse::error_with_code(ErrorCode::DatabaseError, e.to_string())),
    }
}

// ============ Product Attributes (Bindings) ============

#[tauri::command(rename_all = "snake_case")]
pub async fn list_product_attributes(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    product_id: String,
) -> Result<ApiResponse<serde_json::Value>, String> {
    let bridge = bridge.read().await;
    match bridge
        .get::<serde_json::Value>(&format!("/api/products/{}/attributes", encode(&product_id)))
        .await
    {
        Ok(attrs) => Ok(ApiResponse::success(attrs)),
        Err(e) => Ok(ApiResponse::error_with_code(ErrorCode::DatabaseError, e.to_string())),
    }
}

#[tauri::command(rename_all = "snake_case")]
pub async fn bind_product_attribute(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    data: serde_json::Value,
) -> Result<ApiResponse<serde_json::Value>, String> {
    let bridge = bridge.read().await;
    match bridge
        .post::<serde_json::Value, _>("/api/has-attribute", &data)
        .await
    {
        Ok(result) => Ok(ApiResponse::success(result)),
        Err(e) => Ok(ApiResponse::error_with_code(ErrorCode::AttributeBindFailed, e.to_string())),
    }
}

#[tauri::command(rename_all = "snake_case")]
pub async fn unbind_product_attribute(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    id: String,
) -> Result<ApiResponse<DeleteData>, String> {
    let bridge = bridge.read().await;
    match bridge
        .delete::<bool>(&format!("/api/has-attribute/{}", id))
        .await
    {
        Ok(deleted) => Ok(ApiResponse::success(DeleteData { deleted })),
        Err(e) => Ok(ApiResponse::error_with_code(ErrorCode::DatabaseError, e.to_string())),
    }
}

#[tauri::command(rename_all = "snake_case")]
pub async fn update_product_attribute_binding(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    id: String,
    data: serde_json::Value,
) -> Result<ApiResponse<serde_json::Value>, String> {
    let bridge = bridge.read().await;
    match bridge
        .put::<serde_json::Value, _>(&format!("/api/has-attribute/{}", id), &data)
        .await
    {
        Ok(result) => Ok(ApiResponse::success(result)),
        Err(e) => Ok(ApiResponse::error_with_code(ErrorCode::DatabaseError, e.to_string())),
    }
}

// ============ Category Attributes (Bindings) ============

/// List attributes for a category
#[tauri::command(rename_all = "snake_case")]
pub async fn list_category_attributes(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    category_id: String,
) -> Result<ApiResponse<AttributeListData>, String> {
    let bridge = bridge.read().await;
    match bridge
        .get::<Vec<Attribute>>(&format!("/api/categories/{}/attributes", category_id))
        .await
    {
        Ok(templates) => Ok(ApiResponse::success(AttributeListData { templates })),
        Err(e) => Ok(ApiResponse::error_with_code(ErrorCode::DatabaseError, e.to_string())),
    }
}

/// Payload for binding attribute to category
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct BindCategoryAttributeData {
    pub category_id: String,
    pub attribute_id: String,
    pub is_required: Option<bool>,
    pub display_order: Option<i32>,
    pub default_option_id: Option<i32>,
}

/// Bind attribute to category
#[tauri::command(rename_all = "snake_case")]
pub async fn bind_category_attribute(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    data: BindCategoryAttributeData,
) -> Result<ApiResponse<HasAttribute>, String> {
    let bridge = bridge.read().await;
    // Build payload for API
    let payload = serde_json::json!({
        "is_required": data.is_required,
        "display_order": data.display_order,
        "default_option_idx": data.default_option_id,
    });
    match bridge
        .post::<HasAttribute, _>(
            &format!("/api/categories/{}/attributes/{}", data.category_id, data.attribute_id),
            &payload,
        )
        .await
    {
        Ok(binding) => Ok(ApiResponse::success(binding)),
        Err(e) => Ok(ApiResponse::error_with_code(ErrorCode::AttributeBindFailed, e.to_string())),
    }
}

/// Unbind attribute from category
#[tauri::command(rename_all = "snake_case")]
pub async fn unbind_category_attribute(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    category_id: String,
    attribute_id: String,
) -> Result<ApiResponse<DeleteData>, String> {
    let bridge = bridge.read().await;
    match bridge
        .delete::<bool>(&format!(
            "/api/categories/{}/attributes/{}",
            category_id, attribute_id
        ))
        .await
    {
        Ok(deleted) => Ok(ApiResponse::success(DeleteData { deleted })),
        Err(e) => Ok(ApiResponse::error_with_code(ErrorCode::DatabaseError, e.to_string())),
    }
}

/// Payload for batch sort order update
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct CategorySortOrderUpdate {
    pub id: String,
    pub sort_order: i32,
}

/// Response for batch update operation
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct BatchUpdateResponse {
    pub updated: usize,
}

/// Batch update category sort order
#[tauri::command(rename_all = "snake_case")]
pub async fn batch_update_category_sort_order(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    updates: Vec<CategorySortOrderUpdate>,
) -> Result<ApiResponse<BatchUpdateResponse>, String> {
    let bridge = bridge.read().await;
    match bridge
        .put::<BatchUpdateResponse, _>("/api/categories/sort-order", &updates)
        .await
    {
        Ok(result) => Ok(ApiResponse::success(result)),
        Err(e) => Ok(ApiResponse::error_with_code(ErrorCode::DatabaseError, e.to_string())),
    }
}
