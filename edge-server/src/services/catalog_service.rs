//! Catalog Service - Unified Product and Category management with in-memory caching
//!
//! Replaces:
//! - ProductRepository
//! - CategoryRepository
//! - OrdersManager.product_meta_cache
//! - PrintConfigCache (product/category parts)
//! - PriceRuleEngine DB queries

use crate::db::repository::{attribute, image_ref, RepoError, RepoResult};
use super::ImageCleanupService;
use shared::models::{
    AttributeBindingFull, Category, CategoryCreate, CategoryUpdate,
    ImageRefEntityType, Product, ProductCreate, ProductFull, ProductSpec, ProductUpdate, Tag,
};
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;

// =============================================================================
// Types
// =============================================================================

/// Product metadata for price rule matching and tax calculation
#[derive(Debug, Clone, Default)]
pub struct ProductMeta {
    pub category_id: i64,
    pub category_name: String,
    pub tags: Vec<i64>,
    pub tax_rate: i32,
}

/// Kitchen print configuration (computed result with fallback chain applied)
#[derive(Debug, Clone)]
pub struct KitchenPrintConfig {
    pub enabled: bool,
    pub destinations: Vec<String>, // ["1", "2", ...] (print_destination i64 IDs as strings)
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
    pool: SqlitePool,
    /// Products cache: "42" -> ProductFull
    products: Arc<RwLock<HashMap<String, ProductFull>>>,
    /// Categories cache: "42" -> Category
    categories: Arc<RwLock<HashMap<String, Category>>>,
    /// System default print destinations
    print_defaults: Arc<RwLock<PrintDefaults>>,
    /// Image cleanup service
    image_cleanup: ImageCleanupService,
}

impl std::fmt::Debug for CatalogService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let products_count = self.products.read().len();
        let categories_count = self.categories.read().len();
        f.debug_struct("CatalogService")
            .field("products_count", &products_count)
            .field("categories_count", &categories_count)
            .finish()
    }
}

