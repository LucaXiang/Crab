//! Product & ProductSpecification Repository

use super::{make_thing, BaseRepository, RepoError, RepoResult};
use crate::db::models::{
    Product, ProductCreate, ProductUpdate,
    ProductSpecification, ProductSpecificationCreate, ProductSpecificationUpdate,
};
use surrealdb::engine::local::Db;
use surrealdb::Surreal;

const PRODUCT_TABLE: &str = "product";
const SPEC_TABLE: &str = "product_specification";

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
        let product: Option<Product> = self.base.db().select((PRODUCT_TABLE, id)).await?;
        Ok(product)
    }

    /// Find product by id with all relations fetched
    pub async fn find_by_id_full(&self, id: &str) -> RepoResult<Option<Product>> {
        let prod_thing = make_thing(PRODUCT_TABLE, id);
        let mut result = self
            .base
            .db()
            .query("SELECT * FROM product WHERE id = $id FETCH category, kitchen_printer")
            .bind(("id", prod_thing))
            .await?;
        let products: Vec<Product> = result.take(0)?;
        Ok(products.into_iter().next())
    }

    /// Create a new product
    pub async fn create(&self, data: ProductCreate) -> RepoResult<Product> {
        let product = Product {
            id: None,
            name: data.name,
            image: data.image.unwrap_or_default(),
            category: data.category,
            sort_order: data.sort_order.unwrap_or(0),
            tax_rate: data.tax_rate.unwrap_or(0),
            has_multi_spec: data.has_multi_spec.unwrap_or(false),
            receipt_name: data.receipt_name,
            kitchen_print_name: data.kitchen_print_name,
            kitchen_printer: data.kitchen_printer,
            is_kitchen_print_enabled: data.is_kitchen_print_enabled.unwrap_or(-1),
            is_label_print_enabled: data.is_label_print_enabled.unwrap_or(-1),
            is_active: true,
        };

        let created: Option<Product> = self.base.db().create(PRODUCT_TABLE).content(product).await?;
        created.ok_or_else(|| RepoError::Database("Failed to create product".to_string()))
    }

    /// Update a product
    pub async fn update(&self, id: &str, data: ProductUpdate) -> RepoResult<Product> {
        let updated: Option<Product> = self
            .base
            .db()
            .update((PRODUCT_TABLE, id))
            .merge(data)
            .await?;

        updated.ok_or_else(|| RepoError::NotFound(format!("Product {} not found", id)))
    }

    /// Soft delete a product
    pub async fn delete(&self, id: &str) -> RepoResult<bool> {
        let result: Option<Product> = self
            .base
            .db()
            .update((PRODUCT_TABLE, id))
            .merge(ProductUpdate {
                name: None,
                image: None,
                category: None,
                sort_order: None,
                tax_rate: None,
                has_multi_spec: None,
                receipt_name: None,
                kitchen_print_name: None,
                kitchen_printer: None,
                is_kitchen_print_enabled: None,
                is_label_print_enabled: None,
                is_active: Some(false),
            })
            .await?;
        Ok(result.is_some())
    }

    /// Get product with specifications
    pub async fn find_with_specs(&self, id: &str) -> RepoResult<(Product, Vec<ProductSpecification>)> {
        let product = self
            .find_by_id(id)
            .await?
            .ok_or_else(|| RepoError::NotFound(format!("Product {} not found", id)))?;

        let prod_thing = make_thing(PRODUCT_TABLE, id);
        let specs: Vec<ProductSpecification> = self
            .base
            .db()
            .query("SELECT * FROM product_specification WHERE product = $prod AND is_active = true ORDER BY display_order FETCH tags")
            .bind(("prod", prod_thing))
            .await?
            .take(0)?;

        Ok((product, specs))
    }
}

// =============================================================================
// ProductSpecification Repository
// =============================================================================

#[derive(Clone)]
pub struct ProductSpecificationRepository {
    base: BaseRepository,
}

impl ProductSpecificationRepository {
    pub fn new(db: Surreal<Db>) -> Self {
        Self {
            base: BaseRepository::new(db),
        }
    }

