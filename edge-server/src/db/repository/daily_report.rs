//! Daily Report Repository

use super::{BaseRepository, RepoError, RepoResult};
use crate::db::models::{DailyReport, DailyReportGenerate};
use chrono::NaiveDate;
use surrealdb::engine::local::Db;
use surrealdb::{RecordId, Surreal};

/// Validate date format (YYYY-MM-DD)
fn validate_date(date: &str) -> RepoResult<NaiveDate> {
    NaiveDate::parse_from_str(date, "%Y-%m-%d")
        .map_err(|_| RepoError::Validation(format!("Invalid date format: {}", date)))
}

/// Validate date is not in the future
fn validate_not_future_date(date: &str, tz: chrono_tz::Tz) -> RepoResult<()> {
    let parsed_date = validate_date(date)?;
    let today = chrono::Utc::now().with_timezone(&tz).date_naive();

    if parsed_date > today {
        return Err(RepoError::Validation(format!(
            "Cannot generate report for future date: {}",
            date
        )));
    }
    Ok(())
}

#[derive(Clone)]
pub struct DailyReportRepository {
    base: BaseRepository,
    tz: chrono_tz::Tz,
}

impl DailyReportRepository {
    pub fn new(db: Surreal<Db>, tz: chrono_tz::Tz) -> Self {
        Self {
            base: BaseRepository::new(db),
            tz,
        }
    }

    /// Generate daily report for a specific date
    pub async fn generate(
        &self,
        data: DailyReportGenerate,
        operator_id: Option<String>,
        operator_name: Option<String>,
    ) -> RepoResult<DailyReport> {
        // Validate date format and ensure not future
        validate_not_future_date(&data.business_date, self.tz)?;

        // Check if report already exists
        if self.find_by_date(&data.business_date).await?.is_some() {
            return Err(RepoError::Duplicate(format!(
                "Daily report for {} already exists",
                data.business_date
            )));
        }

        let parsed_date = validate_date(&data.business_date)?;
        let start_millis = parsed_date.and_hms_opt(0, 0, 0).unwrap().and_utc().timestamp_millis();
        let end_millis = parsed_date.and_hms_opt(23, 59, 59).unwrap().and_utc().timestamp_millis();

        // Complex aggregation query for daily report
        let mut result = self
            .base
            .db()
            .query(
                r#"
                -- Get all orders for the day (archived orders)
                LET $all_orders = SELECT * FROM order
                    WHERE created_at >= $start
                    AND created_at <= $end;

                -- Filter by status
                LET $completed = SELECT * FROM $all_orders WHERE status = 'COMPLETED';
                LET $void = SELECT * FROM $all_orders WHERE status = 'VOID';

                -- Calculate totals
                LET $total_sales = math::sum($completed.total_amount) OR 0;
                LET $total_paid = math::sum($completed.paid_amount) OR 0;
                LET $total_unpaid = $total_sales - $total_paid;
                LET $void_amount = math::sum($void.total_amount) OR 0;
                LET $total_tax = math::sum($completed.tax) OR 0;
                LET $total_discount = math::sum($completed.discount_amount) OR 0;
                LET $total_surcharge = math::sum($completed.surcharge_amount) OR 0;

                -- Payment breakdowns via graph edge traversal
                LET $completed_ids = (SELECT VALUE id FROM $completed);
                LET $payments = (
                    SELECT
                        out.method AS method,
                        out.amount AS amount
                    FROM has_payment
                    WHERE in IN $completed_ids
                    AND out.cancelled = false
                );
                LET $payment_breakdown = (
                    SELECT
                        method,
                        math::sum(amount) AS amount,
                        count() AS count
                    FROM $payments
                    GROUP BY method
                );

                -- Tax breakdowns by rate (Spain IVA: 0%, 4%, 10%, 21%)
                -- Get order items via graph edge traversal
                LET $order_items = (
                    SELECT
                        out.tax_rate AS tax_rate,
                        (out.quantity * out.unit_price) AS gross_amount,
                        ((out.quantity * out.unit_price) * out.tax_rate / (100 + out.tax_rate)) AS tax_amount,
                        in AS order_id
                    FROM has_item
                    WHERE in IN $completed_ids
                );

                LET $tax_breakdown = (
                    SELECT
                        tax_rate,
                        math::sum(gross_amount) AS gross_amount,
                        math::sum(tax_amount) AS tax_amount,
                        (math::sum(gross_amount) - math::sum(tax_amount)) AS net_amount,
                        array::len(array::distinct(array::group(order_id))) AS order_count
                    FROM $order_items
                    GROUP BY tax_rate
                    ORDER BY tax_rate DESC
                );

                -- Create the report
                CREATE daily_report SET
                    business_date = $date,
                    total_orders = count($all_orders),
                    completed_orders = count($completed),
                    void_orders = count($void),
                    total_sales = $total_sales,
                    total_paid = $total_paid,
                    total_unpaid = $total_unpaid,
                    void_amount = $void_amount,
                    total_tax = $total_tax,
                    total_discount = $total_discount,
                    total_surcharge = $total_surcharge,
                    tax_breakdowns = $tax_breakdown,
                    payment_breakdowns = $payment_breakdown,
                    generated_at = $now,
                    generated_by_id = $gen_id,
                    generated_by_name = $gen_name,
                    note = $note
                RETURN AFTER
            "#,
            )
            .bind(("date", data.business_date))
            .bind(("start", start_millis))
            .bind(("end", end_millis))
            .bind(("gen_id", operator_id))
            .bind(("gen_name", operator_name))
            .bind(("note", data.note))
            .bind(("now", shared::util::now_millis()))
            .await?;

        let reports: Vec<DailyReport> = result.take(0)?;
        reports
            .into_iter()
            .next()
            .ok_or_else(|| RepoError::Database("Failed to generate daily report".to_string()))
    }

