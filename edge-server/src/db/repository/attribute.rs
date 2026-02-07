//! Attribute Repository
//!
//! Manages attributes, options, and bindings.

use super::{RepoError, RepoResult};
use shared::models::{
    Attribute, AttributeBinding, AttributeCreate, AttributeOption, AttributeUpdate,
};
use sqlx::SqlitePool;

// =========================================================================
// Attribute CRUD
// =========================================================================

pub async fn find_all(pool: &SqlitePool) -> RepoResult<Vec<Attribute>> {
    let mut attrs = sqlx::query_as::<_, Attribute>(
        "SELECT id, name, is_multi_select, max_selections, default_option_indices, display_order, is_active, show_on_receipt, receipt_name, show_on_kitchen_print, kitchen_print_name FROM attribute WHERE is_active = 1 ORDER BY display_order",
    )
    .fetch_all(pool)
    .await?;

    // Load options for each attribute
    for attr in &mut attrs {
        attr.options = find_options_by_attribute(pool, attr.id).await?;
    }
    Ok(attrs)
}

pub async fn find_by_id(pool: &SqlitePool, id: i64) -> RepoResult<Option<Attribute>> {
    let mut attr = sqlx::query_as::<_, Attribute>(
        "SELECT id, name, is_multi_select, max_selections, default_option_indices, display_order, is_active, show_on_receipt, receipt_name, show_on_kitchen_print, kitchen_print_name FROM attribute WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    if let Some(ref mut a) = attr {
        a.options = find_options_by_attribute(pool, a.id).await?;
    }
    Ok(attr)
}

pub async fn create(pool: &SqlitePool, data: AttributeCreate) -> RepoResult<Attribute> {
    let default_option_indices_json = data
        .default_option_indices
        .as_ref()
        .map(|v| serde_json::to_string(v).unwrap_or_else(|_| "[]".to_string()));

    let mut tx = pool.begin().await?;

    let id = sqlx::query_scalar::<_, i64>(
        "INSERT INTO attribute (name, is_multi_select, max_selections, default_option_indices, display_order, is_active, show_on_receipt, receipt_name, show_on_kitchen_print, kitchen_print_name) VALUES (?1, ?2, ?3, ?4, ?5, 1, ?6, ?7, ?8, ?9) RETURNING id",
    )
    .bind(&data.name)
    .bind(data.is_multi_select.unwrap_or(false))
    .bind(data.max_selections)
    .bind(&default_option_indices_json)
    .bind(data.display_order.unwrap_or(0))
    .bind(data.show_on_receipt.unwrap_or(false))
    .bind(&data.receipt_name)
    .bind(data.show_on_kitchen_print.unwrap_or(false))
    .bind(&data.kitchen_print_name)
    .fetch_one(&mut *tx)
    .await?;

    // Create options
    if let Some(options) = data.options {
        for opt in options {
            sqlx::query(
                "INSERT INTO attribute_option (attribute_id, name, price_modifier, display_order, is_active, receipt_name, kitchen_print_name, enable_quantity, max_quantity) VALUES (?1, ?2, ?3, ?4, 1, ?5, ?6, ?7, ?8)",
            )
            .bind(id)
            .bind(&opt.name)
            .bind(opt.price_modifier)
            .bind(opt.display_order)
            .bind(&opt.receipt_name)
            .bind(&opt.kitchen_print_name)
            .bind(opt.enable_quantity)
            .bind(opt.max_quantity)
            .execute(&mut *tx)
            .await?;
        }
    }

    tx.commit().await?;

    find_by_id(pool, id)
        .await?
        .ok_or_else(|| RepoError::Database("Failed to create attribute".into()))
}

