//! Attribute Repository (Graph DB style)
//!
//! Uses RELATE to connect products/categories to attributes.

use super::{BaseRepository, RepoError, RepoResult};
use crate::db::models::{
    serde_helpers, Attribute, AttributeBinding, AttributeCreate, AttributeOption, AttributeUpdate,
};
use surrealdb::engine::local::Db;
use surrealdb::{RecordId, Surreal};

const TABLE: &str = "attribute";

#[derive(Clone)]
pub struct AttributeRepository {
    base: BaseRepository,
}

impl AttributeRepository {
    pub fn new(db: Surreal<Db>) -> Self {
        Self {
            base: BaseRepository::new(db),
        }
    }

    // =========================================================================
    // Attribute CRUD
    // =========================================================================

    /// Find all active attributes
    pub async fn find_all(&self) -> RepoResult<Vec<Attribute>> {
        let attrs: Vec<Attribute> = self
            .base
            .db()
            .query("SELECT * FROM attribute WHERE is_active = true ORDER BY display_order")
            .await?
            .take(0)?;
        Ok(attrs)
    }

    /// Find attribute by id
    pub async fn find_by_id(&self, id: &str) -> RepoResult<Option<Attribute>> {
        let thing: RecordId = id
            .parse()
            .map_err(|_| RepoError::Validation(format!("Invalid ID: {}", id)))?;
        let attr: Option<Attribute> = self.base.db().select(thing).await?;
        Ok(attr)
    }

    /// Create a new attribute
    pub async fn create(&self, data: AttributeCreate) -> RepoResult<Attribute> {
        let attr = Attribute {
            id: None,
            name: data.name,
            is_multi_select: data.is_multi_select.unwrap_or(false),
            max_selections: data.max_selections,
            default_option_indices: data.default_option_indices,
            display_order: data.display_order.unwrap_or(0),
            is_active: true,
            show_on_receipt: data.show_on_receipt.unwrap_or(false),
            receipt_name: data.receipt_name,
            show_on_kitchen_print: data.show_on_kitchen_print.unwrap_or(false),
            kitchen_print_name: data.kitchen_print_name,
            options: data.options.unwrap_or_default(),
        };

        let created: Option<Attribute> = self.base.db().create(TABLE).content(attr).await?;
        created.ok_or_else(|| RepoError::Database("Failed to create attribute".to_string()))
    }

    /// Update an attribute
    pub async fn update(&self, id: &str, data: AttributeUpdate) -> RepoResult<Attribute> {
        let thing: RecordId = id
            .parse()
            .map_err(|_| RepoError::Validation(format!("Invalid ID: {}", id)))?;

        // Update using raw query to avoid deserialization issues with null fields
        let mut result = self.base
            .db()
            .query("UPDATE $thing MERGE $data RETURN AFTER")
            .bind(("thing", thing))
            .bind(("data", data))
            .await?;

        result.take::<Option<Attribute>>(0)?
            .ok_or_else(|| RepoError::NotFound(format!("Attribute {} not found", id)))
    }

    /// Add option to attribute
    pub async fn add_option(
        &self,
        attr_id: &str,
        option: AttributeOption,
    ) -> RepoResult<Attribute> {
        let thing: RecordId = attr_id
            .parse()
            .map_err(|_| RepoError::Validation(format!("Invalid ID: {}", attr_id)))?;
        let mut result = self
            .base
            .db()
            .query("UPDATE attribute SET options += $opt WHERE id = $id RETURN AFTER")
            .bind(("id", thing))
            .bind(("opt", option))
            .await?;
        let attrs: Vec<Attribute> = result.take(0)?;
        attrs
            .into_iter()
            .next()
            .ok_or_else(|| RepoError::NotFound(format!("Attribute {} not found", attr_id)))
    }

    /// Update option by index
    pub async fn update_option(
        &self,
        attr_id: &str,
        idx: usize,
        option: AttributeOption,
    ) -> RepoResult<Attribute> {
        let thing: RecordId = attr_id
            .parse()
            .map_err(|_| RepoError::Validation(format!("Invalid ID: {}", attr_id)))?;
        let mut result = self
            .base
            .db()
            .query("UPDATE attribute SET options[$idx] = $opt WHERE id = $id RETURN AFTER")
            .bind(("id", thing))
            .bind(("idx", idx))
            .bind(("opt", option))
            .await?;
        let attrs: Vec<Attribute> = result.take(0)?;
        attrs
            .into_iter()
            .next()
            .ok_or_else(|| RepoError::NotFound(format!("Attribute {} not found", attr_id)))
    }

