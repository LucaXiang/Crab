//! 数据 API Commands
//!
//! 通过 ClientBridge -> CrabClient -> EdgeServer REST API
//! 所有响应使用 ApiResponse<T> 格式包装

use std::sync::Arc;
use tauri::State;
use tokio::sync::RwLock;

use crate::core::{
    error_codes::{attribute, category, printer, product, spec, tag},
    ApiResponse, AttributeData, AttributeListData, CategoryData, CategoryListData, ClientBridge,
    DeleteData, PrinterData, PrinterListData, ProductData, ProductListData, SpecListData,
    TagListData,
};
use shared::models::{
    // Attributes
    Attribute,
    AttributeCreate,
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
    ProductSpecification,
    ProductSpecificationCreate,
    ProductSpecificationUpdate,
    ProductUpdate,
    // Tags
    Tag,
    TagCreate,
    TagUpdate,
};
use urlencoding::encode;

// ============ Tags ============

#[tauri::command]
pub async fn list_tags(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
) -> Result<ApiResponse<TagListData>, String> {
    let bridge = bridge.read().await;
    match bridge.get::<Vec<Tag>>("/api/tags").await {
        Ok(tags) => Ok(ApiResponse::success(TagListData { tags })),
        Err(e) => Ok(ApiResponse::error(tag::LIST_FAILED, e.to_string())),
    }
}

#[tauri::command]
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
        Err(e) => Ok(ApiResponse::error(tag::GET_FAILED, e.to_string())),
    }
}

#[tauri::command]
pub async fn create_tag(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    data: TagCreate,
) -> Result<ApiResponse<Tag>, String> {
    let bridge = bridge.read().await;
    match bridge.post::<Tag, _>("/api/tags", &data).await {
        Ok(tag) => Ok(ApiResponse::success(tag)),
        Err(e) => Ok(ApiResponse::error(tag::CREATE_FAILED, e.to_string())),
    }
}

#[tauri::command]
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
        Err(e) => Ok(ApiResponse::error(tag::UPDATE_FAILED, e.to_string())),
    }
}

#[tauri::command]
pub async fn delete_tag(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    id: String,
) -> Result<ApiResponse<DeleteData>, String> {
    let bridge = bridge.read().await;
    match bridge
        .delete::<bool>(&format!("/api/tags/{}", encode(&id)))
        .await
    {
        Ok(_) => Ok(ApiResponse::success(DeleteData::success())),
        Err(e) => Ok(ApiResponse::error(tag::DELETE_FAILED, e.to_string())),
    }
}

// ============ Categories ============

#[tauri::command]
pub async fn list_categories(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
) -> Result<ApiResponse<CategoryListData>, String> {
    let bridge = bridge.read().await;
    match bridge.get::<Vec<Category>>("/api/categories").await {
        Ok(categories) => Ok(ApiResponse::success(CategoryListData { categories })),
        Err(e) => Ok(ApiResponse::error(category::LIST_FAILED, e.to_string())),
    }
}

#[tauri::command]
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
        Err(e) => Ok(ApiResponse::error(category::GET_FAILED, e.to_string())),
    }
}

#[tauri::command]
pub async fn create_category(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    data: CategoryCreate,
) -> Result<ApiResponse<CategoryData>, String> {
    let bridge = bridge.read().await;
    match bridge.post::<Category, _>("/api/categories", &data).await {
        Ok(cat) => Ok(ApiResponse::success(CategoryData { category: cat })),
        Err(e) => Ok(ApiResponse::error(category::CREATE_FAILED, e.to_string())),
    }
}

#[tauri::command]
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
        Err(e) => Ok(ApiResponse::error(category::UPDATE_FAILED, e.to_string())),
    }
}

#[tauri::command]
pub async fn delete_category(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    id: String,
) -> Result<ApiResponse<DeleteData>, String> {
    let bridge = bridge.read().await;
    match bridge
        .delete::<bool>(&format!("/api/categories/{}", encode(&id)))
        .await
    {
        Ok(_) => Ok(ApiResponse::success(DeleteData::success())),
        Err(e) => Ok(ApiResponse::error(category::DELETE_FAILED, e.to_string())),
    }
}

// ============ Products ============

#[tauri::command]
pub async fn list_products(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
) -> Result<ApiResponse<ProductListData>, String> {
    let bridge = bridge.read().await;
    match bridge.get::<Vec<Product>>("/api/products").await {
        Ok(products) => Ok(ApiResponse::success(ProductListData { products })),
        Err(e) => Ok(ApiResponse::error(product::LIST_FAILED, e.to_string())),
    }
}

#[tauri::command]
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
        Err(e) => Ok(ApiResponse::error(product::GET_FAILED, e.to_string())),
    }
}

