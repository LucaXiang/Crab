//! Shift Repository

use super::{BaseRepository, RepoError, RepoResult};
use crate::db::models::{Shift, ShiftClose, ShiftCreate, ShiftForceClose, ShiftUpdate};
use chrono::NaiveDate;
use surrealdb::engine::local::Db;
use surrealdb::{RecordId, Surreal};

/// Validate date format (YYYY-MM-DD)
fn validate_date(date: &str) -> RepoResult<NaiveDate> {
    NaiveDate::parse_from_str(date, "%Y-%m-%d")
        .map_err(|_| RepoError::Validation(format!("Invalid date format: {}", date)))
}

/// Validate cash amount is non-negative
fn validate_cash_amount(amount: f64, field_name: &str) -> RepoResult<()> {
    if amount < 0.0 {
        return Err(RepoError::Validation(format!(
            "{} cannot be negative: {}",
            field_name, amount
        )));
    }
    Ok(())
}

#[derive(Clone)]
pub struct ShiftRepository {
    base: BaseRepository,
}

impl ShiftRepository {
    pub fn new(db: Surreal<Db>) -> Self {
        Self {
            base: BaseRepository::new(db),
        }
    }

    /// Create a new shift (open shift)
    pub async fn create(&self, data: ShiftCreate) -> RepoResult<Shift> {
        let operator_id: RecordId = data
            .operator_id
            .parse()
            .map_err(|_| RepoError::Validation("Invalid operator_id".to_string()))?;

        // Validate starting cash is non-negative
        validate_cash_amount(data.starting_cash, "Starting cash")?;

        // Check if operator already has an open shift
        let existing = self.find_open_by_operator(&data.operator_id).await?;
        if existing.is_some() {
            return Err(RepoError::Duplicate(
                "Operator already has an open shift".to_string(),
            ));
        }

        // Create new shift
        let mut result = self
            .base
            .db()
            .query(
                r#"
                CREATE shift SET
                    operator_id = $operator_id,
                    operator_name = $operator_name,
                    status = 'OPEN',
                    start_time = $now,
                    starting_cash = $starting_cash,
                    expected_cash = $starting_cash,
                    abnormal_close = false,
                    last_active_at = $now,
                    note = $note,
                    created_at = $now,
                    updated_at = $now
                RETURN AFTER;
            "#,
            )
            .bind(("operator_id", operator_id))
            .bind(("operator_name", data.operator_name))
            .bind(("starting_cash", data.starting_cash))
            .bind(("note", data.note))
            .bind(("now", shared::util::now_millis()))
            .await?;

        let shifts: Vec<Shift> = result.take(0)?;
        shifts.into_iter().next().ok_or_else(|| {
            RepoError::Database("Failed to create shift".to_string())
        })
    }

    /// Find shift by id
    pub async fn find_by_id(&self, id: &str) -> RepoResult<Option<Shift>> {
        let record_id: RecordId = id
            .parse()
            .map_err(|_| RepoError::Validation(format!("Invalid ID: {}", id)))?;
        let shift: Option<Shift> = self.base.db().select(record_id).await?;
        Ok(shift)
    }

    /// Find open shift by operator
    pub async fn find_open_by_operator(&self, operator_id: &str) -> RepoResult<Option<Shift>> {
        let operator_rid: RecordId = operator_id
            .parse()
            .map_err(|_| RepoError::Validation("Invalid operator_id".to_string()))?;

        let mut result = self
            .base
            .db()
            .query("SELECT * FROM shift WHERE operator_id = $op AND status = 'OPEN' LIMIT 1")
            .bind(("op", operator_rid))
            .await?;

        let shifts: Vec<Shift> = result.take(0)?;
        Ok(shifts.into_iter().next())
    }

