//! 数据 API Commands
//!
//! 通过 ClientBridge -> CrabClient -> EdgeServer REST API

use std::sync::Arc;
use tauri::State;
use tokio::sync::RwLock;

use crate::core::ClientBridge;
use shared::models::{
    // Tags
    Tag, TagCreate, TagUpdate,
    // Categories
    Category, CategoryCreate, CategoryUpdate,
    // Products
    Product, ProductCreate, ProductUpdate,
    ProductSpecification, ProductSpecificationCreate, ProductSpecificationUpdate,
    // Attributes
    Attribute, AttributeCreate, AttributeUpdate,
    // Kitchen Printers
    KitchenPrinter, KitchenPrinterCreate, KitchenPrinterUpdate,
};

// ============ Tags ============

#[tauri::command]
pub async fn list_tags(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
) -> Result<Vec<Tag>, String> {
    let bridge = bridge.read().await;
    bridge.get("/api/tags").await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_tag(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    id: String,
) -> Result<Tag, String> {
    let bridge = bridge.read().await;
    bridge.get(&format!("/api/tags/{}", id)).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn create_tag(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    data: TagCreate,
) -> Result<Tag, String> {
    let bridge = bridge.read().await;
    bridge.post("/api/tags", &data).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_tag(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    id: String,
    data: TagUpdate,
) -> Result<Tag, String> {
    let bridge = bridge.read().await;
    bridge.put(&format!("/api/tags/{}", id), &data).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_tag(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    id: String,
) -> Result<bool, String> {
    let bridge = bridge.read().await;
    bridge.delete(&format!("/api/tags/{}", id)).await.map_err(|e| e.to_string())
}

// ============ Categories ============

#[tauri::command]
pub async fn list_categories(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
) -> Result<Vec<Category>, String> {
    let bridge = bridge.read().await;
    bridge.get("/api/categories").await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_category(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    id: String,
) -> Result<Category, String> {
    let bridge = bridge.read().await;
    bridge.get(&format!("/api/categories/{}", id)).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn create_category(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    data: CategoryCreate,
) -> Result<Category, String> {
    let bridge = bridge.read().await;
    bridge.post("/api/categories", &data).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_category(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    id: String,
    data: CategoryUpdate,
) -> Result<Category, String> {
    let bridge = bridge.read().await;
    bridge.put(&format!("/api/categories/{}", id), &data).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_category(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    id: String,
) -> Result<bool, String> {
    let bridge = bridge.read().await;
    bridge.delete(&format!("/api/categories/{}", id)).await.map_err(|e| e.to_string())
}

// ============ Products ============

#[tauri::command]
pub async fn list_products(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
) -> Result<Vec<Product>, String> {
    let bridge = bridge.read().await;
    bridge.get("/api/products").await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_product(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    id: String,
) -> Result<Product, String> {
    let bridge = bridge.read().await;
    bridge.get(&format!("/api/products/{}", id)).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn create_product(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    data: ProductCreate,
) -> Result<Product, String> {
    let bridge = bridge.read().await;
    bridge.post("/api/products", &data).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_product(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    id: String,
    data: ProductUpdate,
) -> Result<Product, String> {
    let bridge = bridge.read().await;
    bridge.put(&format!("/api/products/{}", id), &data).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_product(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    id: String,
) -> Result<bool, String> {
    let bridge = bridge.read().await;
    bridge.delete(&format!("/api/products/{}", id)).await.map_err(|e| e.to_string())
}

// ============ Product Specifications ============

#[tauri::command]
pub async fn list_specs(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    product_id: String,
) -> Result<Vec<ProductSpecification>, String> {
    let bridge = bridge.read().await;
    bridge.get(&format!("/api/specs/product/{}", product_id)).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_spec(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    id: String,
) -> Result<ProductSpecification, String> {
    let bridge = bridge.read().await;
    bridge.get(&format!("/api/specs/{}", id)).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn create_spec(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    data: ProductSpecificationCreate,
) -> Result<ProductSpecification, String> {
    let bridge = bridge.read().await;
    bridge.post("/api/specs", &data).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_spec(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    id: String,
    data: ProductSpecificationUpdate,
) -> Result<ProductSpecification, String> {
    let bridge = bridge.read().await;
    bridge.put(&format!("/api/specs/{}", id), &data).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_spec(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    id: String,
) -> Result<bool, String> {
    let bridge = bridge.read().await;
    bridge.delete(&format!("/api/specs/{}", id)).await.map_err(|e| e.to_string())
}

// ============ Attributes ============

#[tauri::command]
pub async fn list_attributes(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
) -> Result<Vec<Attribute>, String> {
    let bridge = bridge.read().await;
    bridge.get("/api/attributes").await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_attribute(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    id: String,
) -> Result<Attribute, String> {
    let bridge = bridge.read().await;
    bridge.get(&format!("/api/attributes/{}", id)).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn create_attribute(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    data: AttributeCreate,
) -> Result<Attribute, String> {
    let bridge = bridge.read().await;
    bridge.post("/api/attributes", &data).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_attribute(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    id: String,
    data: AttributeUpdate,
) -> Result<Attribute, String> {
    let bridge = bridge.read().await;
    bridge.put(&format!("/api/attributes/{}", id), &data).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_attribute(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    id: String,
) -> Result<bool, String> {
    let bridge = bridge.read().await;
    bridge.delete(&format!("/api/attributes/{}", id)).await.map_err(|e| e.to_string())
}

// ============ Kitchen Printers ============

#[tauri::command]
pub async fn list_kitchen_printers(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
) -> Result<Vec<KitchenPrinter>, String> {
    let bridge = bridge.read().await;
    bridge.get("/api/kitchen-printers").await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_kitchen_printer(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    id: String,
) -> Result<KitchenPrinter, String> {
    let bridge = bridge.read().await;
    bridge.get(&format!("/api/kitchen-printers/{}", id)).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn create_kitchen_printer(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    data: KitchenPrinterCreate,
) -> Result<KitchenPrinter, String> {
    let bridge = bridge.read().await;
    bridge.post("/api/kitchen-printers", &data).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_kitchen_printer(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    id: String,
    data: KitchenPrinterUpdate,
) -> Result<KitchenPrinter, String> {
    let bridge = bridge.read().await;
    bridge.put(&format!("/api/kitchen-printers/{}", id), &data).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_kitchen_printer(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    id: String,
) -> Result<bool, String> {
    let bridge = bridge.read().await;
    bridge.delete(&format!("/api/kitchen-printers/{}", id)).await.map_err(|e| e.to_string())
}

// ============ Product Attributes (Bindings) ============

#[tauri::command]
pub async fn list_product_attributes(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    product_id: String,
) -> Result<serde_json::Value, String> {
    let bridge = bridge.read().await;
    bridge.get(&format!("/api/products/{}/attributes", product_id)).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn bind_product_attribute(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    data: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let bridge = bridge.read().await;
    bridge.post("/api/has-attribute", &data).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn unbind_product_attribute(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    id: String,
) -> Result<bool, String> {
    let bridge = bridge.read().await;
    bridge.delete(&format!("/api/has-attribute/{}", id)).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_product_attribute_binding(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    id: String,
    data: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let bridge = bridge.read().await;
    bridge.put(&format!("/api/has-attribute/{}", id), &data).await.map_err(|e| e.to_string())
}