#[tauri::command]
pub async fn create_product(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    data: ProductCreate,
) -> Result<ApiResponse<ProductData>, String> {
    let bridge = bridge.read().await;
    match bridge.post::<Product, _>("/api/products", &data).await {
        Ok(prod) => Ok(ApiResponse::success(ProductData { product: prod })),
        Err(e) => Ok(ApiResponse::error(product::CREATE_FAILED, e.to_string())),
    }
}

#[tauri::command]
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
        Err(e) => Ok(ApiResponse::error(product::UPDATE_FAILED, e.to_string())),
    }
}

#[tauri::command]
pub async fn delete_product(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    id: String,
) -> Result<ApiResponse<DeleteData>, String> {
    let bridge = bridge.read().await;
    match bridge
        .delete::<bool>(&format!("/api/products/{}", encode(&id)))
        .await
    {
        Ok(_) => Ok(ApiResponse::success(DeleteData::success())),
        Err(e) => Ok(ApiResponse::error(product::DELETE_FAILED, e.to_string())),
    }
}

// ============ Product Specifications ============

#[tauri::command]
pub async fn list_specs(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    product_id: String,
) -> Result<ApiResponse<SpecListData>, String> {
    let bridge = bridge.read().await;
    match bridge
        .get::<Vec<ProductSpecification>>(&format!("/api/specs/product/{}", encode(&product_id)))
        .await
    {
        Ok(specs) => Ok(ApiResponse::success(SpecListData { specs })),
        Err(e) => Ok(ApiResponse::error(spec::LIST_FAILED, e.to_string())),
    }
}

#[tauri::command]
pub async fn get_spec(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    id: String,
) -> Result<ApiResponse<ProductSpecification>, String> {
    let bridge = bridge.read().await;
    match bridge
        .get::<ProductSpecification>(&format!("/api/specs/{}", encode(&id)))
        .await
    {
        Ok(s) => Ok(ApiResponse::success(s)),
        Err(e) => Ok(ApiResponse::error(spec::GET_FAILED, e.to_string())),
    }
}

#[tauri::command]
pub async fn create_spec(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    data: ProductSpecificationCreate,
) -> Result<ApiResponse<ProductSpecification>, String> {
    let bridge = bridge.read().await;
    match bridge
        .post::<ProductSpecification, _>("/api/specs", &data)
        .await
    {
        Ok(s) => Ok(ApiResponse::success(s)),
        Err(e) => Ok(ApiResponse::error(spec::CREATE_FAILED, e.to_string())),
    }
}

#[tauri::command]
pub async fn update_spec(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    id: String,
    data: ProductSpecificationUpdate,
) -> Result<ApiResponse<ProductSpecification>, String> {
    let bridge = bridge.read().await;
    match bridge
        .put::<ProductSpecification, _>(&format!("/api/specs/{}", encode(&id)), &data)
        .await
    {
        Ok(s) => Ok(ApiResponse::success(s)),
        Err(e) => Ok(ApiResponse::error(spec::UPDATE_FAILED, e.to_string())),
    }
}

#[tauri::command]
pub async fn delete_spec(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    id: String,
) -> Result<ApiResponse<DeleteData>, String> {
    let bridge = bridge.read().await;
    match bridge
        .delete::<bool>(&format!("/api/specs/{}", encode(&id)))
        .await
    {
        Ok(_) => Ok(ApiResponse::success(DeleteData::success())),
        Err(e) => Ok(ApiResponse::error(spec::DELETE_FAILED, e.to_string())),
    }
}

// ============ Attributes ============

#[tauri::command]
pub async fn list_attributes(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
) -> Result<ApiResponse<AttributeListData>, String> {
    let bridge = bridge.read().await;
    match bridge.get::<Vec<Attribute>>("/api/attributes").await {
        Ok(templates) => Ok(ApiResponse::success(AttributeListData { templates })),
        Err(e) => Ok(ApiResponse::error(attribute::LIST_FAILED, e.to_string())),
    }
}

#[tauri::command]
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
        Err(e) => Ok(ApiResponse::error(attribute::GET_FAILED, e.to_string())),
    }
}

#[tauri::command]
pub async fn create_attribute(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    data: AttributeCreate,
) -> Result<ApiResponse<AttributeData>, String> {
    let bridge = bridge.read().await;
    match bridge.post::<Attribute, _>("/api/attributes", &data).await {
        Ok(template) => Ok(ApiResponse::success(AttributeData { template })),
        Err(e) => Ok(ApiResponse::error(attribute::CREATE_FAILED, e.to_string())),
    }
}

#[tauri::command]
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
        Err(e) => Ok(ApiResponse::error(attribute::UPDATE_FAILED, e.to_string())),
    }
}

