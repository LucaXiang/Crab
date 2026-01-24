//! Catalog Service - Unified Product and Category management with in-memory caching
//!
//! Replaces:
//! - ProductRepository
//! - CategoryRepository
//! - OrdersManager.product_meta_cache
//! - PrintConfigCache (product/category parts)
//! - PriceRuleEngine DB queries

use crate::db::models::{
    serde_helpers, Attribute, AttributeBindingFull, Category, CategoryCreate, CategoryUpdate,
    Product, ProductCreate, ProductFull, ProductUpdate, Tag,
};
use crate::db::repository::{RepoError, RepoResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use surrealdb::engine::local::Db;
use surrealdb::RecordId;
use surrealdb::Surreal;

// =============================================================================
// Types
// =============================================================================

/// Product with tags fetched (for FETCH queries)
#[derive(Debug, Clone, Deserialize)]
struct ProductWithTags {
    #[serde(default, with = "serde_helpers::option_record_id")]
    pub id: Option<RecordId>,
    pub name: String,
    #[serde(default)]
    pub image: String,
    #[serde(with = "serde_helpers::record_id")]
    pub category: RecordId,
    #[serde(default)]
    pub sort_order: i32,
    #[serde(default)]
    pub tax_rate: i32,
    pub receipt_name: Option<String>,
    pub kitchen_print_name: Option<String>,
    #[serde(default, with = "serde_helpers::vec_record_id")]
    pub kitchen_print_destinations: Vec<RecordId>,
    #[serde(default, with = "serde_helpers::vec_record_id")]
    pub label_print_destinations: Vec<RecordId>,
    #[serde(default)]
    pub is_kitchen_print_enabled: i32,
    #[serde(default)]
    pub is_label_print_enabled: i32,
    #[serde(default = "default_true")]
    pub is_active: bool,
    /// Tags are fetched as full Tag objects
    #[serde(default)]
    pub tags: Vec<Tag>,
    #[serde(default)]
    pub specs: Vec<crate::db::models::EmbeddedSpec>,
}

fn default_true() -> bool {
    true
}

/// Product metadata for price rule matching
#[derive(Debug, Clone, Default)]
pub struct ProductMeta {
    pub category_id: String, // "category:xxx"
    pub tags: Vec<String>,   // ["tag:xxx", ...]
}

/// Kitchen print configuration (computed result with fallback chain applied)
#[derive(Debug, Clone)]
pub struct KitchenPrintConfig {
    pub enabled: bool,
    pub destinations: Vec<String>, // ["print_destination:xxx", ...]
    pub kitchen_name: Option<String>,
}

/// Label print configuration (computed result with fallback chain applied)
#[derive(Debug, Clone)]
pub struct LabelPrintConfig {
    pub enabled: bool,
    pub destinations: Vec<String>,
}

/// System default print destinations
#[derive(Debug, Clone, Default)]
pub struct PrintDefaults {
    pub kitchen_destination: Option<String>,
    pub label_destination: Option<String>,
}

// =============================================================================
// CatalogService
// =============================================================================

/// Unified catalog service for Product and Category management
#[derive(Clone)]
pub struct CatalogService {
    db: Surreal<Db>,
    /// Products cache: "product:xxx" -> ProductFull
    products: Arc<RwLock<HashMap<String, ProductFull>>>,
    /// Categories cache: "category:xxx" -> Category
    categories: Arc<RwLock<HashMap<String, Category>>>,
    /// System default print destinations
    print_defaults: Arc<RwLock<PrintDefaults>>,
}

impl std::fmt::Debug for CatalogService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let products_count = self.products.read().map(|p| p.len()).unwrap_or(0);
        let categories_count = self.categories.read().map(|c| c.len()).unwrap_or(0);
        f.debug_struct("CatalogService")
            .field("products_count", &products_count)
            .field("categories_count", &categories_count)
            .finish()
    }
}

impl CatalogService {
    /// Create a new CatalogService
    pub fn new(db: Surreal<Db>) -> Self {
        Self {
            db,
            products: Arc::new(RwLock::new(HashMap::new())),
            categories: Arc::new(RwLock::new(HashMap::new())),
            print_defaults: Arc::new(RwLock::new(PrintDefaults::default())),
        }
    }