impl CatalogService {
    /// Create a new CatalogService
    ///
    /// `images_dir` is the path to the images directory: {tenant}/server/images/
    pub fn new(pool: SqlitePool, images_dir: std::path::PathBuf) -> Self {
        Self {
            image_cleanup: ImageCleanupService::new(images_dir),
            pool,
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
        let categories: Vec<Category> = sqlx::query_as(
            "SELECT id, name, sort_order, is_kitchen_print_enabled, is_label_print_enabled, is_active, is_virtual, match_mode, is_display FROM category WHERE is_active = 1 ORDER BY sort_order",
        )
        .fetch_all(&self.pool)
        .await?;

        // Load category relations (junction tables)
        let mut cat_map: HashMap<String, Category> = HashMap::new();
        for mut cat in categories {
            let cat_id = cat.id;

            // Kitchen print destinations
            cat.kitchen_print_destinations = sqlx::query_scalar!(
                "SELECT print_destination_id FROM category_kitchen_print_dest WHERE category_id = ?",
                cat_id
            )
            .fetch_all(&self.pool)
            .await
            .unwrap_or_default();

            // Label print destinations
            cat.label_print_destinations = sqlx::query_scalar!(
                "SELECT print_destination_id FROM category_label_print_dest WHERE category_id = ?",
                cat_id
            )
            .fetch_all(&self.pool)
            .await
            .unwrap_or_default();

            // Tag IDs for virtual categories
            cat.tag_ids = sqlx::query_scalar!(
                "SELECT tag_id FROM category_tag WHERE category_id = ?",
                cat_id
            )
            .fetch_all(&self.pool)
            .await
            .unwrap_or_default();

            cat_map.insert(cat_id.to_string(), cat);
        }

        let categories_count = cat_map.len();
        {
            let mut cache = self.categories.write();
            *cache = cat_map;
        }
        tracing::debug!(count = categories_count, "CatalogService loaded categories");

        // 2. Load all active products
        let products: Vec<Product> = sqlx::query_as(
            "SELECT id, name, image, category_id, sort_order, tax_rate, receipt_name, kitchen_print_name, is_kitchen_print_enabled, is_label_print_enabled, is_active, external_id FROM product WHERE is_active = 1 ORDER BY sort_order",
        )
        .fetch_all(&self.pool)
        .await?;

        // 3. Load all attribute bindings with full attribute data
        //    (for all active products and categories)
        let all_bindings = attribute::find_all_bindings_with_attributes(&self.pool).await;

        // Group bindings by (owner_type, owner_id)
        let mut product_bindings: HashMap<i64, Vec<AttributeBindingFull>> = HashMap::new();
        let mut category_bindings: HashMap<i64, Vec<AttributeBindingFull>> = HashMap::new();

        if let Ok(bindings) = all_bindings {
            for (binding, attr) in bindings {
                let full = AttributeBindingFull {
                    id: binding.id,
                    attribute: attr,
                    is_required: binding.is_required,
                    display_order: binding.display_order,
                    default_option_indices: binding.default_option_indices,
                    is_inherited: false,
                };
                if binding.owner_type == "product" {
                    product_bindings.entry(binding.owner_id).or_default().push(full);
                } else if binding.owner_type == "category" {
                    category_bindings.entry(binding.owner_id).or_default().push(full);
                }
            }
        }

        tracing::debug!(
            product_bindings = product_bindings.len(),
            category_bindings = category_bindings.len(),
            "CatalogService loaded attribute bindings"
        );

        // 4. Build ProductFull (outside lock to avoid holding guard across .await)
        let mut built_products = HashMap::new();

        for product in products {
            let product_id = product.id;

            // Load tags
            let tags: Vec<Tag> = sqlx::query_as(
                "SELECT t.id, t.name, t.color, t.display_order, t.is_active, t.is_system FROM tag t JOIN product_tag pt ON t.id = pt.tag_id WHERE pt.product_id = ? AND t.is_active = 1",
            )
            .bind(product_id)
            .fetch_all(&self.pool)
            .await
            .unwrap_or_default();

            // Load specs
            let specs: Vec<ProductSpec> = sqlx::query_as(
                "SELECT id, product_id, name, price, display_order, is_default, is_active, receipt_name, is_root FROM product_spec WHERE product_id = ? AND is_active = 1 ORDER BY display_order",
            )
            .bind(product_id)
            .fetch_all(&self.pool)
            .await
            .unwrap_or_default();

            // Merge: category inherited attributes + product direct attributes
            let mut attributes = product_bindings
                .remove(&product_id)
                .unwrap_or_default();

            // Collect product's own attribute IDs for dedup
            let product_attr_ids: std::collections::HashSet<i64> = attributes
                .iter()
                .map(|b| b.attribute.id)
                .collect();

            // Add inherited category attributes (skip if product already has direct binding)
            if let Some(cat_bindings) = category_bindings.get(&product.category_id) {
                for cb in cat_bindings {
                    if !product_attr_ids.contains(&cb.attribute.id) {
                        let mut inherited = cb.clone();
                        inherited.is_inherited = true;
                        attributes.push(inherited);
                    }
                }
            }

            let full = ProductFull {
                id: product.id,
                name: product.name,
                image: product.image,
                category_id: product.category_id,
                sort_order: product.sort_order,
                tax_rate: product.tax_rate,
                receipt_name: product.receipt_name,
                kitchen_print_name: product.kitchen_print_name,
                is_kitchen_print_enabled: product.is_kitchen_print_enabled,
                is_label_print_enabled: product.is_label_print_enabled,
                is_active: product.is_active,
                external_id: product.external_id,
                specs,
                attributes,
                tags,
            };

            built_products.insert(product_id.to_string(), full);
        }

        // 5. Store in cache (short lock scope, no await)
        {
            let mut cache = self.products.write();
            *cache = built_products;
        }

        let products_count = self.products.read().len();
        tracing::debug!(count = products_count, "CatalogService loaded products");

        Ok(())
    }

    /// Set system default print destinations
    pub fn set_print_defaults(&self, kitchen: Option<String>, label: Option<String>) {
        let mut defaults = self.print_defaults.write();
        defaults.kitchen_destination = kitchen;
        defaults.label_destination = label;
    }

    /// Get system default print destinations
    pub fn get_print_defaults(&self) -> PrintDefaults {
        self.print_defaults.read().clone()
    }

    // =========================================================================
    // Product - Read (from cache)
    // =========================================================================

    /// Get product by ID (from cache)
    pub fn get_product(&self, id: &str) -> Option<ProductFull> {
        let cache = self.products.read();
        cache.get(id).cloned()
    }

    /// List all products (from cache)
    pub fn list_products(&self) -> Vec<ProductFull> {
        let cache = self.products.read();
        let mut products: Vec<_> = cache.values().cloned().collect();
        products.sort_by_key(|p| p.sort_order);
        products
    }

    /// Get products by category ID (from cache)
    pub fn get_products_by_category(&self, category_id: &str) -> Vec<ProductFull> {
        let cache = self.products.read();
        let mut products: Vec<_> = cache
            .values()
            .filter(|p| p.category_id.to_string() == category_id)
            .cloned()
            .collect();
        products.sort_by_key(|p| p.sort_order);
        products
    }

    /// Refresh a single product's cache entry
    pub async fn refresh_product_cache(&self, product_id: &str) -> RepoResult<()> {
        let full = self.fetch_product_full(product_id).await?;
        let mut cache = self.products.write();
        cache.insert(product_id.to_string(), full);
        Ok(())
    }

    /// Refresh cached products in a category (re-fetch from DB to pick up inherited attribute changes)
    pub async fn refresh_products_in_category(&self, category_id: &str) -> RepoResult<()> {
        let product_ids: Vec<String> = {
            let cache = self.products.read();
            cache
                .iter()
                .filter(|(_, p)| p.category_id.to_string() == category_id)
                .map(|(id, _)| id.clone())
                .collect()
        };

        for product_id in product_ids {
            let full = self.fetch_product_full(&product_id).await?;
            let mut cache = self.products.write();
            cache.insert(product_id, full);
        }

        Ok(())
    }

    /// Refresh cached products that reference a given attribute (direct or inherited)
    pub async fn refresh_products_with_attribute(&self, attribute_id: &str) -> RepoResult<()> {
        let attr_id: i64 = attribute_id
            .parse()
            .map_err(|_| RepoError::Validation(format!("Invalid attribute ID: {}", attribute_id)))?;

        let product_ids: Vec<String> = {
            let cache = self.products.read();
            cache
                .iter()
                .filter(|(_, p)| {
                    p.attributes.iter().any(|b| b.attribute.id == attr_id)
                })
                .map(|(id, _)| id.clone())
                .collect()
        };

        for product_id in product_ids {
            let full = self.fetch_product_full(&product_id).await?;
            let mut cache = self.products.write();
            cache.insert(product_id, full);
        }

        Ok(())
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
            let categories = self.categories.read();
            let cat_id = data.category_id.to_string();
            if let Some(cat) = categories.get(&cat_id)
                && cat.is_virtual
            {
                return Err(RepoError::Validation(
                    "Product cannot belong to a virtual category".into(),
                ));
            }
        }

        // Insert product
        let image = data.image.as_deref().unwrap_or("");
        let sort_order = data.sort_order.unwrap_or(0);
        let tax_rate = data.tax_rate.unwrap_or(0);
        let is_kitchen_print_enabled = data.is_kitchen_print_enabled.unwrap_or(-1);
        let is_label_print_enabled = data.is_label_print_enabled.unwrap_or(-1);
        let product_id = sqlx::query_scalar!(
            r#"INSERT INTO product (name, image, category_id, sort_order, tax_rate, receipt_name, kitchen_print_name, is_kitchen_print_enabled, is_label_print_enabled, is_active, external_id) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 1, ?10) RETURNING id as "id!""#,
            data.name,
            image,
            data.category_id,
            sort_order,
            tax_rate,
            data.receipt_name,
            data.kitchen_print_name,
            is_kitchen_print_enabled,
            is_label_print_enabled,
            data.external_id,
        )
        .fetch_one(&self.pool)
        .await?;

        // Insert specs
        for spec in &data.specs {
            sqlx::query!(
                "INSERT INTO product_spec (product_id, name, price, display_order, is_default, is_active, receipt_name, is_root) VALUES (?1, ?2, ?3, ?4, ?5, 1, ?6, ?7)",
                product_id,
                spec.name,
                spec.price,
                spec.display_order,
                spec.is_default,
                spec.receipt_name,
                spec.is_root,
            )
            .execute(&self.pool)
            .await?;
        }

        // Insert tags (junction table)
        if let Some(ref tag_ids) = data.tags {
            for tag_id in tag_ids {
                sqlx::query!("INSERT OR IGNORE INTO product_tag (product_id, tag_id) VALUES (?, ?)",
                    product_id,
                    tag_id,
                )
                .execute(&self.pool)
                .await?;
            }
        }

        // Fetch the created product with all relations
        let product_id_str = product_id.to_string();
        let full = self.fetch_product_full(&product_id_str).await?;

        // Sync image references
        let image_hashes = Self::extract_product_image_hashes(&full);
        let _ = image_ref::sync_refs(
            &self.pool,
            ImageRefEntityType::Product,
            &product_id_str,
            image_hashes,
        )
        .await;

        // Update cache
        {
            let mut cache = self.products.write();
            cache.insert(product_id_str, full.clone());
        }

        Ok(full)
    }