    /// Remove option by index
    pub async fn remove_option(&self, attr_id: &str, idx: usize) -> RepoResult<Attribute> {
        let thing: RecordId = attr_id
            .parse()
            .map_err(|_| RepoError::Validation(format!("Invalid ID: {}", attr_id)))?;
        let mut result = self
            .base
            .db()
            .query("UPDATE attribute SET options = array::remove(options, $idx) WHERE id = $id RETURN AFTER")
            .bind(("id", thing))
            .bind(("idx", idx))
            .await?;
        let attrs: Vec<Attribute> = result.take(0)?;
        attrs
            .into_iter()
            .next()
            .ok_or_else(|| RepoError::NotFound(format!("Attribute {} not found", attr_id)))
    }

    /// Hard delete attribute
    pub async fn delete(&self, id: &str) -> RepoResult<bool> {
        let thing: RecordId = id
            .parse()
            .map_err(|_| RepoError::Validation(format!("Invalid ID: {}", id)))?;

        // Delete all relations first
        self.base
            .db()
            .query("DELETE has_attribute WHERE out = $attr")
            .bind(("attr", thing.clone()))
            .await?;

        // Hard delete
        self.base
            .db()
            .query("DELETE $thing")
            .bind(("thing", thing))
            .await?;

        Ok(true)
    }

    // =========================================================================
    // Graph Relations (RELATE)
    // =========================================================================

    /// Link attribute to product using RELATE
    pub async fn link_to_product(
        &self,
        product_id: &str,
        attr_id: &str,
        is_required: bool,
        display_order: i32,
        default_option_indices: Option<Vec<i32>>,
    ) -> RepoResult<AttributeBinding> {
        let product_thing: RecordId = product_id
            .parse()
            .map_err(|_| RepoError::Validation(format!("Invalid product ID: {}", product_id)))?;
        let attr_thing: RecordId = attr_id
            .parse()
            .map_err(|_| RepoError::Validation(format!("Invalid attribute ID: {}", attr_id)))?;
        let mut result = self
            .base
            .db()
            .query(
                "RELATE $from->has_attribute->$to SET is_required = $req, display_order = $order, default_option_indices = $default_indices",
            )
            .bind(("from", product_thing))
            .bind(("to", attr_thing))
            .bind(("req", is_required))
            .bind(("order", display_order))
            .bind(("default_indices", default_option_indices))
            .await?;
        let edges: Vec<AttributeBinding> = result.take(0)?;
        edges
            .into_iter()
            .next()
            .ok_or_else(|| RepoError::Database("Failed to create relation".to_string()))
    }

    /// Link attribute to category using RELATE
    pub async fn link_to_category(
        &self,
        category_id: &str,
        attr_id: &str,
        is_required: bool,
        display_order: i32,
        default_option_indices: Option<Vec<i32>>,
    ) -> RepoResult<AttributeBinding> {
        let category_thing: RecordId = category_id
            .parse()
            .map_err(|_| RepoError::Validation(format!("Invalid category ID: {}", category_id)))?;
        let attr_thing: RecordId = attr_id
            .parse()
            .map_err(|_| RepoError::Validation(format!("Invalid attribute ID: {}", attr_id)))?;
        let mut result = self
            .base
            .db()
            .query(
                "RELATE $from->has_attribute->$to SET is_required = $req, display_order = $order, default_option_indices = $default_indices",
            )
            .bind(("from", category_thing))
            .bind(("to", attr_thing))
            .bind(("req", is_required))
            .bind(("order", display_order))
            .bind(("default_indices", default_option_indices))
            .await?;
        let edges: Vec<AttributeBinding> = result.take(0)?;
        edges
            .into_iter()
            .next()
            .ok_or_else(|| RepoError::Database("Failed to create relation".to_string()))
    }

    /// Unlink attribute from product
    pub async fn unlink_from_product(&self, product_id: &str, attr_id: &str) -> RepoResult<bool> {
        let product_thing: RecordId = product_id
            .parse()
            .map_err(|_| RepoError::Validation(format!("Invalid product ID: {}", product_id)))?;
        let attr_thing: RecordId = attr_id
            .parse()
            .map_err(|_| RepoError::Validation(format!("Invalid attribute ID: {}", attr_id)))?;
        self.base
            .db()
            .query("DELETE has_attribute WHERE in = $from AND out = $to")
            .bind(("from", product_thing))
            .bind(("to", attr_thing))
            .await?;
        Ok(true)
    }