    // =========================================================================
    // Warmup
    // =========================================================================

    /// Load all products and categories into memory cache
    pub async fn warmup(&self) -> RepoResult<()> {
        // 1. Load all categories
        let categories: Vec<Category> = self
            .db
            .query("SELECT * FROM category WHERE is_active = true ORDER BY sort_order")
            .await?
            .take(0)?;

        {
            let mut cache = self.categories.write().unwrap();
            cache.clear();
            for cat in &categories {
                if let Some(id) = &cat.id {
                    cache.insert(id.to_string(), cat.clone());
                }
            }
        }
        tracing::info!("📦 CatalogService: Loaded {} categories", categories.len());

        // 2. Load all products with tags fetched (using ProductWithTags to deserialize full Tag objects)
        let products: Vec<ProductWithTags> = self
            .db
            .query("SELECT * FROM product WHERE is_active = true ORDER BY sort_order FETCH tags")
            .await?
            .take(0)?;

        // 3. Load all attribute bindings with full attribute data
        #[derive(Debug, Deserialize)]
        struct BindingRow {
            #[serde(default, with = "serde_helpers::option_record_id")]
            id: Option<RecordId>,
            #[serde(rename = "in", with = "serde_helpers::record_id")]
            from: RecordId,
            #[serde(rename = "out")]
            to: Attribute,
            #[serde(default)]
            is_required: bool,
            #[serde(default)]
            display_order: i32,
            default_option_idx: Option<i32>,
        }

        let bindings: Vec<BindingRow> = self
            .db
            .query("SELECT * FROM attribute_binding WHERE in.is_active = true FETCH out")
            .await?
            .take(0)?;

        // Group bindings by product_id
        let mut product_bindings: HashMap<String, Vec<AttributeBindingFull>> = HashMap::new();
        for binding in bindings {
            let from_id = binding.from.to_string();
            // Only include product bindings (not category bindings)
            if from_id.starts_with("product:") {
                let full = AttributeBindingFull {
                    id: binding.id,
                    attribute: binding.to,
                    is_required: binding.is_required,
                    display_order: binding.display_order,
                    default_option_idx: binding.default_option_idx,
                };
                product_bindings
                    .entry(from_id)
                    .or_default()
                    .push(full);
            }
        }

        // 4. Build ProductFull and store in cache
        {
            let mut cache = self.products.write().unwrap();
            cache.clear();

            for product in products {
                let product_id = match &product.id {
                    Some(id) => id.to_string(),
                    None => continue,
                };

                // Tags are already fetched as full Tag objects (name, color, etc.)
                let tags = product.tags;

                // Get attribute bindings for this product
                let attributes = product_bindings
                    .remove(&product_id)
                    .unwrap_or_default();

                let full = ProductFull {
                    id: product.id,
                    name: product.name,
                    image: product.image,
                    category: product.category,
                    sort_order: product.sort_order,
                    tax_rate: product.tax_rate,
                    receipt_name: product.receipt_name,
                    kitchen_print_name: product.kitchen_print_name,
                    kitchen_print_destinations: product.kitchen_print_destinations,
                    label_print_destinations: product.label_print_destinations,
                    is_kitchen_print_enabled: product.is_kitchen_print_enabled,
                    is_label_print_enabled: product.is_label_print_enabled,
                    is_active: product.is_active,
                    specs: product.specs,
                    attributes,
                    tags,
                };

                cache.insert(product_id, full);
            }
        }

        let products_count = self.products.read().unwrap().len();
        tracing::info!("📦 CatalogService: Loaded {} products", products_count);

        Ok(())
    }

    /// Set system default print destinations
    pub fn set_print_defaults(&self, kitchen: Option<String>, label: Option<String>) {
        let mut defaults = self.print_defaults.write().unwrap();
        defaults.kitchen_destination = kitchen;
        defaults.label_destination = label;
    }

    /// Get system default print destinations
    pub fn get_print_defaults(&self) -> PrintDefaults {
        self.print_defaults.read().unwrap().clone()
    }