    /// Update a product
    pub async fn update_product(&self, id: &str, data: ProductUpdate) -> RepoResult<ProductFull> {
        let product_id: i64 = id
            .parse()
            .map_err(|_| RepoError::Validation(format!("Invalid product ID: {}", id)))?;

        // Check if there's anything to update
        let has_scalar_updates = data.name.is_some()
            || data.image.is_some()
            || data.category_id.is_some()
            || data.sort_order.is_some()
            || data.tax_rate.is_some()
            || data.receipt_name.is_some()
            || data.kitchen_print_name.is_some()
            || data.is_kitchen_print_enabled.is_some()
            || data.is_label_print_enabled.is_some()
            || data.is_active.is_some()
            || data.external_id.is_some();

        if !has_scalar_updates && data.tags.is_none() && data.specs.is_none() {
            return self.get_product(id)
                .ok_or_else(|| RepoError::NotFound(format!("Product {} not found", id)));
        }

        // Validate category if changing
        if let Some(new_cat_id) = data.category_id {
            let categories = self.categories.read();
            let cat_id = new_cat_id.to_string();
            if let Some(cat) = categories.get(&cat_id)
                && cat.is_virtual
            {
                return Err(RepoError::Validation(
                    "Product cannot belong to a virtual category".into(),
                ));
            }
        }

        // Execute update of scalar fields using COALESCE pattern
        if has_scalar_updates {
            sqlx::query!(
                "UPDATE product SET name = COALESCE(?1, name), image = COALESCE(?2, image), category_id = COALESCE(?3, category_id), sort_order = COALESCE(?4, sort_order), tax_rate = COALESCE(?5, tax_rate), receipt_name = COALESCE(?6, receipt_name), kitchen_print_name = COALESCE(?7, kitchen_print_name), is_kitchen_print_enabled = COALESCE(?8, is_kitchen_print_enabled), is_label_print_enabled = COALESCE(?9, is_label_print_enabled), is_active = COALESCE(?10, is_active), external_id = COALESCE(?11, external_id) WHERE id = ?12",
                data.name,
                data.image,
                data.category_id,
                data.sort_order,
                data.tax_rate,
                data.receipt_name,
                data.kitchen_print_name,
                data.is_kitchen_print_enabled,
                data.is_label_print_enabled,
                data.is_active,
                data.external_id,
                product_id,
            )
            .execute(&self.pool)
            .await?;
        }

        // Replace tags if provided
        if let Some(ref tag_ids) = data.tags {
            sqlx::query!("DELETE FROM product_tag WHERE product_id = ?", product_id)
                .execute(&self.pool)
                .await?;
            for tag_id in tag_ids {
                sqlx::query!("INSERT OR IGNORE INTO product_tag (product_id, tag_id) VALUES (?, ?)",
                    product_id,
                    tag_id,
                )
                .execute(&self.pool)
                .await?;
            }
        }

        // Replace specs if provided
        if let Some(ref specs) = data.specs {
            sqlx::query!("DELETE FROM product_spec WHERE product_id = ?", product_id)
                .execute(&self.pool)
                .await?;
            for spec in specs {
                sqlx::query!(
                    "INSERT INTO product_spec (product_id, name, price, display_order, is_default, is_active, receipt_name, is_root) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                    product_id,
                    spec.name,
                    spec.price,
                    spec.display_order,
                    spec.is_default,
                    spec.is_active,
                    spec.receipt_name,
                    spec.is_root,
                )
                .execute(&self.pool)
                .await?;
            }
        }

        // Fetch full product data
        let full = self.fetch_product_full(id).await?;

        // Sync image references and cleanup orphans
        let image_hashes = Self::extract_product_image_hashes(&full);
        let removed_hashes = image_ref::sync_refs(
            &self.pool,
            ImageRefEntityType::Product,
            id,
            image_hashes,
        )
        .await
        .unwrap_or_default();

        // Cleanup orphan images (do this after transaction committed)
        if !removed_hashes.is_empty() {
            let orphans = image_ref::find_orphan_hashes(&self.pool, &removed_hashes)
                .await
                .unwrap_or_default();
            self.image_cleanup.cleanup_orphan_images(&orphans).await;
        }

        // Update cache
        {
            let mut cache = self.products.write();
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
        let product_id: i64 = id
            .parse()
            .map_err(|_| RepoError::Validation(format!("Invalid product ID: {}", id)))?;

        // Get image references before deleting
        let image_hashes = image_ref::delete_entity_refs(
            &self.pool,
            ImageRefEntityType::Product,
            id,
        )
        .await
        .unwrap_or_default();

        // Clean up attribute bindings
        sqlx::query!("DELETE FROM attribute_binding WHERE owner_type = 'product' AND owner_id = ?", product_id)
            .execute(&self.pool)
            .await?;

        // Clean up tag bindings
        sqlx::query!("DELETE FROM product_tag WHERE product_id = ?", product_id)
            .execute(&self.pool)
            .await?;

        // Delete specs
        sqlx::query!("DELETE FROM product_spec WHERE product_id = ?", product_id)
            .execute(&self.pool)
            .await?;

        // Delete product
        let result = sqlx::query!("DELETE FROM product WHERE id = ?", product_id)
            .execute(&self.pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(RepoError::NotFound(format!("Product {} not found", id)));
        }

        // Update cache
        {
            let mut cache = self.products.write();
            cache.remove(id);
        }

        // Cleanup orphan images (after transaction committed)
        if !image_hashes.is_empty() {
            let orphans = image_ref::find_orphan_hashes(&self.pool, &image_hashes)
                .await
                .unwrap_or_default();
            self.image_cleanup.cleanup_orphan_images(&orphans).await;
        }

        Ok(())
    }

    /// Add tag to product
    pub async fn add_product_tag(&self, product_id: &str, tag_id: &str) -> RepoResult<ProductFull> {
        let pid: i64 = product_id
            .parse()
            .map_err(|_| RepoError::Validation(format!("Invalid product ID: {}", product_id)))?;
        let tid: i64 = tag_id
            .parse()
            .map_err(|_| RepoError::Validation(format!("Invalid tag ID: {}", tag_id)))?;

        // Insert into junction table (ignore if already exists)
        sqlx::query!("INSERT OR IGNORE INTO product_tag (product_id, tag_id) VALUES (?, ?)", pid, tid)
            .execute(&self.pool)
            .await?;

        // Refresh full product and update cache
        let full = self.fetch_product_full(product_id).await?;
        {
            let mut cache = self.products.write();
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
        let pid: i64 = product_id
            .parse()
            .map_err(|_| RepoError::Validation(format!("Invalid product ID: {}", product_id)))?;
        let tid: i64 = tag_id
            .parse()
            .map_err(|_| RepoError::Validation(format!("Invalid tag ID: {}", tag_id)))?;

        // Delete from junction table
        sqlx::query!("DELETE FROM product_tag WHERE product_id = ? AND tag_id = ?", pid, tid)
            .execute(&self.pool)
            .await?;

        // Refresh full product and update cache
        let full = self.fetch_product_full(product_id).await?;
        {
            let mut cache = self.products.write();
            cache.insert(product_id.to_string(), full.clone());
        }

        Ok(full)
    }

    /// Fetch full product data from DB (helper)
    async fn fetch_product_full(&self, product_id: &str) -> RepoResult<ProductFull> {
        let pid: i64 = product_id
            .parse()
            .map_err(|_| RepoError::Validation(format!("Invalid product ID: {}", product_id)))?;

        // Fetch product
        let product: Product = sqlx::query_as(
            "SELECT id, name, image, category_id, sort_order, tax_rate, receipt_name, kitchen_print_name, is_kitchen_print_enabled, is_label_print_enabled, is_active, external_id FROM product WHERE id = ?",
        )
        .bind(pid)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| RepoError::NotFound(format!("Product {} not found", product_id)))?;

        // Fetch tags
        let tags: Vec<Tag> = sqlx::query_as(
            "SELECT t.id, t.name, t.color, t.display_order, t.is_active, t.is_system FROM tag t JOIN product_tag pt ON t.id = pt.tag_id WHERE pt.product_id = ? AND t.is_active = 1",
        )
        .bind(pid)
        .fetch_all(&self.pool)
        .await
        .unwrap_or_default();

        // Fetch specs
        let specs: Vec<ProductSpec> = sqlx::query_as(
            "SELECT id, product_id, name, price, display_order, is_default, is_active, receipt_name, is_root FROM product_spec WHERE product_id = ? ORDER BY display_order",
        )
        .bind(pid)
        .fetch_all(&self.pool)
        .await
        .unwrap_or_default();

        // Fetch product's direct attribute bindings
        let product_binding_pairs = attribute::find_bindings_for_owner(&self.pool, "product", pid).await?;
        let mut attributes: Vec<AttributeBindingFull> = product_binding_pairs
            .into_iter()
            .map(|(binding, attr)| AttributeBindingFull {
                id: binding.id,
                attribute: attr,
                is_required: binding.is_required,
                display_order: binding.display_order,
                default_option_indices: binding.default_option_indices,
                is_inherited: false,
            })
            .collect();

        // Merge inherited category attributes
        let cat_binding_pairs = attribute::find_bindings_for_owner(&self.pool, "category", product.category_id).await?;
        let product_attr_ids: std::collections::HashSet<i64> = attributes
            .iter()
            .map(|b| b.attribute.id)
            .collect();

        for (binding, attr) in cat_binding_pairs {
            if !product_attr_ids.contains(&attr.id) {
                attributes.push(AttributeBindingFull {
                    id: binding.id,
                    attribute: attr,
                    is_required: binding.is_required,
                    display_order: binding.display_order,
                    default_option_indices: binding.default_option_indices,
                    is_inherited: true,
                });
            }
        }

        Ok(ProductFull {
            id: product.id,
            name: product.name,
            image: product.image,
            category_id: product.category_id,
            sort_order: product.sort_order,
            tax_rate: product.tax_rate,
            receipt_name: product.receipt_name,
            kitchen_print_name: product.kitchen_print_name,
            is_kitchen_print_enabled: product.is_kitchen_print_enabled,
            is_label_print_enabled: product.is_label_print_enabled,
            is_active: product.is_active,
            external_id: product.external_id,
            specs,
            attributes,
            tags,
        })
    }

    /// Extract image hashes from a product
    ///
    /// Product only has a single image field, so return a set with 0 or 1 hash.
    fn extract_product_image_hashes(product: &ProductFull) -> std::collections::HashSet<String> {
        let mut hashes = std::collections::HashSet::new();
        if !product.image.is_empty() {
            hashes.insert(product.image.clone());
        }
        hashes
    }

    // =========================================================================
    // Category - Read (from cache)
    // =========================================================================

    /// Get category by ID (from cache)
    pub fn get_category(&self, id: &str) -> Option<Category> {
        let cache = self.categories.read();
        cache.get(id).cloned()
    }

    /// List all categories (from cache)
    pub fn list_categories(&self) -> Vec<Category> {
        let cache = self.categories.read();
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
            let categories = self.categories.read();
            if categories.values().any(|c| c.name == data.name) {
                return Err(RepoError::Duplicate(format!(
                    "Category '{}' already exists",
                    data.name
                )));
            }
        }

        let sort_order = data.sort_order.unwrap_or(0);
        let is_kitchen_print_enabled = data.is_kitchen_print_enabled.unwrap_or(false);
        let is_label_print_enabled = data.is_label_print_enabled.unwrap_or(false);
        let is_virtual = data.is_virtual.unwrap_or(false);
        let match_mode = data.match_mode.as_deref().unwrap_or("any");
        let is_display = data.is_display.unwrap_or(true);
        let category_id = sqlx::query_scalar!(
            r#"INSERT INTO category (name, sort_order, is_kitchen_print_enabled, is_label_print_enabled, is_active, is_virtual, match_mode, is_display) VALUES (?1, ?2, ?3, ?4, 1, ?5, ?6, ?7) RETURNING id as "id!""#,
            data.name,
            sort_order,
            is_kitchen_print_enabled,
            is_label_print_enabled,
            is_virtual,
            match_mode,
            is_display,
        )
        .fetch_one(&self.pool)
        .await?;

        // Insert kitchen print destinations
        for dest_id in &data.kitchen_print_destinations {
            sqlx::query!("INSERT OR IGNORE INTO category_kitchen_print_dest (category_id, print_destination_id) VALUES (?, ?)",
                category_id, dest_id,
            )
            .execute(&self.pool)
            .await?;
        }

        // Insert label print destinations
        for dest_id in &data.label_print_destinations {
            sqlx::query!("INSERT OR IGNORE INTO category_label_print_dest (category_id, print_destination_id) VALUES (?, ?)",
                category_id, dest_id,
            )
            .execute(&self.pool)
            .await?;
        }

        // Insert tag IDs
        for tag_id in &data.tag_ids {
            sqlx::query!("INSERT OR IGNORE INTO category_tag (category_id, tag_id) VALUES (?, ?)",
                category_id, tag_id,
            )
            .execute(&self.pool)
            .await?;
        }

        // Fetch back the full category
        let created = self.fetch_category_full(category_id).await?;

        // Update cache
        {
            let mut cache = self.categories.write();
            cache.insert(category_id.to_string(), created.clone());
        }

        Ok(created)
    }