    /// Unlink attribute from category
    pub async fn unlink_from_category(&self, category_id: &str, attr_id: &str) -> RepoResult<bool> {
        let category_thing: RecordId = category_id
            .parse()
            .map_err(|_| RepoError::Validation(format!("Invalid category ID: {}", category_id)))?;
        let attr_thing: RecordId = attr_id
            .parse()
            .map_err(|_| RepoError::Validation(format!("Invalid attribute ID: {}", attr_id)))?;
        self.base
            .db()
            .query("DELETE has_attribute WHERE in = $from AND out = $to")
            .bind(("from", category_thing))
            .bind(("to", attr_thing))
            .await?;
        Ok(true)
    }

    /// Get attributes for a product (Graph traversal)
    pub async fn find_by_product(&self, product_id: &str) -> RepoResult<Vec<Attribute>> {
        let product_thing: RecordId = product_id
            .parse()
            .map_err(|_| RepoError::Validation(format!("Invalid product ID: {}", product_id)))?;
        let attrs: Vec<Attribute> = self
            .base
            .db()
            .query("SELECT ->has_attribute->attribute.* FROM $prod")
            .bind(("prod", product_thing))
            .await?
            .take(0)?;
        Ok(attrs)
    }

    /// Get attributes for a category (Graph traversal)
    pub async fn find_by_category(&self, category_id: &str) -> RepoResult<Vec<Attribute>> {
        let category_thing: RecordId = category_id
            .parse()
            .map_err(|_| RepoError::Validation(format!("Invalid category ID: {}", category_id)))?;
        let attrs: Vec<Attribute> = self
            .base
            .db()
            .query("SELECT ->has_attribute->attribute.* FROM $cat")
            .bind(("cat", category_thing))
            .await?
            .take(0)?;
        Ok(attrs)
    }

    /// Get product attribute bindings with full attribute data
    /// Returns (AttributeBinding, Attribute) pairs for a product
    pub async fn find_bindings_for_product(
        &self,
        product_id: &str,
    ) -> RepoResult<Vec<(AttributeBinding, Attribute)>> {
        let product_thing: RecordId = product_id
            .parse()
            .map_err(|_| RepoError::Validation(format!("Invalid product ID: {}", product_id)))?;
        // Query the has_attribute edge table and fetch the attribute
        let mut result = self
            .base
            .db()
            .query(
                r#"
                SELECT *, out.* as attr_data
                FROM has_attribute
                WHERE in = $prod AND out.is_active = true
                ORDER BY display_order
                "#,
            )
            .bind(("prod", product_thing))
            .await?;

        #[derive(Debug, serde::Deserialize)]
        struct BindingRow {
            #[serde(default, with = "serde_helpers::option_record_id")]
            id: Option<RecordId>,
            #[serde(rename = "in", with = "serde_helpers::record_id")]
            from: RecordId,
            #[serde(with = "serde_helpers::record_id")]
            out: RecordId,
            is_required: bool,
            display_order: i32,
            default_option_indices: Option<Vec<i32>>,
            attr_data: Attribute,
        }

        let rows: Vec<BindingRow> = result.take(0)?;

        Ok(rows
            .into_iter()
            .map(|row| {
                let binding = AttributeBinding {
                    id: row.id,
                    from: row.from,
                    to: row.out,
                    is_required: row.is_required,
                    display_order: row.display_order,
                    default_option_indices: row.default_option_indices,
                };
                (binding, row.attr_data)
            })
            .collect())
    }

    /// Get all attributes for a product (including inherited from category)
    pub async fn find_effective_for_product(&self, product_id: &str) -> RepoResult<Vec<Attribute>> {
        // Get product's category
        let prod_thing: RecordId = product_id
            .parse()
            .map_err(|_| RepoError::Validation(format!("Invalid product ID: {}", product_id)))?;
        let mut result = self
            .base
            .db()
            .query(
                r#"
                LET $cat = (SELECT category FROM product WHERE id = $prod)[0].category;

                -- Product direct attributes
                LET $prod_attrs = SELECT ->has_attribute->attribute.* FROM $prod;

                -- Category attributes
                LET $cat_attrs = SELECT ->has_attribute->attribute.* FROM $cat;

                -- Combine and deduplicate
                RETURN array::distinct(array::concat($prod_attrs, $cat_attrs));
                "#
            )
            .bind(("prod", prod_thing))
            .await?;

        let attrs: Vec<Attribute> = result.take(0)?;
        Ok(attrs)
    }
}