    // =========================================================================
    // Product - Read (from cache)
    // =========================================================================

    /// Get product by ID (from cache)
    pub fn get_product(&self, id: &str) -> Option<ProductFull> {
        let cache = self.products.read().unwrap();
        cache.get(id).cloned()
    }

    /// List all products (from cache)
    pub fn list_products(&self) -> Vec<ProductFull> {
        let cache = self.products.read().unwrap();
        let mut products: Vec<_> = cache.values().cloned().collect();
        products.sort_by_key(|p| p.sort_order);
        products
    }

    /// Get products by category ID (from cache)
    pub fn get_products_by_category(&self, category_id: &str) -> Vec<ProductFull> {
        let cache = self.products.read().unwrap();
        let mut products: Vec<_> = cache
            .values()
            .filter(|p| p.category.to_string() == category_id)
            .cloned()
            .collect();
        products.sort_by_key(|p| p.sort_order);
        products
    }

    // =========================================================================
    // Product - Write (DB first, then cache)
    // =========================================================================

    /// Create a new product
    pub async fn create_product(&self, data: ProductCreate) -> RepoResult<ProductFull> {
        // Validate specs
        if data.specs.is_empty() {
            return Err(RepoError::Validation("specs cannot be empty".into()));
        }
        let default_count = data.specs.iter().filter(|s| s.is_default).count();
        if default_count > 1 {
            return Err(RepoError::Validation("only one default spec allowed".into()));
        }

        // Validate category is not virtual
        {
            let categories = self.categories.read().unwrap();
            let cat_id = data.category.to_string();
            if let Some(cat) = categories.get(&cat_id)
                && cat.is_virtual
            {
                return Err(RepoError::Validation(
                    "Product cannot belong to a virtual category".into(),
                ));
            }
        }

        let product = Product {
            id: None,
            name: data.name,
            image: data.image.unwrap_or_default(),
            category: data.category,
            sort_order: data.sort_order.unwrap_or(0),
            tax_rate: data.tax_rate.unwrap_or(0),
            receipt_name: data.receipt_name,
            kitchen_print_name: data.kitchen_print_name,
            kitchen_print_destinations: data.kitchen_print_destinations.unwrap_or_default(),
            label_print_destinations: data.label_print_destinations.unwrap_or_default(),
            is_kitchen_print_enabled: data.is_kitchen_print_enabled.unwrap_or(-1),
            is_label_print_enabled: data.is_label_print_enabled.unwrap_or(-1),
            is_active: true,
            tags: data.tags.unwrap_or_default(),
            specs: data.specs,
        };

        let created: Option<Product> = self.db.create("product").content(product).await?;
        let created =
            created.ok_or_else(|| RepoError::Database("Failed to create product".into()))?;

        // Build ProductFull with empty attributes and tags (they need to be fetched separately)
        let product_id = created.id.as_ref().map(|t| t.to_string()).unwrap_or_default();

        // Fetch the created product with tags
        let full = self.fetch_product_full(&product_id).await?;

        // Update cache
        {
            let mut cache = self.products.write().unwrap();
            cache.insert(product_id, full.clone());
        }

        Ok(full)
    }