    /// Find any open shift (for startup recovery)
    pub async fn find_any_open(&self) -> RepoResult<Option<Shift>> {
        let mut result = self
            .base
            .db()
            .query("SELECT * FROM shift WHERE status = 'OPEN' LIMIT 1")
            .await?;

        let shifts: Vec<Shift> = result.take(0)?;
        Ok(shifts.into_iter().next())
    }

    /// Find all shifts (paginated, newest first)
    pub async fn find_all(&self, limit: i32, offset: i32) -> RepoResult<Vec<Shift>> {
        let mut result = self
            .base
            .db()
            .query("SELECT * FROM shift ORDER BY start_time DESC LIMIT $limit START $offset")
            .bind(("limit", limit))
            .bind(("offset", offset))
            .await?;

        let shifts: Vec<Shift> = result.take(0)?;
        Ok(shifts)
    }

    /// Find shifts by date range
    pub async fn find_by_date_range(
        &self,
        start_date: &str,
        end_date: &str,
    ) -> RepoResult<Vec<Shift>> {
        // Validate date formats
        let start_parsed = validate_date(start_date)?;
        let end_parsed = validate_date(end_date)?;

        let start_millis = start_parsed.and_hms_opt(0, 0, 0).unwrap().and_utc().timestamp_millis();
        let end_millis = end_parsed.and_hms_opt(23, 59, 59).unwrap().and_utc().timestamp_millis();

        let mut result = self
            .base
            .db()
            .query(
                r#"
                SELECT * FROM shift
                WHERE start_time >= $start AND start_time <= $end
                ORDER BY start_time DESC
            "#,
            )
            .bind(("start", start_millis))
            .bind(("end", end_millis))
            .await?;

        let shifts: Vec<Shift> = result.take(0)?;
        Ok(shifts)
    }

    /// Update shift (only allowed when OPEN)
    pub async fn update(&self, id: &str, data: ShiftUpdate) -> RepoResult<Shift> {
        let record_id: RecordId = id
            .parse()
            .map_err(|_| RepoError::Validation(format!("Invalid ID: {}", id)))?;

        // Build update query
        let mut result = self
            .base
            .db()
            .query(
                r#"
                UPDATE shift SET
                    starting_cash = IF $starting_cash != NONE THEN $starting_cash ELSE starting_cash END,
                    expected_cash = IF $starting_cash != NONE THEN $starting_cash + (expected_cash - starting_cash) ELSE expected_cash END,
                    note = IF $note != NONE THEN $note ELSE note END,
                    last_active_at = $now,
                    updated_at = $now
                WHERE id = $id AND status = 'OPEN'
                RETURN AFTER
            "#,
            )
            .bind(("id", record_id))
            .bind(("starting_cash", data.starting_cash))
            .bind(("note", data.note))
            .bind(("now", shared::util::now_millis()))
            .await?;

        let shifts: Vec<Shift> = result.take(0)?;
        shifts.into_iter().next().ok_or_else(|| {
            RepoError::NotFound(format!("Shift {} not found or already closed", id))
        })
    }

    /// Close shift (正常收班)
    pub async fn close(&self, id: &str, data: ShiftClose) -> RepoResult<Shift> {
        let record_id: RecordId = id
            .parse()
            .map_err(|_| RepoError::Validation(format!("Invalid ID: {}", id)))?;

        // Validate actual cash is non-negative
        validate_cash_amount(data.actual_cash, "Actual cash")?;

        let mut result = self
            .base
            .db()
            .query(
                r#"
                UPDATE shift SET
                    status = 'CLOSED',
                    end_time = $now,
                    actual_cash = $actual_cash,
                    cash_variance = $actual_cash - expected_cash,
                    abnormal_close = false,
                    note = IF $note != NONE THEN $note ELSE note END,
                    last_active_at = $now,
                    updated_at = $now
                WHERE id = $id AND status = 'OPEN'
                RETURN AFTER
            "#,
            )
            .bind(("id", record_id))
            .bind(("actual_cash", data.actual_cash))
            .bind(("note", data.note))
            .bind(("now", shared::util::now_millis()))
            .await?;

        let shifts: Vec<Shift> = result.take(0)?;
        shifts.into_iter().next().ok_or_else(|| {
            RepoError::NotFound(format!("Shift {} not found or already closed", id))
        })
    }

