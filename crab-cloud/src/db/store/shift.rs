//! Shift database operations (edge → cloud sync only)

use sqlx::PgPool;

use super::BoxError;

/// Shift data from edge sync (subset of shared::models::Shift)
#[derive(serde::Deserialize)]
struct ShiftSyncData {
    operator_id: i64,
    operator_name: String,
    #[serde(default = "default_status")]
    status: String,
    start_time: i64,
    end_time: Option<i64>,
    #[serde(default)]
    starting_cash: f64,
    #[serde(default)]
    expected_cash: f64,
    actual_cash: Option<f64>,
    cash_variance: Option<f64>,
    #[serde(default)]
    abnormal_close: bool,
    last_active_at: Option<i64>,
    note: Option<String>,
    created_at: Option<i64>,
}

fn default_status() -> String {
    "OPEN".to_string()
}

// ── Edge Sync ──

pub async fn upsert_shift_from_sync(
    pool: &PgPool,
    store_id: i64,
    source_id: i64,
    data: &serde_json::Value,
    now: i64,
) -> Result<(), BoxError> {
    let shift: ShiftSyncData = serde_json::from_value(data.clone())?;
    sqlx::query(
        r#"
        INSERT INTO store_shifts (
            store_id, source_id, operator_id, operator_name, status,
            start_time, end_time, starting_cash, expected_cash,
            actual_cash, cash_variance, abnormal_close, last_active_at,
            note, created_at, updated_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)
        ON CONFLICT (store_id, source_id)
        DO UPDATE SET
            operator_name = EXCLUDED.operator_name, status = EXCLUDED.status,
            end_time = EXCLUDED.end_time,
            starting_cash = EXCLUDED.starting_cash, expected_cash = EXCLUDED.expected_cash,
            actual_cash = EXCLUDED.actual_cash, cash_variance = EXCLUDED.cash_variance,
            abnormal_close = EXCLUDED.abnormal_close, last_active_at = EXCLUDED.last_active_at,
            note = EXCLUDED.note, updated_at = EXCLUDED.updated_at
        "#,
    )
    .bind(store_id)
    .bind(source_id)
    .bind(shift.operator_id)
    .bind(&shift.operator_name)
    .bind(&shift.status)
    .bind(shift.start_time)
    .bind(shift.end_time)
    .bind(shift.starting_cash)
    .bind(shift.expected_cash)
    .bind(shift.actual_cash)
    .bind(shift.cash_variance)
    .bind(shift.abnormal_close)
    .bind(shift.last_active_at)
    .bind(&shift.note)
    .bind(shift.created_at)
    .bind(now)
    .execute(pool)
    .await?;
    Ok(())
}
