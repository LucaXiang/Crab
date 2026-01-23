//! Product Repository

use super::{BaseRepository, RepoError, RepoResult, make_thing, strip_table_prefix};
use crate::db::models::{Product, ProductCreate, ProductUpdate};
use surrealdb::Surreal;
use surrealdb::engine::local::Db;

const PRODUCT_TABLE: &str = "product";

// =============================================================================
// Product Repository
// =============================================================================

#[derive(Clone)]
pub struct ProductRepository {
    base: BaseRepository,
}

impl ProductRepository {
    pub fn new(db: Surreal<Db>) -> Self {
        Self {
            base: BaseRepository::new(db),
        }
    }

    /// Find all active products
    pub async fn find_all(&self) -> RepoResult<Vec<Product>> {
        let products: Vec<Product> = self
            .base
            .db()
            .query("SELECT * FROM product WHERE is_active = true ORDER BY sort_order")
            .await?
            .take(0)?;
        Ok(products)
    }

    /// Find products by category with category data fetched
    pub async fn find_by_category(&self, category_id: &str) -> RepoResult<Vec<Product>> {
        let cat_thing = make_thing("category", category_id);
        let products: Vec<Product> = self
            .base
            .db()
            .query("SELECT * FROM product WHERE category = $cat AND is_active = true ORDER BY sort_order FETCH category")
            .bind(("cat", cat_thing))
            .await?
            .take(0)?;
        Ok(products)
    }

    /// Find product by id with category fetched
    pub async fn find_by_id(&self, id: &str) -> RepoResult<Option<Product>> {
        let pure_id = strip_table_prefix(PRODUCT_TABLE, id);
        let product: Option<Product> = self.base.db().select((PRODUCT_TABLE, pure_id)).await?;
        Ok(product)
    }

    /// Find product by id with all relations fetched
    pub async fn find_by_id_full(&self, id: &str) -> RepoResult<Option<Product>> {
        let prod_thing = make_thing(PRODUCT_TABLE, id);
        let mut result = self
            .base
            .db()
            .query("SELECT * FROM product WHERE id = $id FETCH category, print_destinations")
            .bind(("id", prod_thing))
            .await?;
        let products: Vec<Product> = result.take(0)?;
        Ok(products.into_iter().next())
    }

    /// Sync product_spec table after create/update (best effort, ignore if table doesn't exist)
    async fn sync_product_specs(
        &self,
        product_id: &surrealdb::sql::Thing,
        specs: &[crate::db::models::EmbeddedSpec],
    ) -> RepoResult<()> {
        // Delete existing product_spec records for this product (ignore errors)
        let _ = self.base
            .db()
            .query("DELETE product_spec WHERE product = $product")
            .bind(("product", product_id.clone()))
            .await;

        // Insert new records for specs with external_id (ignore errors if table doesn't exist)
        for (index, spec) in specs.iter().enumerate() {
            if let Some(external_id) = spec.external_id {
                let _ = self.base
                    .db()
                    .query(
                        "CREATE product_spec SET product = $product, spec_index = $index, external_id = $external_id",
                    )
                    .bind(("product", product_id.clone()))
                    .bind(("index", index as i32))
                    .bind(("external_id", external_id))
                    .await;
            }
        }
        Ok(())
    }

    /// Create a new product
    pub async fn create(&self, data: ProductCreate) -> RepoResult<Product> {
        // 校验 specs 非空
        if data.specs.is_empty() {
            return Err(RepoError::Validation("specs cannot be empty".into()));
        }
        // 校验最多一个 default
        let default_count = data.specs.iter().filter(|s| s.is_default).count();
        if default_count > 1 {
            return Err(RepoError::Validation(
                "only one default spec allowed".into(),
            ));
        }

        let specs_clone = data.specs.clone();
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

        let created: Option<Product> = self
            .base
            .db()
            .create(PRODUCT_TABLE)
            .content(product)
            .await?;

        let created = created.ok_or_else(|| RepoError::Database("Failed to create product".to_string()))?;

        // Sync product_spec table for external_id uniqueness
        if let Some(ref id) = created.id {
            self.sync_product_specs(id, &specs_clone).await?;
        }

        Ok(created)
    }

    /// Update a product
    pub async fn update(&self, id: &str, data: ProductUpdate) -> RepoResult<Product> {
        let pure_id = strip_table_prefix(PRODUCT_TABLE, id);
        let thing = make_thing(PRODUCT_TABLE, pure_id);

        // Clone specs before moving data
        let specs_to_sync = data.specs.clone();

        // Build dynamic SET clauses with proper type bindings
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
            // No fields to update
            return self.find_by_id(pure_id)
                .await?
                .ok_or_else(|| RepoError::NotFound(format!("Product {} not found", id)));
        }

        let query_str = format!("UPDATE $thing SET {} RETURN AFTER", set_parts.join(", "));
        tracing::info!("ProductRepository::update - query: {}, id: {}", query_str, id);

        // Build query with direct type bindings (Things bound as Things, not strings)
        let mut query = self.base.db().query(&query_str).bind(("thing", thing.clone()));

