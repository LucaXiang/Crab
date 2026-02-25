//! One-time ID migration: remap legacy autoincrement IDs to snowflake IDs.
//!
//! Runs at startup after SQLx migrations. Detects legacy IDs (< 1 billion)
//! and remaps them to snowflake IDs, preserving all FK relationships.
//!
//! TODO: Remove this module after public beta when all edge devices have migrated.

use sqlx::SqlitePool;
use std::collections::HashMap;

/// Legacy ID threshold: IDs below this are autoincrement and need migration.
const LEGACY_THRESHOLD: i64 = 1_000_000_000;

/// Run the ID migration if any legacy IDs are detected.
pub async fn migrate_ids_if_needed(pool: &SqlitePool) -> Result<(), String> {
    // Quick check: does any main table have legacy IDs?
    let has_legacy: bool = sqlx::query_scalar(
        "SELECT EXISTS(
            SELECT 1 FROM product WHERE id < ?1
            UNION ALL SELECT 1 FROM category WHERE id < ?1
            UNION ALL SELECT 1 FROM tag WHERE id < ?1
            UNION ALL SELECT 1 FROM attribute WHERE id < ?1
            UNION ALL SELECT 1 FROM employee WHERE id < ?1
            UNION ALL SELECT 1 FROM zone WHERE id < ?1
            UNION ALL SELECT 1 FROM dining_table WHERE id < ?1
            UNION ALL SELECT 1 FROM price_rule WHERE id < ?1
            UNION ALL SELECT 1 FROM label_template WHERE id < ?1
            UNION ALL SELECT 1 FROM role WHERE id < ?1
            UNION ALL SELECT 1 FROM print_destination WHERE id < ?1
            UNION ALL SELECT 1 FROM shift WHERE id < ?1
            LIMIT 1
        )",
    )
    .bind(LEGACY_THRESHOLD)
    .fetch_one(pool)
    .await
    .map_err(|e| format!("ID migration check failed: {e}"))?;

    if !has_legacy {
        return Ok(());
    }

    tracing::info!("Legacy autoincrement IDs detected, starting migration to snowflake IDs...");

    // Disable FK checks during migration
    sqlx::query("PRAGMA foreign_keys = OFF")
        .execute(pool)
        .await
        .map_err(|e| format!("Failed to disable FK: {e}"))?;

    let result = do_migration(pool).await;

    // Re-enable FK checks
    sqlx::query("PRAGMA foreign_keys = ON")
        .execute(pool)
        .await
        .map_err(|e| format!("Failed to re-enable FK: {e}"))?;

    // Verify FK integrity
    let fk_check: Vec<(String, i64, String, i64)> = sqlx::query_as("PRAGMA foreign_key_check")
        .fetch_all(pool)
        .await
        .map_err(|e| format!("FK integrity check failed: {e}"))?;

    if !fk_check.is_empty() {
        tracing::error!(
            "FK integrity violations after migration: {:?}",
            &fk_check[..fk_check.len().min(10)]
        );
        return Err(format!(
            "FK integrity check failed: {} violations",
            fk_check.len()
        ));
    }

    result?;
    tracing::info!("ID migration completed successfully");
    Ok(())
}

