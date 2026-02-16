//! Shift Repository

use super::{RepoError, RepoResult};
use shared::models::{Shift, ShiftClose, ShiftCreate, ShiftForceClose, ShiftUpdate};
use sqlx::SqlitePool;

fn validate_cash_amount(amount: f64, field_name: &str) -> RepoResult<()> {
    if amount < 0.0 {
        return Err(RepoError::Validation(format!(
            "{field_name} cannot be negative: {amount}"
        )));
    }
    Ok(())
}

pub async fn find_by_id(pool: &SqlitePool, id: i64) -> RepoResult<Option<Shift>> {
    let shift = sqlx::query_as::<_, Shift>(
        "SELECT id, operator_id, operator_name, status, start_time, end_time, starting_cash, expected_cash, actual_cash, cash_variance, abnormal_close, last_active_at, note, created_at, updated_at FROM shift WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    Ok(shift)
}

pub async fn create(pool: &SqlitePool, data: ShiftCreate) -> RepoResult<Shift> {
    validate_cash_amount(data.starting_cash, "Starting cash")?;

    // Global single shift: only one OPEN shift allowed at a time
    if find_any_open(pool).await?.is_some() {
        return Err(RepoError::Duplicate("A shift is already open".into()));
    }

    let now = shared::util::now_millis();
    let id = sqlx::query_scalar!(
        r#"INSERT INTO shift (operator_id, operator_name, status, start_time, starting_cash, expected_cash, abnormal_close, last_active_at, note, created_at, updated_at) VALUES (?1, ?2, 'OPEN', ?3, ?4, ?4, 0, ?3, ?5, ?3, ?3) RETURNING id as "id!""#,
        data.operator_id,
        data.operator_name,
        now,
        data.starting_cash,
        data.note
    )
    .fetch_one(pool)
    .await?;

    find_by_id(pool, id)
        .await?
        .ok_or_else(|| RepoError::Database("Failed to create shift".into()))
}

pub async fn find_any_open(pool: &SqlitePool) -> RepoResult<Option<Shift>> {
    let shift = sqlx::query_as::<_, Shift>(
        "SELECT id, operator_id, operator_name, status, start_time, end_time, starting_cash, expected_cash, actual_cash, cash_variance, abnormal_close, last_active_at, note, created_at, updated_at FROM shift WHERE status = 'OPEN' LIMIT 1",
    )
    .fetch_optional(pool)
    .await?;
    Ok(shift)
}

pub async fn find_all(pool: &SqlitePool, limit: i32, offset: i32) -> RepoResult<Vec<Shift>> {
    let shifts = sqlx::query_as::<_, Shift>(
        "SELECT id, operator_id, operator_name, status, start_time, end_time, starting_cash, expected_cash, actual_cash, cash_variance, abnormal_close, last_active_at, note, created_at, updated_at FROM shift ORDER BY start_time DESC LIMIT ? OFFSET ?",
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;
    Ok(shifts)
}

pub async fn find_by_date_range(
    pool: &SqlitePool,
    start_millis: i64,
    end_millis: i64,
) -> RepoResult<Vec<Shift>> {
    let shifts = sqlx::query_as::<_, Shift>(
        "SELECT id, operator_id, operator_name, status, start_time, end_time, starting_cash, expected_cash, actual_cash, cash_variance, abnormal_close, last_active_at, note, created_at, updated_at FROM shift WHERE start_time >= ? AND start_time < ? ORDER BY start_time DESC",
    )
    .bind(start_millis)
    .bind(end_millis)
    .fetch_all(pool)
    .await?;
    Ok(shifts)
}

pub async fn update(pool: &SqlitePool, id: i64, data: ShiftUpdate) -> RepoResult<Shift> {
    let now = shared::util::now_millis();

    // When starting_cash changes, adjust expected_cash accordingly
    let rows = sqlx::query!(
        "UPDATE shift SET starting_cash = COALESCE(?1, starting_cash), expected_cash = CASE WHEN ?1 IS NOT NULL THEN ?1 + (expected_cash - starting_cash) ELSE expected_cash END, note = COALESCE(?2, note), last_active_at = ?3, updated_at = ?3 WHERE id = ?4 AND status = 'OPEN'",
        data.starting_cash,
        data.note,
        now,
        id
    )
    .execute(pool)
    .await?;

    if rows.rows_affected() == 0 {
        return Err(RepoError::NotFound(format!(
            "Shift {id} not found or already closed"
        )));
    }
    find_by_id(pool, id)
        .await?
        .ok_or_else(|| RepoError::NotFound(format!("Shift {id} not found")))
}

pub async fn close(pool: &SqlitePool, id: i64, data: ShiftClose) -> RepoResult<Shift> {
    validate_cash_amount(data.actual_cash, "Actual cash")?;
    let now = shared::util::now_millis();

    // Atomic: compute cash_variance = actual_cash - expected_cash in SQL
    let rows = sqlx::query!(
        "UPDATE shift SET status = 'CLOSED', end_time = ?1, actual_cash = ?2, cash_variance = (?2 - expected_cash), abnormal_close = 0, note = COALESCE(?3, note), last_active_at = ?1, updated_at = ?1 WHERE id = ?4 AND status = 'OPEN'",
        now,
        data.actual_cash,
        data.note,
        id
    )
    .execute(pool)
    .await?;

    if rows.rows_affected() == 0 {
        return Err(RepoError::NotFound(format!(
            "Shift {id} not found or already closed"
        )));
    }
    find_by_id(pool, id)
        .await?
        .ok_or_else(|| RepoError::NotFound(format!("Shift {id} not found")))
}

pub async fn force_close(pool: &SqlitePool, id: i64, data: ShiftForceClose) -> RepoResult<Shift> {
    let now = shared::util::now_millis();
    let note = data
        .note
        .as_deref()
        .unwrap_or("Force closed without cash counting");

    let rows = sqlx::query!(
        "UPDATE shift SET status = 'CLOSED', end_time = ?1, abnormal_close = 1, note = ?2, last_active_at = ?1, updated_at = ?1 WHERE id = ?3 AND status = 'OPEN'",
        now,
        note,
        id
    )
    .execute(pool)
    .await?;

    if rows.rows_affected() == 0 {
        return Err(RepoError::NotFound(format!(
            "Shift {id} not found or already closed"
        )));
    }
    find_by_id(pool, id)
        .await?
        .ok_or_else(|| RepoError::NotFound(format!("Shift {id} not found")))
}

pub async fn find_stale_shifts(
    pool: &SqlitePool,
    business_day_start: i64,
) -> RepoResult<Vec<Shift>> {
    let shifts = sqlx::query_as::<_, Shift>(
        "SELECT id, operator_id, operator_name, status, start_time, end_time, starting_cash, expected_cash, actual_cash, cash_variance, abnormal_close, last_active_at, note, created_at, updated_at FROM shift WHERE status = 'OPEN' AND start_time < ?",
    )
    .bind(business_day_start)
    .fetch_all(pool)
    .await?;
    Ok(shifts)
}

pub async fn add_cash_payment(pool: &SqlitePool, amount: f64) -> RepoResult<()> {
    let now = shared::util::now_millis();
    sqlx::query!(
        "UPDATE shift SET expected_cash = expected_cash + ?1, last_active_at = ?2, updated_at = ?2 WHERE status = 'OPEN'",
        amount,
        now,
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn heartbeat(pool: &SqlitePool, id: i64) -> RepoResult<()> {
    let now = shared::util::now_millis();
    sqlx::query!(
        "UPDATE shift SET last_active_at = ? WHERE id = ? AND status = 'OPEN'",
        now,
        id
    )
    .execute(pool)
    .await?;
    Ok(())
}