    /// Find all specs for a product
    pub async fn find_by_product(&self, product_id: &str) -> RepoResult<Vec<ProductSpecification>> {
        let prod_thing = make_thing(PRODUCT_TABLE, product_id);
        let specs: Vec<ProductSpecification> = self
            .base
            .db()
            .query("SELECT * FROM product_specification WHERE product = $prod AND is_active = true ORDER BY display_order FETCH tags")
            .bind(("prod", prod_thing))
            .await?
            .take(0)?;
        Ok(specs)
    }

    /// Find spec by id
    pub async fn find_by_id(&self, id: &str) -> RepoResult<Option<ProductSpecification>> {
        let spec: Option<ProductSpecification> = self.base.db().select((SPEC_TABLE, id)).await?;
        Ok(spec)
    }

    /// Find spec by id with tags fetched
    pub async fn find_by_id_with_tags(&self, id: &str) -> RepoResult<Option<ProductSpecification>> {
        let spec_thing = make_thing(SPEC_TABLE, id);
        let mut result = self
            .base
            .db()
            .query("SELECT * FROM product_specification WHERE id = $id FETCH tags")
            .bind(("id", spec_thing))
            .await?;
        let specs: Vec<ProductSpecification> = result.take(0)?;
        Ok(specs.into_iter().next())
    }

    /// Create a new spec
    pub async fn create(&self, data: ProductSpecificationCreate) -> RepoResult<ProductSpecification> {
        let spec = ProductSpecification {
            id: None,
            product: data.product,
            name: data.name,
            price: data.price,
            display_order: data.display_order.unwrap_or(0),
            is_default: data.is_default.unwrap_or(false),
            is_active: true,
            is_root: data.is_root.unwrap_or(false),
            external_id: data.external_id,
            tags: data.tags.unwrap_or_default(),
            created_at: None,
            updated_at: None,
        };

        let created: Option<ProductSpecification> = self.base.db().create(SPEC_TABLE).content(spec).await?;
        created.ok_or_else(|| RepoError::Database("Failed to create specification".to_string()))
    }

    /// Update a spec
    pub async fn update(&self, id: &str, data: ProductSpecificationUpdate) -> RepoResult<ProductSpecification> {
        let updated: Option<ProductSpecification> = self
            .base
            .db()
            .update((SPEC_TABLE, id))
            .merge(data)
            .await?;

        updated.ok_or_else(|| RepoError::NotFound(format!("Specification {} not found", id)))
    }

    /// Add tag to spec
    pub async fn add_tag(&self, spec_id: &str, tag_id: &str) -> RepoResult<ProductSpecification> {
        let spec_thing = make_thing(SPEC_TABLE, spec_id);
        let tag_thing = make_thing("tag", tag_id);
        let mut result = self
            .base
            .db()
            .query("UPDATE product_specification SET tags += $tag WHERE id = $id RETURN AFTER")
            .bind(("id", spec_thing))
            .bind(("tag", tag_thing))
            .await?;
        let specs: Vec<ProductSpecification> = result.take(0)?;
        specs
            .into_iter()
            .next()
            .ok_or_else(|| RepoError::NotFound(format!("Specification {} not found", spec_id)))
    }

    /// Remove tag from spec
    pub async fn remove_tag(&self, spec_id: &str, tag_id: &str) -> RepoResult<ProductSpecification> {
        let spec_thing = make_thing(SPEC_TABLE, spec_id);
        let tag_thing = make_thing("tag", tag_id);
        let mut result = self
            .base
            .db()
            .query("UPDATE product_specification SET tags -= $tag WHERE id = $id RETURN AFTER")
            .bind(("id", spec_thing))
            .bind(("tag", tag_thing))
            .await?;
        let specs: Vec<ProductSpecification> = result.take(0)?;
        specs
            .into_iter()
            .next()
            .ok_or_else(|| RepoError::NotFound(format!("Specification {} not found", spec_id)))
    }

    /// Soft delete a spec
    pub async fn delete(&self, id: &str) -> RepoResult<bool> {
        let result: Option<ProductSpecification> = self
            .base
            .db()
            .update((SPEC_TABLE, id))
            .merge(ProductSpecificationUpdate {
                name: None,
                price: None,
                display_order: None,
                is_default: None,
                is_active: Some(false),
                is_root: None,
                external_id: None,
                tags: None,
            })
            .await?;
        Ok(result.is_some())
    }
}