    /// Update a product
    pub async fn update_product(&self, id: &str, data: ProductUpdate) -> RepoResult<ProductFull> {
        let thing = id.parse::<RecordId>()
            .map_err(|_| RepoError::Validation(format!("Invalid product ID: {}", id)))?;

        // Build dynamic SET clauses
        let mut set_parts: Vec<&str> = Vec::new();
        if data.name.is_some() { set_parts.push("name = $name"); }
        if data.image.is_some() { set_parts.push("image = $image"); }
        if data.category.is_some() { set_parts.push("category = $category"); }
        if data.sort_order.is_some() { set_parts.push("sort_order = $sort_order"); }
        if data.tax_rate.is_some() { set_parts.push("tax_rate = $tax_rate"); }
        if data.receipt_name.is_some() { set_parts.push("receipt_name = $receipt_name"); }
        if data.kitchen_print_name.is_some() { set_parts.push("kitchen_print_name = $kitchen_print_name"); }
        if data.kitchen_print_destinations.is_some() { set_parts.push("kitchen_print_destinations = $kitchen_print_destinations"); }
        if data.label_print_destinations.is_some() { set_parts.push("label_print_destinations = $label_print_destinations"); }
        if data.is_kitchen_print_enabled.is_some() { set_parts.push("is_kitchen_print_enabled = $is_kitchen_print_enabled"); }
        if data.is_label_print_enabled.is_some() { set_parts.push("is_label_print_enabled = $is_label_print_enabled"); }
        if data.is_active.is_some() { set_parts.push("is_active = $is_active"); }
        if data.tags.is_some() { set_parts.push("tags = $tags"); }
        if data.specs.is_some() { set_parts.push("specs = $specs"); }

        if set_parts.is_empty() {
            return self.get_product(id)
                .ok_or_else(|| RepoError::NotFound(format!("Product {} not found", id)));
        }

        // Validate category if changing
        if let Some(ref new_cat) = data.category {
            let categories = self.categories.read().unwrap();
            let cat_id = new_cat.to_string();
            if let Some(cat) = categories.get(&cat_id)
                && cat.is_virtual
            {
                return Err(RepoError::Validation(
                    "Product cannot belong to a virtual category".into(),
                ));
            }
        }

        let query_str = format!("UPDATE $thing SET {} RETURN AFTER", set_parts.join(", "));
        let mut query = self.db.query(&query_str).bind(("thing", thing));

        // Bind each field
        if let Some(v) = data.name { query = query.bind(("name", v)); }
        if let Some(v) = data.image { query = query.bind(("image", v)); }
        if let Some(v) = data.category { query = query.bind(("category", v)); }
        if let Some(v) = data.sort_order { query = query.bind(("sort_order", v)); }
        if let Some(v) = data.tax_rate { query = query.bind(("tax_rate", v)); }
        if let Some(v) = data.receipt_name { query = query.bind(("receipt_name", v)); }
        if let Some(v) = data.kitchen_print_name { query = query.bind(("kitchen_print_name", v)); }
        if let Some(v) = data.kitchen_print_destinations { query = query.bind(("kitchen_print_destinations", v)); }
        if let Some(v) = data.label_print_destinations { query = query.bind(("label_print_destinations", v)); }
        if let Some(v) = data.is_kitchen_print_enabled { query = query.bind(("is_kitchen_print_enabled", v)); }
        if let Some(v) = data.is_label_print_enabled { query = query.bind(("is_label_print_enabled", v)); }
        if let Some(v) = data.is_active { query = query.bind(("is_active", v)); }
        if let Some(v) = data.tags { query = query.bind(("tags", v)); }
        if let Some(v) = data.specs {
            query = query.bind(("specs", serde_json::to_value(&v).unwrap_or_default()));
        }

        let mut result = query.await?;
        let _updated: Vec<Product> = result.take(0)?;

        // Fetch full product data
        let full = self.fetch_product_full(id).await?;

        // Update cache
        {
            let mut cache = self.products.write().unwrap();
            if full.is_active {
                cache.insert(id.to_string(), full.clone());
            } else {
                cache.remove(id);
            }
        }

        Ok(full)
    }

    /// Delete a product
    pub async fn delete_product(&self, id: &str) -> RepoResult<()> {
        let thing = id.parse::<RecordId>()
            .map_err(|_| RepoError::Validation(format!("Invalid product ID: {}", id)))?;

        // Clean up attribute_binding edges
        self.db
            .query("DELETE attribute_binding WHERE in = $product")
            .bind(("product", thing.clone()))
            .await?;

        // Delete product
        let result: Option<Product> = self.db.delete(("product", thing.key().to_string())).await?;
        if result.is_none() {
            return Err(RepoError::NotFound(format!("Product {} not found", id)));
        }

        // Update cache
        {
            let mut cache = self.products.write().unwrap();
            cache.remove(id);
        }

        Ok(())
    }