    /// Force close shift (强制关闭，不盘点)
    pub async fn force_close(&self, id: &str, data: ShiftForceClose) -> RepoResult<Shift> {
        let record_id: RecordId = id
            .parse()
            .map_err(|_| RepoError::Validation(format!("Invalid ID: {}", id)))?;

        let mut result = self
            .base
            .db()
            .query(
                r#"
                UPDATE shift SET
                    status = 'CLOSED',
                    end_time = $now,
                    abnormal_close = true,
                    note = IF $note != NONE THEN $note ELSE '强制关闭，未盘点现金' END,
                    last_active_at = $now,
                    updated_at = $now
                WHERE id = $id AND status = 'OPEN'
                RETURN AFTER
            "#,
            )
            .bind(("id", record_id))
            .bind(("note", data.note))
            .bind(("now", shared::util::now_millis()))
            .await?;

        let shifts: Vec<Shift> = result.take(0)?;
        shifts.into_iter().next().ok_or_else(|| {
            RepoError::NotFound(format!("Shift {} not found or already closed", id))
        })
    }

    /// Recover stale shifts (启动时自动关闭跨营业日的班次)
    ///
    /// `business_day_start` - 当前营业日开始时间 (Unix millis)
    /// 例如: 如果 cutoff 是 06:00，当前时间是 2024-01-15 10:00
    ///       则 business_day_start = millis of 2024-01-15T06:00:00Z
    ///       如果当前时间是 2024-01-15 03:00 (凌晨)
    ///       则 business_day_start = millis of 2024-01-14T06:00:00Z (昨天的 06:00)
    pub async fn recover_stale_shifts(&self, business_day_start: i64) -> RepoResult<Vec<Shift>> {
        let mut result = self
            .base
            .db()
            .query(
                r#"
                UPDATE shift SET
                    status = 'CLOSED',
                    end_time = $now,
                    abnormal_close = true,
                    note = '跨营业日自动结算',
                    updated_at = $now
                WHERE status = 'OPEN'
                AND start_time < $business_day_start
                RETURN AFTER
            "#,
            )
            .bind(("business_day_start", business_day_start))
            .bind(("now", shared::util::now_millis()))
            .await?;

        let recovered: Vec<Shift> = result.take(0)?;
        Ok(recovered)
    }

    /// Update expected_cash when cash payment is added
    pub async fn add_cash_payment(&self, operator_id: &str, amount: f64) -> RepoResult<()> {
        let operator_rid: RecordId = operator_id
            .parse()
            .map_err(|_| RepoError::Validation("Invalid operator_id".to_string()))?;

        self.base
            .db()
            .query(
                r#"
                UPDATE shift SET
                    expected_cash = expected_cash + $amount,
                    last_active_at = $now,
                    updated_at = $now
                WHERE operator_id = $op AND status = 'OPEN'
            "#,
            )
            .bind(("op", operator_rid))
            .bind(("amount", amount))
            .bind(("now", shared::util::now_millis()))
            .await?;

        Ok(())
    }

    /// Update heartbeat (last_active_at)
    pub async fn heartbeat(&self, id: &str) -> RepoResult<()> {
        let record_id: RecordId = id
            .parse()
            .map_err(|_| RepoError::Validation(format!("Invalid ID: {}", id)))?;

        self.base
            .db()
            .query(
                r#"
                UPDATE shift SET
                    last_active_at = $now
                WHERE id = $id AND status = 'OPEN'
            "#,
            )
            .bind(("id", record_id))
            .bind(("now", shared::util::now_millis()))
            .await?;

        Ok(())
    }
}