    /// Update a category
    pub async fn update_category(&self, id: &str, data: CategoryUpdate) -> RepoResult<Category> {
        let category_id: i64 = id
            .parse()
            .map_err(|_| RepoError::Validation(format!("Invalid category ID: {}", id)))?;

        // Check existing
        let existing = self
            .get_category(id)
            .ok_or_else(|| RepoError::NotFound(format!("Category {} not found", id)))?;

        // Check duplicate name if changing
        if let Some(ref new_name) = data.name
            && new_name != &existing.name
        {
            let categories = self.categories.read();
            if categories.values().any(|c| &c.name == new_name) {
                return Err(RepoError::Duplicate(format!(
                    "Category '{}' already exists",
                    new_name
                )));
            }
        }

        // Update scalar fields using COALESCE
        sqlx::query!(
            "UPDATE category SET name = COALESCE(?1, name), sort_order = COALESCE(?2, sort_order), is_kitchen_print_enabled = COALESCE(?3, is_kitchen_print_enabled), is_label_print_enabled = COALESCE(?4, is_label_print_enabled), is_virtual = COALESCE(?5, is_virtual), match_mode = COALESCE(?6, match_mode), is_display = COALESCE(?7, is_display), is_active = COALESCE(?8, is_active) WHERE id = ?9",
            data.name,
            data.sort_order,
            data.is_kitchen_print_enabled,
            data.is_label_print_enabled,
            data.is_virtual,
            data.match_mode,
            data.is_display,
            data.is_active,
            category_id,
        )
        .execute(&self.pool)
        .await?;

        // Replace kitchen print destinations if provided
        if let Some(ref dests) = data.kitchen_print_destinations {
            sqlx::query!("DELETE FROM category_kitchen_print_dest WHERE category_id = ?", category_id)
                .execute(&self.pool)
                .await?;
            for dest_id in dests {
                sqlx::query!("INSERT OR IGNORE INTO category_kitchen_print_dest (category_id, print_destination_id) VALUES (?, ?)",
                    category_id, dest_id,
                )
                .execute(&self.pool)
                .await?;
            }
        }

        // Replace label print destinations if provided
        if let Some(ref dests) = data.label_print_destinations {
            sqlx::query!("DELETE FROM category_label_print_dest WHERE category_id = ?", category_id)
                .execute(&self.pool)
                .await?;
            for dest_id in dests {
                sqlx::query!("INSERT OR IGNORE INTO category_label_print_dest (category_id, print_destination_id) VALUES (?, ?)",
                    category_id, dest_id,
                )
                .execute(&self.pool)
                .await?;
            }
        }

        // Replace tag IDs if provided
        if let Some(ref tag_ids) = data.tag_ids {
            sqlx::query!("DELETE FROM category_tag WHERE category_id = ?", category_id)
                .execute(&self.pool)
                .await?;
            for tag_id in tag_ids {
                sqlx::query!("INSERT OR IGNORE INTO category_tag (category_id, tag_id) VALUES (?, ?)",
                    category_id, tag_id,
                )
                .execute(&self.pool)
                .await?;
            }
        }

        // Fetch back the full category
        let updated = self.fetch_category_full(category_id).await?;

        // Update cache
        {
            let mut cache = self.categories.write();
            cache.insert(id.to_string(), updated.clone());
        }

        Ok(updated)
    }