    /// Add tag to product
    pub async fn add_product_tag(&self, product_id: &str, tag_id: &str) -> RepoResult<ProductFull> {
        let prod_thing = product_id.parse::<RecordId>()
            .map_err(|_| RepoError::Validation(format!("Invalid product ID: {}", product_id)))?;
        let tag_thing = tag_id.parse::<RecordId>()
            .map_err(|_| RepoError::Validation(format!("Invalid tag ID: {}", tag_id)))?;

        // Update in DB
        let mut result = self
            .db
            .query("UPDATE product SET tags += $tag WHERE id = $id RETURN AFTER")
            .bind(("id", prod_thing))
            .bind(("tag", tag_thing))
            .await?;
        let _products: Vec<Product> = result.take(0)?;

        // Refresh full product and update cache
        let full = self.fetch_product_full(product_id).await?;
        {
            let mut cache = self.products.write().unwrap();
            cache.insert(product_id.to_string(), full.clone());
        }

        Ok(full)
    }

    /// Remove tag from product
    pub async fn remove_product_tag(
        &self,
        product_id: &str,
        tag_id: &str,
    ) -> RepoResult<ProductFull> {
        let prod_thing = product_id.parse::<RecordId>()
            .map_err(|_| RepoError::Validation(format!("Invalid product ID: {}", product_id)))?;
        let tag_thing = tag_id.parse::<RecordId>()
            .map_err(|_| RepoError::Validation(format!("Invalid tag ID: {}", tag_id)))?;

        // Update in DB
        let mut result = self
            .db
            .query("UPDATE product SET tags -= $tag WHERE id = $id RETURN AFTER")
            .bind(("id", prod_thing))
            .bind(("tag", tag_thing))
            .await?;
        let _products: Vec<Product> = result.take(0)?;

        // Refresh full product and update cache
        let full = self.fetch_product_full(product_id).await?;
        {
            let mut cache = self.products.write().unwrap();
            cache.insert(product_id.to_string(), full.clone());
        }

        Ok(full)
    }

    /// Fetch full product data from DB (helper)
    async fn fetch_product_full(&self, product_id: &str) -> RepoResult<ProductFull> {
        let thing = product_id.parse::<RecordId>()
            .map_err(|_| RepoError::Validation(format!("Invalid product ID: {}", product_id)))?;

        // Fetch product with tags (using ProductWithTags to get full Tag objects)
        let mut result = self
            .db
            .query("SELECT * FROM product WHERE id = $id FETCH tags")
            .bind(("id", thing.clone()))
            .await?;
        let products: Vec<ProductWithTags> = result.take(0)?;
        let product = products
            .into_iter()
            .next()
            .ok_or_else(|| RepoError::NotFound(format!("Product {} not found", product_id)))?;

        // Fetch attribute bindings
        #[derive(Debug, Deserialize)]
        struct BindingRow {
            #[serde(default, with = "serde_helpers::option_record_id")]
            id: Option<RecordId>,
            #[serde(rename = "out")]
            to: Attribute,
            #[serde(default)]
            is_required: bool,
            #[serde(default)]
            display_order: i32,
            default_option_idx: Option<i32>,
        }

        let mut bindings_result = self
            .db
            .query("SELECT * FROM attribute_binding WHERE in = $product FETCH out")
            .bind(("product", thing))
            .await?;
        let bindings: Vec<BindingRow> = bindings_result.take(0)?;

        let attributes: Vec<AttributeBindingFull> = bindings
            .into_iter()
            .map(|b| AttributeBindingFull {
                id: b.id,
                attribute: b.to,
                is_required: b.is_required,
                display_order: b.display_order,
                default_option_idx: b.default_option_idx,
            })
            .collect();

        // Tags are already fetched as full Tag objects
        let tags = product.tags;

        Ok(ProductFull {
            id: product.id,
            name: product.name,
            image: product.image,
            category: product.category,
            sort_order: product.sort_order,
            tax_rate: product.tax_rate,
            receipt_name: product.receipt_name,
            kitchen_print_name: product.kitchen_print_name,
            kitchen_print_destinations: product.kitchen_print_destinations,
            label_print_destinations: product.label_print_destinations,
            is_kitchen_print_enabled: product.is_kitchen_print_enabled,
            is_label_print_enabled: product.is_label_print_enabled,
            is_active: product.is_active,
            specs: product.specs,
            attributes,
            tags,
        })
    }