async fn do_migration(pool: &SqlitePool) -> Result<(), String> {
    // Migrate in dependency order: parents first, then children
    // Each function returns a mapping of old_id -> new_id

    // ── Level 0: No FK dependencies ──
    let role_map = migrate_table(pool, "role").await?;
    let zone_map = migrate_table(pool, "zone").await?;
    let tag_map = migrate_table(pool, "tag").await?;
    let attribute_map = migrate_table(pool, "attribute").await?;
    let print_dest_map = migrate_table(pool, "print_destination").await?;
    let label_template_map = migrate_table(pool, "label_template").await?;
    let daily_report_map = migrate_table(pool, "daily_report").await?;

    // ── Level 1: Depends on Level 0 ──
    let employee_map = migrate_table(pool, "employee").await?;
    // employee.role_id → role
    update_fk(pool, "employee", "role_id", &role_map).await?;

    let category_map = migrate_table(pool, "category").await?;

    let dining_table_map = migrate_table(pool, "dining_table").await?;
    // dining_table.zone_id → zone
    update_fk(pool, "dining_table", "zone_id", &zone_map).await?;

    let attribute_option_map = migrate_table(pool, "attribute_option").await?;
    // attribute_option.attribute_id → attribute
    update_fk(pool, "attribute_option", "attribute_id", &attribute_map).await?;

    let printer_map = migrate_table(pool, "printer").await?;
    // printer.print_destination_id → print_destination
    update_fk(pool, "printer", "print_destination_id", &print_dest_map).await?;

    // label_field
    let label_field_map = migrate_table(pool, "label_field").await?;
    // label_field.template_id → label_template
    update_fk(pool, "label_field", "template_id", &label_template_map).await?;

    // daily_report sub-tables
    let tax_bk_map = migrate_table(pool, "daily_report_tax_breakdown").await?;
    update_fk(
        pool,
        "daily_report_tax_breakdown",
        "report_id",
        &daily_report_map,
    )
    .await?;
    let pay_bk_map = migrate_table(pool, "daily_report_payment_breakdown").await?;
    update_fk(
        pool,
        "daily_report_payment_breakdown",
        "report_id",
        &daily_report_map,
    )
    .await?;

    // ── Level 2: Depends on Level 1 ──
    let product_map = migrate_table(pool, "product").await?;
    // product.category_id → category
    update_fk(pool, "product", "category_id", &category_map).await?;

    let product_spec_map = migrate_table(pool, "product_spec").await?;
    // product_spec.product_id → product
    update_fk(pool, "product_spec", "product_id", &product_map).await?;

    // price_rule.created_by → employee (nullable)
    let price_rule_map = migrate_table(pool, "price_rule").await?;
    update_fk_nullable(pool, "price_rule", "created_by", &employee_map).await?;

    // shift.operator_id → employee
    let shift_map = migrate_table(pool, "shift").await?;
    update_fk(pool, "shift", "operator_id", &employee_map).await?;

    // ── Junction tables (composite PK, no own ID to migrate) ──
    update_fk(pool, "product_tag", "product_id", &product_map).await?;
    update_fk(pool, "product_tag", "tag_id", &tag_map).await?;

    update_fk(pool, "category_tag", "category_id", &category_map).await?;
    update_fk(pool, "category_tag", "tag_id", &tag_map).await?;

    update_fk(pool, "category_print_dest", "category_id", &category_map).await?;
    update_fk(
        pool,
        "category_print_dest",
        "print_destination_id",
        &print_dest_map,
    )
    .await?;

    // ── attribute_binding: owner_id depends on owner_type ──
    let ab_map = migrate_table(pool, "attribute_binding").await?;
    update_fk(pool, "attribute_binding", "attribute_id", &attribute_map).await?;
    // owner_id: split by owner_type
    update_fk_where(
        pool,
        "attribute_binding",
        "owner_id",
        &product_map,
        "owner_type = 'product'",
    )
    .await?;
    update_fk_where(
        pool,
        "attribute_binding",
        "owner_id",
        &category_map,
        "owner_type = 'category'",
    )
    .await?;

    // ── image_ref: entity_id depends on entity_type ──
    update_fk_where(
        pool,
        "image_ref",
        "entity_id",
        &product_map,
        "entity_type = 'product'",
    )
    .await?;
    update_fk_where(
        pool,
        "image_ref",
        "entity_id",
        &category_map,
        "entity_type = 'category'",
    )
    .await?;

    // ── store_info: singleton, always id=1 ──
    let store_info_map = migrate_table(pool, "store_info").await?;

    // Log summary
    let total: usize = role_map.len()
        + zone_map.len()
        + tag_map.len()
        + attribute_map.len()
        + print_dest_map.len()
        + label_template_map.len()
        + daily_report_map.len()
        + employee_map.len()
        + category_map.len()
        + dining_table_map.len()
        + attribute_option_map.len()
        + printer_map.len()
        + label_field_map.len()
        + tax_bk_map.len()
        + pay_bk_map.len()
        + product_map.len()
        + product_spec_map.len()
        + price_rule_map.len()
        + shift_map.len()
        + ab_map.len()
        + store_info_map.len();

    tracing::info!("Migrated {total} rows across all tables");
    Ok(())
}