    /// Delete a category
    pub async fn delete_category(&self, id: &str) -> RepoResult<()> {
        let category_id: i64 = id
            .parse()
            .map_err(|_| RepoError::Validation(format!("Invalid category ID: {}", id)))?;

        // Check if category has products
        let count = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM product WHERE category_id = ? AND is_active = 1",
            category_id,
        )
        .fetch_one(&self.pool)
        .await?;

        if count > 0 {
            return Err(RepoError::Validation(
                "Cannot delete category with active products".into(),
            ));
        }

        // Clean up attribute bindings
        sqlx::query!("DELETE FROM attribute_binding WHERE owner_type = 'category' AND owner_id = ?", category_id)
            .execute(&self.pool)
            .await?;

        // Clean up junction tables
        sqlx::query!("DELETE FROM category_kitchen_print_dest WHERE category_id = ?", category_id)
            .execute(&self.pool)
            .await?;
        sqlx::query!("DELETE FROM category_label_print_dest WHERE category_id = ?", category_id)
            .execute(&self.pool)
            .await?;
        sqlx::query!("DELETE FROM category_tag WHERE category_id = ?", category_id)
            .execute(&self.pool)
            .await?;

        // Delete category
        sqlx::query!("DELETE FROM category WHERE id = ?", category_id)
            .execute(&self.pool)
            .await?;

