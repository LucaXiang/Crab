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

        let product = Product {
            id: None,
            name: data.name,
            image: data.image.unwrap_or_default(),
            category: data.category,
            sort_order: data.sort_order.unwrap_or(0),
            tax_rate: data.tax_rate.unwrap_or(0),
            receipt_name: data.receipt_name,
            kitchen_print_name: data.kitchen_print_name,
            print_destinations: data.print_destinations.unwrap_or_default(),
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
        created.ok_or_else(|| RepoError::Database("Failed to create product".to_string()))
    }

    /// Update a product
    pub async fn update(&self, id: &str, data: ProductUpdate) -> RepoResult<Product> {
        let pure_id = strip_table_prefix(PRODUCT_TABLE, id);
        let thing = make_thing(PRODUCT_TABLE, pure_id);

        self.base
            .db()
            .query("UPDATE $thing MERGE $data")
            .bind(("thing", thing))
            .bind(("data", data))
            .await?;

        self.find_by_id(pure_id)
            .await?
            .ok_or_else(|| RepoError::NotFound(format!("Product {} not found", id)))
    }

    /// Hard delete a product (also cleans up has_attribute edges)
    pub async fn delete(&self, id: &str) -> RepoResult<bool> {
        let thing = make_thing(PRODUCT_TABLE, id);

        // Clean up has_attribute edges first
        self.base
            .db()
            .query("DELETE has_attribute WHERE in = $product")
            .bind(("product", thing.clone()))
            .await?;

        // Then delete the product
        let result: Option<Product> = self.base.db().delete((PRODUCT_TABLE, id)).await?;
        Ok(result.is_some())
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
