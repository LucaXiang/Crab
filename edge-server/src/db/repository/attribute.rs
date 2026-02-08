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

    batch_load_options(pool, &mut attrs).await?;
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

    let is_multi_select = data.is_multi_select.unwrap_or(false);
    let display_order = data.display_order.unwrap_or(0);
    let show_on_receipt = data.show_on_receipt.unwrap_or(false);
    let show_on_kitchen_print = data.show_on_kitchen_print.unwrap_or(false);
    let id = sqlx::query_scalar!(
        r#"INSERT INTO attribute (name, is_multi_select, max_selections, default_option_indices, display_order, is_active, show_on_receipt, receipt_name, show_on_kitchen_print, kitchen_print_name) VALUES (?1, ?2, ?3, ?4, ?5, 1, ?6, ?7, ?8, ?9) RETURNING id as "id!""#,
        data.name,
        is_multi_select,
        data.max_selections,
        default_option_indices_json,
        display_order,
        show_on_receipt,
        data.receipt_name,
        show_on_kitchen_print,
        data.kitchen_print_name,
    )
    .fetch_one(&mut *tx)
    .await?;

    // Create options
    if let Some(options) = data.options {
        for opt in options {
            sqlx::query!(
                "INSERT INTO attribute_option (attribute_id, name, price_modifier, display_order, is_active, receipt_name, kitchen_print_name, enable_quantity, max_quantity) VALUES (?1, ?2, ?3, ?4, 1, ?5, ?6, ?7, ?8)",
                id,
                opt.name,
                opt.price_modifier,
                opt.display_order,
                opt.receipt_name,
                opt.kitchen_print_name,
                opt.enable_quantity,
                opt.max_quantity,
            )
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

    let rows = sqlx::query!(
        "UPDATE attribute SET name = COALESCE(?1, name), is_multi_select = COALESCE(?2, is_multi_select), max_selections = COALESCE(?3, max_selections), default_option_indices = COALESCE(?4, default_option_indices), display_order = COALESCE(?5, display_order), show_on_receipt = COALESCE(?6, show_on_receipt), receipt_name = COALESCE(?7, receipt_name), show_on_kitchen_print = COALESCE(?8, show_on_kitchen_print), kitchen_print_name = COALESCE(?9, kitchen_print_name), is_active = COALESCE(?10, is_active) WHERE id = ?11",
        data.name,
        data.is_multi_select,
        data.max_selections,
        default_option_indices_json,
        data.display_order,
        data.show_on_receipt,
        data.receipt_name,
        data.show_on_kitchen_print,
        data.kitchen_print_name,
        data.is_active,
        id,
    )
    .execute(pool)
    .await?;

    if rows.rows_affected() == 0 {
        return Err(RepoError::NotFound(format!("Attribute {id} not found")));
    }

    // Replace options if provided (atomic: delete + re-create in transaction)
    if let Some(options) = data.options {
        let mut tx = pool.begin().await?;
        sqlx::query!("DELETE FROM attribute_option WHERE attribute_id = ?", id)
            .execute(&mut *tx)
            .await?;
        for opt in &options {
            sqlx::query!(
                "INSERT INTO attribute_option (attribute_id, name, price_modifier, display_order, is_active, receipt_name, kitchen_print_name, enable_quantity, max_quantity) VALUES (?1, ?2, ?3, ?4, 1, ?5, ?6, ?7, ?8)",
                id,
                opt.name,
                opt.price_modifier,
                opt.display_order,
                opt.receipt_name,
                opt.kitchen_print_name,
                opt.enable_quantity,
                opt.max_quantity,
            )
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
    sqlx::query!("DELETE FROM attribute_binding WHERE attribute_id = ?", id)
        .execute(&mut *tx)
        .await?;
    // Options cascade via FK
    sqlx::query!("DELETE FROM attribute WHERE id = ?", id)
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

/// Batch load options for multiple attributes (eliminates N+1)
async fn batch_load_options(pool: &SqlitePool, attrs: &mut [Attribute]) -> RepoResult<()> {
    if attrs.is_empty() {
        return Ok(());
    }
    let ids: Vec<i64> = attrs.iter().map(|a| a.id).collect();
    let placeholders = ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    let sql = format!(
        "SELECT id, attribute_id, name, price_modifier, display_order, is_active, receipt_name, kitchen_print_name, enable_quantity, max_quantity FROM attribute_option WHERE attribute_id IN ({placeholders}) ORDER BY display_order"
    );
    let mut query = sqlx::query_as::<_, AttributeOption>(&sql);
    for id in &ids {
        query = query.bind(id);
    }
    let all_options = query.fetch_all(pool).await?;

    let mut map: std::collections::HashMap<i64, Vec<AttributeOption>> = std::collections::HashMap::new();
    for opt in all_options {
        map.entry(opt.attribute_id).or_default().push(opt);
    }
    for attr in attrs.iter_mut() {
        attr.options = map.remove(&attr.id).unwrap_or_default();
    }
    Ok(())
}

/// Batch load attributes by IDs with their options (eliminates N+1 in binding queries)
async fn batch_find_attributes(pool: &SqlitePool, ids: &[i64]) -> RepoResult<std::collections::HashMap<i64, Attribute>> {
    if ids.is_empty() {
        return Ok(std::collections::HashMap::new());
    }
    let placeholders = ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    let sql = format!(
        "SELECT id, name, is_multi_select, max_selections, default_option_indices, display_order, is_active, show_on_receipt, receipt_name, show_on_kitchen_print, kitchen_print_name FROM attribute WHERE id IN ({placeholders})"
    );
    let mut query = sqlx::query_as::<_, Attribute>(&sql);
    for id in ids {
        query = query.bind(id);
    }
    let mut attrs: Vec<Attribute> = query.fetch_all(pool).await?;
    batch_load_options(pool, &mut attrs).await?;

    let map = attrs.into_iter().map(|a| (a.id, a)).collect();
    Ok(map)
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

    let id = sqlx::query_scalar!(
        r#"INSERT INTO attribute_binding (owner_type, owner_id, attribute_id, is_required, display_order, default_option_indices) VALUES (?1, ?2, ?3, ?4, ?5, ?6) RETURNING id as "id!""#,
        owner_type,
        owner_id,
        attribute_id,
        is_required,
        display_order,
        indices_json,
    )
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
    sqlx::query!(
        "DELETE FROM attribute_binding WHERE owner_type = ? AND owner_id = ? AND attribute_id = ?",
        owner_type,
        owner_id,
        attribute_id,
    )
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

    let rows = sqlx::query!(
        "UPDATE attribute_binding SET is_required = COALESCE(?1, is_required), display_order = COALESCE(?2, display_order), default_option_indices = COALESCE(?3, default_option_indices) WHERE id = ?4",
        is_required,
        display_order,
        indices_json,
        id,
    )
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
    let rows = sqlx::query!("DELETE FROM attribute_binding WHERE id = ?", id)
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
    let count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM attribute_binding WHERE owner_type = ? AND owner_id = ? AND attribute_id = ?",
        owner_type,
        owner_id,
        attribute_id,
    )
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

    if bindings.is_empty() {
        return Ok(vec![]);
    }

    // Batch load all attributes
    let attr_ids: Vec<i64> = bindings.iter().map(|b| b.attribute_id).collect();
    let attrs = batch_find_attributes(pool, &attr_ids).await?;

    let mut result = Vec::new();
    for binding in bindings {
        if let Some(attr) = attrs.get(&binding.attribute_id) {
            if attr.is_active {
                result.push((binding, attr.clone()));
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

    if bindings.is_empty() {
        return Ok(vec![]);
    }

    // Batch load all unique attributes
    let attr_ids: Vec<i64> = bindings.iter().map(|b| b.attribute_id).collect::<std::collections::HashSet<_>>().into_iter().collect();
    let attrs = batch_find_attributes(pool, &attr_ids).await?;

    let mut result = Vec::new();
    for binding in bindings {
        if let Some(attr) = attrs.get(&binding.attribute_id) {
            if attr.is_active {
                result.push((binding, attr.clone()));
            }
        }
    }
    Ok(result)
}