        // Update cache
        {
            let mut cache = self.categories.write();
            cache.remove(id);
        }

        Ok(())
    }

    /// Fetch a category with all its relations from DB (helper)
    async fn fetch_category_full(&self, category_id: i64) -> RepoResult<Category> {
        let mut cat: Category = sqlx::query_as(
            "SELECT id, name, sort_order, is_kitchen_print_enabled, is_label_print_enabled, is_active, is_virtual, match_mode, is_display FROM category WHERE id = ?",
        )
        .bind(category_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| RepoError::NotFound(format!("Category {} not found", category_id)))?;

        cat.kitchen_print_destinations = sqlx::query_scalar!(
            "SELECT print_destination_id FROM category_kitchen_print_dest WHERE category_id = ?",
            category_id,
        )
        .fetch_all(&self.pool)
        .await
        .unwrap_or_default();

        cat.label_print_destinations = sqlx::query_scalar!(
            "SELECT print_destination_id FROM category_label_print_dest WHERE category_id = ?",
            category_id,
        )
        .fetch_all(&self.pool)
        .await
        .unwrap_or_default();

        cat.tag_ids = sqlx::query_scalar!(
            "SELECT tag_id FROM category_tag WHERE category_id = ?",
            category_id,
        )
        .fetch_all(&self.pool)
        .await
        .unwrap_or_default();

        Ok(cat)
    }

    // =========================================================================
    // Convenience Methods (for price rules, printing, etc.)
    // =========================================================================

    /// Get product metadata for price rule matching
    pub fn get_product_meta(&self, product_id: &str) -> Option<ProductMeta> {
        let cache = self.products.read();
        cache.get(product_id).map(|p| {
            let cat_key = p.category_id.to_string();
            let category_name = {
                let cat_cache = self.categories.read();
                cat_cache.get(&cat_key).map(|c| c.name.clone()).unwrap_or_default()
            };
            ProductMeta {
                category_id: p.category_id,
                category_name,
                tags: p.tags.iter().map(|t| t.id).collect(),
                tax_rate: p.tax_rate,
            }
        })
    }

    /// Get product metadata for multiple products
    pub fn get_product_meta_batch(&self, product_ids: &[i64]) -> HashMap<i64, ProductMeta> {
        let cache = self.products.read();
        let cat_cache = self.categories.read();
        product_ids
            .iter()
            .filter_map(|&id| {
                let key = id.to_string();
                cache.get(&key).map(|p| {
                    let cat_key = p.category_id.to_string();
                    let category_name = cat_cache.get(&cat_key).map(|c| c.name.clone()).unwrap_or_default();
                    (
                        id,
                        ProductMeta {
                            category_id: p.category_id,
                            category_name,
                            tags: p.tags.iter().map(|t| t.id).collect(),
                            tax_rate: p.tax_rate,
                        },
                    )
                })
            })
            .collect()
    }

    /// Get kitchen print configuration for a product (with fallback chain)
    ///
    /// Priority: product.is_kitchen_print_enabled > category.is_kitchen_print_enabled
    /// Destinations: category.destinations > global default
    pub fn get_kitchen_print_config(&self, product_id: &str) -> Option<KitchenPrintConfig> {
        let products = self.products.read();
        let product = products.get(product_id)?;

        let categories = self.categories.read();
        let category = categories.get(&product.category_id.to_string());

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

        // Determine destinations (category > global default)
        let destinations = if let Some(cat) = category.filter(|c| !c.is_virtual) {
            if !cat.kitchen_print_destinations.is_empty() {
                cat.kitchen_print_destinations.iter().map(|id| id.to_string()).collect()
            } else {
                let defaults = self.print_defaults.read();
                defaults.kitchen_destination.iter().cloned().collect()
            }
        } else {
            let defaults = self.print_defaults.read();
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
        let products = self.products.read();
        let product = products.get(product_id)?;

        let categories = self.categories.read();
        let category = categories.get(&product.category_id.to_string());

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

        // Determine destinations (category > global default)
        let destinations = if let Some(cat) = category.filter(|c| !c.is_virtual) {
            if !cat.label_print_destinations.is_empty() {
                cat.label_print_destinations.iter().map(|id| id.to_string()).collect()
            } else {
                let defaults = self.print_defaults.read();
                defaults.label_destination.iter().cloned().collect()
            }
        } else {
            let defaults = self.print_defaults.read();
            defaults.label_destination.iter().cloned().collect()
        };

        Some(LabelPrintConfig {
            enabled,
            destinations,
        })
    }

    /// Check if kitchen printing is enabled (system level)
    pub fn is_kitchen_print_enabled(&self) -> bool {
        let defaults = self.print_defaults.read();
        defaults.kitchen_destination.is_some()
    }

    /// Check if label printing is enabled (system level)
    pub fn is_label_print_enabled(&self) -> bool {
        let defaults = self.print_defaults.read();
        defaults.label_destination.is_some()
    }
}