    // =========================================================================
    // Category - Read (from cache)
    // =========================================================================

    /// Get category by ID (from cache)
    pub fn get_category(&self, id: &str) -> Option<Category> {
        let cache = self.categories.read().unwrap();
        cache.get(id).cloned()
    }

    /// List all categories (from cache)
    pub fn list_categories(&self) -> Vec<Category> {
        let cache = self.categories.read().unwrap();
        let mut categories: Vec<_> = cache.values().cloned().collect();
        categories.sort_by_key(|c| c.sort_order);
        categories
    }

    // =========================================================================
    // Category - Write (DB first, then cache)
    // =========================================================================

    /// Create a new category
    pub async fn create_category(&self, data: CategoryCreate) -> RepoResult<Category> {
        // Check duplicate name
        {
            let categories = self.categories.read().unwrap();
            if categories.values().any(|c| c.name == data.name) {
                return Err(RepoError::Duplicate(format!(
                    "Category '{}' already exists",
                    data.name
                )));
            }
        }

        let kitchen_print_destinations: Vec<RecordId> = data
            .kitchen_print_destinations
            .iter()
            .filter_map(|id| id.parse().ok())
            .collect();

        let label_print_destinations: Vec<RecordId> = data
            .label_print_destinations
            .iter()
            .filter_map(|id| id.parse().ok())
            .collect();

        let tag_ids: Vec<RecordId> = data
            .tag_ids
            .iter()
            .filter_map(|id| id.parse().ok())
            .collect();

        let category = Category {
            id: None,
            name: data.name,
            sort_order: data.sort_order.unwrap_or(0),
            kitchen_print_destinations,
            label_print_destinations,
            is_kitchen_print_enabled: data.is_kitchen_print_enabled.unwrap_or(true),
            is_label_print_enabled: data.is_label_print_enabled.unwrap_or(true),
            is_active: true,
            is_virtual: data.is_virtual.unwrap_or(false),
            tag_ids,
            match_mode: data.match_mode.unwrap_or_else(|| "any".to_string()),
            is_display: data.is_display.unwrap_or(true),
        };

        let created: Option<Category> = self.db.create("category").content(category).await?;
        let created =
            created.ok_or_else(|| RepoError::Database("Failed to create category".into()))?;

        // Update cache
        let category_id = created.id.as_ref().map(|t| t.to_string()).unwrap_or_default();
        {
            let mut cache = self.categories.write().unwrap();
            cache.insert(category_id, created.clone());
        }

        Ok(created)
    }