/// Migrate all legacy IDs in a table to snowflake IDs.
/// Returns mapping of old_id → new_id for FK updates.
async fn migrate_table(pool: &SqlitePool, table: &str) -> Result<HashMap<i64, i64>, String> {
    // Safe: table names are hardcoded, not user input
    let query = format!("SELECT id FROM \"{table}\" WHERE id < ?");
    let old_ids: Vec<i64> = sqlx::query_scalar(&query)
        .bind(LEGACY_THRESHOLD)
        .fetch_all(pool)
        .await
        .map_err(|e| format!("Failed to read {table} IDs: {e}"))?;

    if old_ids.is_empty() {
        return Ok(HashMap::new());
    }

    let mut id_map = HashMap::with_capacity(old_ids.len());
    for old_id in &old_ids {
        let new_id = shared::util::snowflake_id();
        id_map.insert(*old_id, new_id);
    }

    // Update each row's primary key
    let update_sql = format!("UPDATE \"{table}\" SET id = ? WHERE id = ?");
    for (old_id, new_id) in &id_map {
        sqlx::query(&update_sql)
            .bind(new_id)
            .bind(old_id)
            .execute(pool)
            .await
            .map_err(|e| format!("Failed to update {table} id {old_id} → {new_id}: {e}"))?;
    }

    tracing::info!(
        table,
        count = id_map.len(),
        "Migrated primary keys to snowflake IDs"
    );
    Ok(id_map)
}

/// Update a NOT NULL foreign key column using the id mapping.
async fn update_fk(
    pool: &SqlitePool,
    table: &str,
    column: &str,
    id_map: &HashMap<i64, i64>,
) -> Result<(), String> {
    if id_map.is_empty() {
        return Ok(());
    }
    // Safe: table/column names are hardcoded
    let sql = format!("UPDATE \"{table}\" SET \"{column}\" = ? WHERE \"{column}\" = ?");
    for (old_id, new_id) in id_map {
        sqlx::query(&sql)
            .bind(new_id)
            .bind(old_id)
            .execute(pool)
            .await
            .map_err(|e| format!("Failed to update {table}.{column} {old_id} → {new_id}: {e}"))?;
    }
    Ok(())
}

/// Update a nullable foreign key column.
async fn update_fk_nullable(
    pool: &SqlitePool,
    table: &str,
    column: &str,
    id_map: &HashMap<i64, i64>,
) -> Result<(), String> {
    if id_map.is_empty() {
        return Ok(());
    }
    let sql = format!(
        "UPDATE \"{table}\" SET \"{column}\" = ? WHERE \"{column}\" = ? AND \"{column}\" IS NOT NULL"
    );
    for (old_id, new_id) in id_map {
        sqlx::query(&sql)
            .bind(new_id)
            .bind(old_id)
            .execute(pool)
            .await
            .map_err(|e| format!("Failed to update {table}.{column} {old_id} → {new_id}: {e}"))?;
    }
    Ok(())
}

/// Update a foreign key column with an additional WHERE condition.
async fn update_fk_where(
    pool: &SqlitePool,
    table: &str,
    column: &str,
    id_map: &HashMap<i64, i64>,
    condition: &str,
) -> Result<(), String> {
    if id_map.is_empty() {
        return Ok(());
    }
    // Safe: all parameters are hardcoded strings
    let sql =
        format!("UPDATE \"{table}\" SET \"{column}\" = ? WHERE \"{column}\" = ? AND {condition}");
    for (old_id, new_id) in id_map {
        sqlx::query(&sql)
            .bind(new_id)
            .bind(old_id)
            .execute(pool)
            .await
            .map_err(|e| {
                format!("Failed to update {table}.{column} ({condition}) {old_id} → {new_id}: {e}")
            })?;
    }
    Ok(())
}