        // Bind each field with its native type
        if let Some(v) = data.name { query = query.bind(("name", v)); }
        if let Some(v) = data.image { query = query.bind(("image", v)); }
        if let Some(v) = data.category { query = query.bind(("category", v)); } // Thing type
        if let Some(v) = data.sort_order { query = query.bind(("sort_order", v)); }
        if let Some(v) = data.tax_rate {
            tracing::info!("ProductRepository::update - binding tax_rate: {}", v);
            query = query.bind(("tax_rate", v));
        }
        if let Some(v) = data.receipt_name { query = query.bind(("receipt_name", v)); }
        if let Some(v) = data.kitchen_print_name { query = query.bind(("kitchen_print_name", v)); }
        if let Some(v) = data.kitchen_print_destinations { query = query.bind(("kitchen_print_destinations", v)); } // Vec<Thing>
        if let Some(v) = data.label_print_destinations { query = query.bind(("label_print_destinations", v)); } // Vec<Thing>
        if let Some(v) = data.is_kitchen_print_enabled { query = query.bind(("is_kitchen_print_enabled", v)); }
        if let Some(v) = data.is_label_print_enabled { query = query.bind(("is_label_print_enabled", v)); }
        if let Some(v) = data.is_active { query = query.bind(("is_active", v)); }
        if let Some(v) = data.tags { query = query.bind(("tags", v)); } // Vec<Thing>
        if let Some(v) = data.specs {
            // specs need to be serialized as JSON value for embedded objects
            query = query.bind(("specs", serde_json::to_value(&v).unwrap_or_default()));
        }

        let mut result = query.await?;
        let products: Vec<Product> = result.take(0)?;

        // Sync product_spec table if specs were updated
        if let Some(specs) = specs_to_sync {
            self.sync_product_specs(&thing, &specs).await?;
        }

        products.into_iter().next()
            .ok_or_else(|| RepoError::NotFound(format!("Product {} not found", id)))
    }

    /// Hard delete a product (also cleans up has_attribute edges and product_spec)
    pub async fn delete(&self, id: &str) -> RepoResult<()> {
        let pure_id = strip_table_prefix(PRODUCT_TABLE, id);
        let thing = make_thing(PRODUCT_TABLE, pure_id);

        // Clean up product_spec records first (ignore error if table doesn't exist)
        let _ = self.base
            .db()
            .query("DELETE product_spec WHERE product = $product")
            .bind(("product", thing.clone()))
            .await;

        // Clean up has_attribute edges
        let _ = self.base
            .db()
            .query("DELETE has_attribute WHERE in = $product")
            .bind(("product", thing.clone()))
            .await;

        // Then delete the product
        let result: Option<Product> = self.base.db().delete((PRODUCT_TABLE, pure_id)).await?;
        if result.is_none() {
            return Err(RepoError::NotFound(format!("Product {} not found", id)));
        }
        Ok(())
    }

    /// Find all active products with print destinations fetched
    pub async fn find_all_with_destinations(&self) -> RepoResult<Vec<Product>> {
        let products: Vec<Product> = self
            .base
            .db()
            .query("SELECT * FROM product WHERE is_active = true ORDER BY sort_order FETCH kitchen_print_destinations, label_print_destinations")
            .await?
            .take(0)?;
        Ok(products)
    }

    /// Find all active products with tags fetched
    pub async fn find_all_with_tags(&self) -> RepoResult<Vec<Product>> {
        let products: Vec<Product> = self
            .base
            .db()
            .query("SELECT * FROM product WHERE is_active = true ORDER BY sort_order FETCH tags")
            .await?
            .take(0)?;
        Ok(products)
    }

    /// Find product by id with tags fetched
    pub async fn find_by_id_with_tags(&self, id: &str) -> RepoResult<Option<Product>> {
        let prod_thing = make_thing(PRODUCT_TABLE, id);
        let mut result = self
            .base
            .db()
            .query("SELECT * FROM product WHERE id = $id FETCH tags")
            .bind(("id", prod_thing))
            .await?;
        let products: Vec<Product> = result.take(0)?;
        Ok(products.into_iter().next())
    }

    /// Add tag to product
    pub async fn add_tag(&self, product_id: &str, tag_id: &str) -> RepoResult<Product> {
        let prod_thing = make_thing(PRODUCT_TABLE, product_id);
        let tag_thing = make_thing("tag", tag_id);
        let mut result = self
            .base
            .db()
            .query("UPDATE product SET tags += $tag WHERE id = $id RETURN AFTER")
            .bind(("id", prod_thing))
            .bind(("tag", tag_thing))
            .await?;
        let products: Vec<Product> = result.take(0)?;
        products
            .into_iter()
            .next()
            .ok_or_else(|| RepoError::NotFound(format!("Product {} not found", product_id)))
    }

    /// Remove tag from product
    pub async fn remove_tag(&self, product_id: &str, tag_id: &str) -> RepoResult<Product> {
        let prod_thing = make_thing(PRODUCT_TABLE, product_id);
        let tag_thing = make_thing("tag", tag_id);
        let mut result = self
            .base
            .db()
            .query("UPDATE product SET tags -= $tag WHERE id = $id RETURN AFTER")
            .bind(("id", prod_thing))
            .bind(("tag", tag_thing))
            .await?;
        let products: Vec<Product> = result.take(0)?;
        products
            .into_iter()
            .next()
            .ok_or_else(|| RepoError::NotFound(format!("Product {} not found", product_id)))
    }
}
