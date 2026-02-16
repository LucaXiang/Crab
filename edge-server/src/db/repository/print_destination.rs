//! Print Destination Repository

use super::{RepoError, RepoResult};
use shared::models::{PrintDestination, PrintDestinationCreate, PrintDestinationUpdate, Printer};
use sqlx::SqlitePool;

pub async fn find_all(pool: &SqlitePool) -> RepoResult<Vec<PrintDestination>> {
    let mut dests = sqlx::query_as::<_, PrintDestination>(
        "SELECT id, name, description, purpose, is_active FROM print_destination WHERE is_active = 1 ORDER BY name",
    )
    .fetch_all(pool)
    .await?;

    batch_load_printers(pool, &mut dests).await?;
    Ok(dests)
}

pub async fn find_all_with_inactive(pool: &SqlitePool) -> RepoResult<Vec<PrintDestination>> {
    let mut dests = sqlx::query_as::<_, PrintDestination>(
        "SELECT id, name, description, purpose, is_active FROM print_destination ORDER BY name",
    )
    .fetch_all(pool)
    .await?;

    batch_load_printers(pool, &mut dests).await?;
    Ok(dests)
}

pub async fn find_by_id(pool: &SqlitePool, id: i64) -> RepoResult<Option<PrintDestination>> {
    let mut dest = sqlx::query_as::<_, PrintDestination>(
        "SELECT id, name, description, purpose, is_active FROM print_destination WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    if let Some(ref mut d) = dest {
        d.printers = find_printers(pool, d.id).await?;
    }
    Ok(dest)
}

pub async fn find_by_name(pool: &SqlitePool, name: &str) -> RepoResult<Option<PrintDestination>> {
    let mut dest = sqlx::query_as::<_, PrintDestination>(
        "SELECT id, name, description, purpose, is_active FROM print_destination WHERE name = ? LIMIT 1",
    )
    .bind(name)
    .fetch_optional(pool)
    .await?;

    if let Some(ref mut d) = dest {
        d.printers = find_printers(pool, d.id).await?;
    }
    Ok(dest)
}

pub async fn create(
    pool: &SqlitePool,
    data: PrintDestinationCreate,
) -> RepoResult<PrintDestination> {
    let mut tx = pool.begin().await?;

    let id = sqlx::query_scalar!(
        r#"INSERT INTO print_destination (name, description, purpose, is_active) VALUES (?, ?, ?, ?) RETURNING id as "id!""#,
        data.name,
        data.description,
        data.purpose,
        data.is_active
    )
    .fetch_one(&mut *tx)
    .await?;

    // Create printers
    for printer in &data.printers {
        sqlx::query!(
            "INSERT INTO printer (print_destination_id, connection, protocol, ip, port, driver_name, priority, is_active) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            id,
            printer.connection,
            printer.protocol,
            printer.ip,
            printer.port,
            printer.driver_name,
            printer.priority,
            printer.is_active
        )
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;

    find_by_id(pool, id)
        .await?
        .ok_or_else(|| RepoError::Database("Failed to create print destination".into()))
}

pub async fn update(
    pool: &SqlitePool,
    id: i64,
    data: PrintDestinationUpdate,
) -> RepoResult<PrintDestination> {
    let rows = sqlx::query!(
        "UPDATE print_destination SET name = COALESCE(?1, name), description = COALESCE(?2, description), purpose = COALESCE(?3, purpose), is_active = COALESCE(?4, is_active) WHERE id = ?5",
        data.name,
        data.description,
        data.purpose,
        data.is_active,
        id
    )
    .execute(pool)
    .await?;

    if rows.rows_affected() == 0 {
        return Err(RepoError::NotFound(format!(
            "Print destination {id} not found"
        )));
    }

    // Replace printers if provided (atomic: delete + re-create in transaction)
    if let Some(printers) = &data.printers {
        let mut tx = pool.begin().await?;
        sqlx::query!("DELETE FROM printer WHERE print_destination_id = ?", id)
            .execute(&mut *tx)
            .await?;
        for printer in printers {
            sqlx::query!(
                "INSERT INTO printer (print_destination_id, connection, protocol, ip, port, driver_name, priority, is_active) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                id,
                printer.connection,
                printer.protocol,
                printer.ip,
                printer.port,
                printer.driver_name,
                printer.priority,
                printer.is_active
            )
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
    }

    find_by_id(pool, id)
        .await?
        .ok_or_else(|| RepoError::NotFound(format!("Print destination {id} not found")))
}

pub async fn delete(pool: &SqlitePool, id: i64) -> RepoResult<bool> {
    // Printers cascade via FK
    sqlx::query!("DELETE FROM print_destination WHERE id = ?", id)
        .execute(pool)
        .await?;
    Ok(true)
}

// ── Printers ────────────────────────────────────────────────────────────

async fn find_printers(pool: &SqlitePool, dest_id: i64) -> RepoResult<Vec<Printer>> {
    let printers = sqlx::query_as::<_, Printer>(
        "SELECT id, print_destination_id, connection, protocol, ip, port, driver_name, priority, is_active FROM printer WHERE print_destination_id = ? ORDER BY priority",
    )
    .bind(dest_id)
    .fetch_all(pool)
    .await?;
    Ok(printers)
}

/// Batch load printers for multiple destinations (eliminates N+1)
async fn batch_load_printers(pool: &SqlitePool, dests: &mut [PrintDestination]) -> RepoResult<()> {
    if dests.is_empty() {
        return Ok(());
    }
    let ids: Vec<i64> = dests.iter().map(|d| d.id).collect();
    let placeholders = ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    let sql = format!(
        "SELECT id, print_destination_id, connection, protocol, ip, port, driver_name, priority, is_active FROM printer WHERE print_destination_id IN ({placeholders}) ORDER BY priority"
    );
    let mut query = sqlx::query_as::<_, Printer>(&sql);
    for id in &ids {
        query = query.bind(id);
    }
    let all_printers = query.fetch_all(pool).await?;

    let mut map: std::collections::HashMap<i64, Vec<Printer>> = std::collections::HashMap::new();
    for p in all_printers {
        map.entry(p.print_destination_id).or_default().push(p);
    }
    for dest in dests.iter_mut() {
        dest.printers = map.remove(&dest.id).unwrap_or_default();
    }
    Ok(())
}