    /// Update a category
    pub async fn update_category(&self, id: &str, data: CategoryUpdate) -> RepoResult<Category> {
        let thing = id.parse::<RecordId>()
            .map_err(|_| RepoError::Validation(format!("Invalid category ID: {}", id)))?;

        // Check existing
        let existing = self
            .get_category(id)
            .ok_or_else(|| RepoError::NotFound(format!("Category {} not found", id)))?;

        // Check duplicate name if changing
        if let Some(ref new_name) = data.name
            && new_name != &existing.name
        {
            let categories = self.categories.read().unwrap();
            if categories.values().any(|c| &c.name == new_name) {
                return Err(RepoError::Duplicate(format!(
                    "Category '{}' already exists",
                    new_name
                )));
            }
        }

        #[derive(Serialize)]
        struct CategoryUpdateDb {
            #[serde(skip_serializing_if = "Option::is_none")]
            name: Option<String>,
            #[serde(skip_serializing_if = "Option::is_none")]
            sort_order: Option<i32>,
            #[serde(
                skip_serializing_if = "Option::is_none",
                with = "serde_helpers::option_vec_record_id"
            )]
            kitchen_print_destinations: Option<Vec<RecordId>>,
            #[serde(
                skip_serializing_if = "Option::is_none",
                with = "serde_helpers::option_vec_record_id"
            )]
            label_print_destinations: Option<Vec<RecordId>>,
            #[serde(skip_serializing_if = "Option::is_none")]
            is_kitchen_print_enabled: Option<bool>,
            #[serde(skip_serializing_if = "Option::is_none")]
            is_label_print_enabled: Option<bool>,
            #[serde(skip_serializing_if = "Option::is_none")]
            is_active: Option<bool>,
            #[serde(skip_serializing_if = "Option::is_none")]
            is_virtual: Option<bool>,
            #[serde(
                skip_serializing_if = "Option::is_none",
                with = "serde_helpers::option_vec_record_id"
            )]
            tag_ids: Option<Vec<RecordId>>,
            #[serde(skip_serializing_if = "Option::is_none")]
            match_mode: Option<String>,
            #[serde(skip_serializing_if = "Option::is_none")]
            is_display: Option<bool>,
        }

        let update_data = CategoryUpdateDb {
            name: data.name,
            sort_order: data.sort_order,
            kitchen_print_destinations: data
                .kitchen_print_destinations
                .map(|ids| ids.iter().filter_map(|id| id.parse().ok()).collect()),
            label_print_destinations: data
                .label_print_destinations
                .map(|ids| ids.iter().filter_map(|id| id.parse().ok()).collect()),
            is_kitchen_print_enabled: data.is_kitchen_print_enabled,
            is_label_print_enabled: data.is_label_print_enabled,
            is_active: data.is_active,
            is_virtual: data.is_virtual,
            tag_ids: data
                .tag_ids
                .map(|ids| ids.iter().filter_map(|id| id.parse().ok()).collect()),
            match_mode: data.match_mode,
            is_display: data.is_display,
        };

        self.db
            .query("UPDATE $thing MERGE $data")
            .bind(("thing", thing.clone()))
            .bind(("data", update_data))
            .await?;

        // Fetch updated category
        let updated: Option<Category> = self.db.select(("category", thing.key().to_string())).await?;
        let updated =
            updated.ok_or_else(|| RepoError::NotFound(format!("Category {} not found", id)))?;

        // Update cache
        {
            let mut cache = self.categories.write().unwrap();
            if updated.is_active {
                cache.insert(id.to_string(), updated.clone());
            } else {
                cache.remove(id);
            }
        }

        Ok(updated)
    }

    /// Delete a category
    pub async fn delete_category(&self, id: &str) -> RepoResult<()> {
        let cat_thing = id.parse::<RecordId>()
            .map_err(|_| RepoError::Validation(format!("Invalid category ID: {}", id)))?;

        // Check if category has products
        let mut result = self
            .db
            .query("SELECT count() FROM product WHERE category = $cat AND is_active = true GROUP ALL")
            .bind(("cat", cat_thing.clone()))
            .await?;
        let count: Option<i64> = result.take((0, "count"))?;

        if count.unwrap_or(0) > 0 {
            return Err(RepoError::Validation(
                "Cannot delete category with active products".into(),
            ));
        }

        // Clean up attribute_binding edges
        self.db
            .query("DELETE attribute_binding WHERE in = $category")
            .bind(("category", cat_thing.clone()))
            .await?;

        // Delete category
        self.db
            .query("DELETE $thing")
            .bind(("thing", cat_thing))
            .await?;

        // Update cache
        {
            let mut cache = self.categories.write().unwrap();
            cache.remove(id);
        }

        Ok(())
    }

    // =========================================================================
    // Convenience Methods (for price rules, printing, etc.)
    // =========================================================================

    /// Get product metadata for price rule matching
    pub fn get_product_meta(&self, product_id: &str) -> Option<ProductMeta> {
        let cache = self.products.read().unwrap();
        cache.get(product_id).map(|p| ProductMeta {
            category_id: p.category.to_string(),
            tags: p.tags.iter().filter_map(|t| t.id.as_ref()).map(|t| t.to_string()).collect(),
        })
    }

    /// Get product metadata for multiple products
    pub fn get_product_meta_batch(&self, product_ids: &[String]) -> HashMap<String, ProductMeta> {
        let cache = self.products.read().unwrap();
        product_ids
            .iter()
            .filter_map(|id| {
                cache.get(id).map(|p| {
                    (
                        id.clone(),
                        ProductMeta {
                            category_id: p.category.to_string(),
                            tags: p.tags.iter().filter_map(|t| t.id.as_ref()).map(|t| t.to_string()).collect(),
                        },
                    )
                })
            })
            .collect()
    }

    /// Get kitchen print configuration for a product (with fallback chain)
    ///
    /// Priority: product.is_kitchen_print_enabled > category.is_kitchen_print_enabled
    /// Destinations: product.destinations > category.destinations > global default
    pub fn get_kitchen_print_config(&self, product_id: &str) -> Option<KitchenPrintConfig> {
        let products = self.products.read().unwrap();
        let product = products.get(product_id)?;

        let categories = self.categories.read().unwrap();
        let category = categories.get(&product.category.to_string());

        // Determine if enabled (product > category)
        let enabled = match product.is_kitchen_print_enabled {
            1 => true,   // Explicitly enabled
            0 => false,  // Explicitly disabled
            _ => {       // -1: Inherit from category
                category
                    .filter(|c| !c.is_virtual)
                    .map(|c| c.is_kitchen_print_enabled)
                    .unwrap_or(false)
            }
        };

        if !enabled {
            return Some(KitchenPrintConfig {
                enabled: false,
                destinations: vec![],
                kitchen_name: None,
            });
        }

        // Determine destinations (product > category > global default)
        let destinations = if !product.kitchen_print_destinations.is_empty() {
            product.kitchen_print_destinations.iter().map(|t| t.to_string()).collect()
        } else if let Some(cat) = category.filter(|c| !c.is_virtual) {
            if !cat.kitchen_print_destinations.is_empty() {
                cat.kitchen_print_destinations.iter().map(|t| t.to_string()).collect()
            } else {
                let defaults = self.print_defaults.read().unwrap();
                defaults.kitchen_destination.iter().cloned().collect()
            }
        } else {
            let defaults = self.print_defaults.read().unwrap();
            defaults.kitchen_destination.iter().cloned().collect()
        };

        Some(KitchenPrintConfig {
            enabled,
            destinations,
            kitchen_name: product.kitchen_print_name.clone(),
        })
    }

    /// Get label print configuration for a product (with fallback chain)
    pub fn get_label_print_config(&self, product_id: &str) -> Option<LabelPrintConfig> {
        let products = self.products.read().unwrap();
        let product = products.get(product_id)?;

        let categories = self.categories.read().unwrap();
        let category = categories.get(&product.category.to_string());

        // Determine if enabled (product > category)
        let enabled = match product.is_label_print_enabled {
            1 => true,
            0 => false,
            _ => {
                category
                    .filter(|c| !c.is_virtual)
                    .map(|c| c.is_label_print_enabled)
                    .unwrap_or(false)
            }
        };

        if !enabled {
            return Some(LabelPrintConfig {
                enabled: false,
                destinations: vec![],
            });
        }

        // Determine destinations
        let destinations = if !product.label_print_destinations.is_empty() {
            product.label_print_destinations.iter().map(|t| t.to_string()).collect()
        } else if let Some(cat) = category.filter(|c| !c.is_virtual) {
            if !cat.label_print_destinations.is_empty() {
                cat.label_print_destinations.iter().map(|t| t.to_string()).collect()
            } else {
                let defaults = self.print_defaults.read().unwrap();
                defaults.label_destination.iter().cloned().collect()
            }
        } else {
            let defaults = self.print_defaults.read().unwrap();
            defaults.label_destination.iter().cloned().collect()
        };

        Some(LabelPrintConfig {
            enabled,
            destinations,
        })
    }

    /// Check if kitchen printing is enabled (system level)
    pub fn is_kitchen_print_enabled(&self) -> bool {
        let defaults = self.print_defaults.read().unwrap();
        defaults.kitchen_destination.is_some()
    }

    /// Check if label printing is enabled (system level)
    pub fn is_label_print_enabled(&self) -> bool {
        let defaults = self.print_defaults.read().unwrap();
        defaults.label_destination.is_some()
    }
}