#[tauri::command]
pub async fn delete_attribute(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    id: String,
) -> Result<ApiResponse<DeleteData>, String> {
    let bridge = bridge.read().await;
    match bridge
        .delete::<bool>(&format!("/api/attributes/{}", encode(&id)))
        .await
    {
        Ok(_) => Ok(ApiResponse::success(DeleteData::success())),
        Err(e) => Ok(ApiResponse::error(attribute::DELETE_FAILED, e.to_string())),
    }
}

// ============ Kitchen Printers ============

#[tauri::command]
pub async fn list_kitchen_printers(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
) -> Result<ApiResponse<PrinterListData>, String> {
    let bridge = bridge.read().await;
    match bridge
        .get::<Vec<KitchenPrinter>>("/api/kitchen-printers")
        .await
    {
        Ok(printers) => Ok(ApiResponse::success(PrinterListData { printers })),
        Err(e) => Ok(ApiResponse::error(printer::LIST_FAILED, e.to_string())),
    }
}

#[tauri::command]
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
        Err(e) => Ok(ApiResponse::error(printer::GET_FAILED, e.to_string())),
    }
}

#[tauri::command]
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
        Err(e) => Ok(ApiResponse::error(printer::CREATE_FAILED, e.to_string())),
    }
}

#[tauri::command]
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
        Err(e) => Ok(ApiResponse::error(printer::UPDATE_FAILED, e.to_string())),
    }
}

#[tauri::command]
pub async fn delete_kitchen_printer(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    id: String,
) -> Result<ApiResponse<DeleteData>, String> {
    let bridge = bridge.read().await;
    match bridge
        .delete::<bool>(&format!("/api/kitchen-printers/{}", encode(&id)))
        .await
    {
        Ok(_) => Ok(ApiResponse::success(DeleteData::success())),
        Err(e) => Ok(ApiResponse::error(printer::DELETE_FAILED, e.to_string())),
    }
}

// ============ Product Attributes (Bindings) ============

#[tauri::command]
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
        Err(e) => Ok(ApiResponse::error(attribute::LIST_FAILED, e.to_string())),
    }
}

#[tauri::command]
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
        Err(e) => Ok(ApiResponse::error(attribute::BIND_FAILED, e.to_string())),
    }
}

#[tauri::command]
pub async fn unbind_product_attribute(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    id: String,
) -> Result<ApiResponse<DeleteData>, String> {
    let bridge = bridge.read().await;
    match bridge
        .delete::<bool>(&format!("/api/has-attribute/{}", id))
        .await
    {
        Ok(_) => Ok(ApiResponse::success(DeleteData::success())),
        Err(e) => Ok(ApiResponse::error(attribute::UNBIND_FAILED, e.to_string())),
    }
}

#[tauri::command]
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
        Err(e) => Ok(ApiResponse::error(attribute::UPDATE_FAILED, e.to_string())),
    }
}

// ============ Category Attributes (Bindings) ============

/// List attributes for a category
#[tauri::command]
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
        Err(e) => Ok(ApiResponse::error(attribute::LIST_FAILED, e.to_string())),
    }
}

/// Payload for binding attribute to category
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct BindCategoryAttributePayload {
    pub is_required: Option<bool>,
    pub display_order: Option<i32>,
    pub default_option_idx: Option<i32>,
}

/// Bind attribute to category
#[tauri::command]
pub async fn bind_category_attribute(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    category_id: String,
    attr_id: String,
    payload: BindCategoryAttributePayload,
) -> Result<ApiResponse<HasAttribute>, String> {
    let bridge = bridge.read().await;
    match bridge
        .post::<HasAttribute, _>(
            &format!("/api/categories/{}/attributes/{}", category_id, attr_id),
            &payload,
        )
        .await
    {
        Ok(binding) => Ok(ApiResponse::success(binding)),
        Err(e) => Ok(ApiResponse::error(attribute::BIND_FAILED, e.to_string())),
    }
}

/// Unbind attribute from category
#[tauri::command]
pub async fn unbind_category_attribute(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    category_id: String,
    attr_id: String,
) -> Result<ApiResponse<DeleteData>, String> {
    let bridge = bridge.read().await;
    match bridge
        .delete::<bool>(&format!(
            "/api/categories/{}/attributes/{}",
            category_id, attr_id
        ))
        .await
    {
        Ok(_) => Ok(ApiResponse::success(DeleteData::success())),
        Err(e) => Ok(ApiResponse::error(attribute::UNBIND_FAILED, e.to_string())),
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
#[tauri::command]
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
        Err(e) => Ok(ApiResponse::error(category::UPDATE_FAILED, e.to_string())),
    }
}