pub async fn update(pool: &SqlitePool, id: i64, data: AttributeUpdate) -> RepoResult<Attribute> {
    let default_option_indices_json = data
        .default_option_indices
        .as_ref()
        .map(|v| serde_json::to_string(v).unwrap_or_else(|_| "[]".to_string()));

    let rows = sqlx::query(
        "UPDATE attribute SET name = COALESCE(?1, name), is_multi_select = COALESCE(?2, is_multi_select), max_selections = COALESCE(?3, max_selections), default_option_indices = COALESCE(?4, default_option_indices), display_order = COALESCE(?5, display_order), show_on_receipt = COALESCE(?6, show_on_receipt), receipt_name = COALESCE(?7, receipt_name), show_on_kitchen_print = COALESCE(?8, show_on_kitchen_print), kitchen_print_name = COALESCE(?9, kitchen_print_name), is_active = COALESCE(?10, is_active) WHERE id = ?11",
    )
    .bind(&data.name)
    .bind(data.is_multi_select)
    .bind(data.max_selections)
    .bind(&default_option_indices_json)
    .bind(data.display_order)
    .bind(data.show_on_receipt)
    .bind(&data.receipt_name)
    .bind(data.show_on_kitchen_print)
    .bind(&data.kitchen_print_name)
    .bind(data.is_active)
    .bind(id)
    .execute(pool)
    .await?;

    if rows.rows_affected() == 0 {
        return Err(RepoError::NotFound(format!("Attribute {id} not found")));
    }

    // Replace options if provided (atomic: delete + re-create in transaction)
    if let Some(options) = data.options {
        let mut tx = pool.begin().await?;
        sqlx::query("DELETE FROM attribute_option WHERE attribute_id = ?")
            .bind(id)
            .execute(&mut *tx)
            .await?;
        for opt in &options {
            sqlx::query(
                "INSERT INTO attribute_option (attribute_id, name, price_modifier, display_order, is_active, receipt_name, kitchen_print_name, enable_quantity, max_quantity) VALUES (?1, ?2, ?3, ?4, 1, ?5, ?6, ?7, ?8)",
            )
            .bind(id)
            .bind(&opt.name)
            .bind(opt.price_modifier)
            .bind(opt.display_order)
            .bind(&opt.receipt_name)
            .bind(&opt.kitchen_print_name)
            .bind(opt.enable_quantity)
            .bind(opt.max_quantity)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
    }

    find_by_id(pool, id)
        .await?
        .ok_or_else(|| RepoError::NotFound(format!("Attribute {id} not found")))
}

pub async fn delete(pool: &SqlitePool, id: i64) -> RepoResult<bool> {
    let mut tx = pool.begin().await?;
    // Delete bindings first
    sqlx::query("DELETE FROM attribute_binding WHERE attribute_id = ?")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    // Options cascade via FK
    sqlx::query("DELETE FROM attribute WHERE id = ?")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(true)
}

// =========================================================================
// Attribute Options
// =========================================================================

async fn find_options_by_attribute(pool: &SqlitePool, attribute_id: i64) -> RepoResult<Vec<AttributeOption>> {
    let options = sqlx::query_as::<_, AttributeOption>(
        "SELECT id, attribute_id, name, price_modifier, display_order, is_active, receipt_name, kitchen_print_name, enable_quantity, max_quantity FROM attribute_option WHERE attribute_id = ? ORDER BY display_order",
    )
    .bind(attribute_id)
    .fetch_all(pool)
    .await?;
    Ok(options)
}

// =========================================================================
// Attribute Bindings
// =========================================================================

pub async fn link(
    pool: &SqlitePool,
    owner_type: &str,
    owner_id: i64,
    attribute_id: i64,
    is_required: bool,
    display_order: i32,
    default_option_indices: Option<Vec<i32>>,
) -> RepoResult<AttributeBinding> {
    let indices_json = default_option_indices
        .as_ref()
        .map(|v| serde_json::to_string(v).unwrap_or_else(|_| "[]".to_string()));

    let id = sqlx::query_scalar::<_, i64>(
        "INSERT INTO attribute_binding (owner_type, owner_id, attribute_id, is_required, display_order, default_option_indices) VALUES (?1, ?2, ?3, ?4, ?5, ?6) RETURNING id",
    )
    .bind(owner_type)
    .bind(owner_id)
    .bind(attribute_id)
    .bind(is_required)
    .bind(display_order)
    .bind(&indices_json)
    .fetch_one(pool)
    .await?;

    find_binding_by_id(pool, id)
        .await?
        .ok_or_else(|| RepoError::Database("Failed to create binding".into()))
}

pub async fn unlink(
    pool: &SqlitePool,
    owner_type: &str,
    owner_id: i64,
    attribute_id: i64,
) -> RepoResult<bool> {
    sqlx::query(
        "DELETE FROM attribute_binding WHERE owner_type = ? AND owner_id = ? AND attribute_id = ?",
    )
    .bind(owner_type)
    .bind(owner_id)
    .bind(attribute_id)
    .execute(pool)
    .await?;
    Ok(true)
}

pub async fn find_binding_by_id(pool: &SqlitePool, id: i64) -> RepoResult<Option<AttributeBinding>> {
    let binding = sqlx::query_as::<_, AttributeBinding>(
        "SELECT id, owner_type, owner_id, attribute_id, is_required, display_order, default_option_indices FROM attribute_binding WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    Ok(binding)
}

pub async fn update_binding(
    pool: &SqlitePool,
    id: i64,
    is_required: Option<bool>,
    display_order: Option<i32>,
    default_option_indices: Option<Vec<i32>>,
) -> RepoResult<AttributeBinding> {
    let indices_json = default_option_indices
        .as_ref()
        .map(|v| serde_json::to_string(v).unwrap_or_else(|_| "[]".to_string()));

    let rows = sqlx::query(
        "UPDATE attribute_binding SET is_required = COALESCE(?1, is_required), display_order = COALESCE(?2, display_order), default_option_indices = COALESCE(?3, default_option_indices) WHERE id = ?4",
    )
    .bind(is_required)
    .bind(display_order)
    .bind(&indices_json)
    .bind(id)
    .execute(pool)
    .await?;

    if rows.rows_affected() == 0 {
        return Err(RepoError::NotFound(format!("Binding {id} not found")));
    }

    find_binding_by_id(pool, id)
        .await?
        .ok_or_else(|| RepoError::NotFound(format!("Binding {id} not found")))
}

pub async fn delete_binding(pool: &SqlitePool, id: i64) -> RepoResult<bool> {
    let rows = sqlx::query("DELETE FROM attribute_binding WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(rows.rows_affected() > 0)
}

/// Check if owner already has a specific attribute bound
pub async fn has_binding(
    pool: &SqlitePool,
    owner_type: &str,
    owner_id: i64,
    attribute_id: i64,
) -> RepoResult<bool> {
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM attribute_binding WHERE owner_type = ? AND owner_id = ? AND attribute_id = ?",
    )
    .bind(owner_type)
    .bind(owner_id)
    .bind(attribute_id)
    .fetch_one(pool)
    .await?;
    Ok(count > 0)
}

/// Find bindings for a product or category, with full attribute data
pub async fn find_bindings_for_owner(
    pool: &SqlitePool,
    owner_type: &str,
    owner_id: i64,
) -> RepoResult<Vec<(AttributeBinding, Attribute)>> {
    let bindings = sqlx::query_as::<_, AttributeBinding>(
        "SELECT id, owner_type, owner_id, attribute_id, is_required, display_order, default_option_indices FROM attribute_binding WHERE owner_type = ? AND owner_id = ? ORDER BY display_order",
    )
    .bind(owner_type)
    .bind(owner_id)
    .fetch_all(pool)
    .await?;

    let mut result = Vec::new();
    for binding in bindings {
        if let Some(attr) = find_by_id(pool, binding.attribute_id).await? {
            if attr.is_active {
                result.push((binding, attr));
            }
        }
    }
    Ok(result)
}

/// Find ALL attribute bindings with their full attribute data (for bulk warmup)
pub async fn find_all_bindings_with_attributes(
    pool: &SqlitePool,
) -> RepoResult<Vec<(AttributeBinding, Attribute)>> {
    let bindings = sqlx::query_as::<_, AttributeBinding>(
        "SELECT id, owner_type, owner_id, attribute_id, is_required, display_order, default_option_indices FROM attribute_binding ORDER BY display_order",
    )
    .fetch_all(pool)
    .await?;

    let mut result = Vec::new();
    // Cache attributes to avoid repeated fetches for the same attribute_id
    let mut attr_cache: std::collections::HashMap<i64, Option<Attribute>> = std::collections::HashMap::new();
    for binding in bindings {
        let attr = if let Some(cached) = attr_cache.get(&binding.attribute_id) {
            cached.clone()
        } else {
            let attr = find_by_id(pool, binding.attribute_id).await?;
            attr_cache.insert(binding.attribute_id, attr.clone());
            attr
        };
        if let Some(attr) = attr {
            if attr.is_active {
                result.push((binding, attr));
            }
        }
    }
    Ok(result)
}