    /// Find report by date
    pub async fn find_by_date(&self, date: &str) -> RepoResult<Option<DailyReport>> {
        let mut result = self
            .base
            .db()
            .query("SELECT * FROM daily_report WHERE business_date = $date LIMIT 1")
            .bind(("date", date.to_string()))
            .await?;

        let reports: Vec<DailyReport> = result.take(0)?;
        Ok(reports.into_iter().next())
    }

    /// Find report by id
    pub async fn find_by_id(&self, id: &str) -> RepoResult<Option<DailyReport>> {
        let record_id: RecordId = id
            .parse()
            .map_err(|_| RepoError::Validation(format!("Invalid ID: {}", id)))?;
        let report: Option<DailyReport> = self.base.db().select(record_id).await?;
        Ok(report)
    }

    /// Find all reports (paginated)
    pub async fn find_all(&self, limit: i32, offset: i32) -> RepoResult<Vec<DailyReport>> {
        let mut result = self
            .base
            .db()
            .query(
                "SELECT * FROM daily_report ORDER BY business_date DESC LIMIT $limit START $offset",
            )
            .bind(("limit", limit))
            .bind(("offset", offset))
            .await?;

        let reports: Vec<DailyReport> = result.take(0)?;
        Ok(reports)
    }

    /// Find reports by date range
    pub async fn find_by_date_range(
        &self,
        start_date: &str,
        end_date: &str,
    ) -> RepoResult<Vec<DailyReport>> {
        // Validate date formats
        validate_date(start_date)?;
        validate_date(end_date)?;

        let mut result = self
            .base
            .db()
            .query(
                r#"
                SELECT * FROM daily_report
                WHERE business_date >= $start AND business_date <= $end
                ORDER BY business_date DESC
            "#,
            )
            .bind(("start", start_date.to_string()))
            .bind(("end", end_date.to_string()))
            .await?;

        let reports: Vec<DailyReport> = result.take(0)?;
        Ok(reports)
    }

    /// Delete report (admin only)
    pub async fn delete(&self, id: &str) -> RepoResult<bool> {
        let record_id: RecordId = id
            .parse()
            .map_err(|_| RepoError::Validation(format!("Invalid ID: {}", id)))?;

        self.base
            .db()
            .query("DELETE $id")
            .bind(("id", record_id))
            .await?;

        Ok(true)
    }
}
