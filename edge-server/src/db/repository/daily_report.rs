//! Daily Report Repository

use super::{RepoError, RepoResult};
use shared::models::{DailyReport, DailyReportGenerate, PaymentMethodBreakdown, TaxBreakdown};
use sqlx::SqlitePool;

pub async fn find_by_id(pool: &SqlitePool, id: i64) -> RepoResult<Option<DailyReport>> {
    let mut report = sqlx::query_as::<_, DailyReport>(
        "SELECT id, business_date, total_orders, completed_orders, void_orders, total_sales, total_paid, total_unpaid, void_amount, total_tax, total_discount, total_surcharge, generated_at, generated_by_id, generated_by_name, note FROM daily_report WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    if let Some(ref mut r) = report {
        r.tax_breakdowns = find_tax_breakdowns(pool, r.id).await?;
        r.payment_breakdowns = find_payment_breakdowns(pool, r.id).await?;
    }
    Ok(report)
}

pub async fn find_by_date(pool: &SqlitePool, date: &str) -> RepoResult<Option<DailyReport>> {
    let mut report = sqlx::query_as::<_, DailyReport>(
        "SELECT id, business_date, total_orders, completed_orders, void_orders, total_sales, total_paid, total_unpaid, void_amount, total_tax, total_discount, total_surcharge, generated_at, generated_by_id, generated_by_name, note FROM daily_report WHERE business_date = ? LIMIT 1",
    )
    .bind(date)
    .fetch_optional(pool)
    .await?;

    if let Some(ref mut r) = report {
        r.tax_breakdowns = find_tax_breakdowns(pool, r.id).await?;
        r.payment_breakdowns = find_payment_breakdowns(pool, r.id).await?;
    }
    Ok(report)
}

pub async fn find_all(pool: &SqlitePool, limit: i32, offset: i32) -> RepoResult<Vec<DailyReport>> {
    let mut reports = sqlx::query_as::<_, DailyReport>(
        "SELECT id, business_date, total_orders, completed_orders, void_orders, total_sales, total_paid, total_unpaid, void_amount, total_tax, total_discount, total_surcharge, generated_at, generated_by_id, generated_by_name, note FROM daily_report ORDER BY business_date DESC LIMIT ? OFFSET ?",
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    for r in &mut reports {
        r.tax_breakdowns = find_tax_breakdowns(pool, r.id).await?;
        r.payment_breakdowns = find_payment_breakdowns(pool, r.id).await?;
    }
    Ok(reports)
}

pub async fn find_by_date_range(
    pool: &SqlitePool,
    start_date: &str,
    end_date: &str,
) -> RepoResult<Vec<DailyReport>> {
    let mut reports = sqlx::query_as::<_, DailyReport>(
        "SELECT id, business_date, total_orders, completed_orders, void_orders, total_sales, total_paid, total_unpaid, void_amount, total_tax, total_discount, total_surcharge, generated_at, generated_by_id, generated_by_name, note FROM daily_report WHERE business_date >= ? AND business_date <= ? ORDER BY business_date DESC",
    )
    .bind(start_date)
    .bind(end_date)
    .fetch_all(pool)
    .await?;

    for r in &mut reports {
        r.tax_breakdowns = find_tax_breakdowns(pool, r.id).await?;
        r.payment_breakdowns = find_payment_breakdowns(pool, r.id).await?;
    }
    Ok(reports)
}

/// Generate daily report from archived_order data
pub async fn generate(
    pool: &SqlitePool,
    data: DailyReportGenerate,
    start_millis: i64,
    end_millis: i64,
    operator_id: Option<String>,
    operator_name: Option<String>,
) -> RepoResult<DailyReport> {
    let now = shared::util::now_millis();

    // Aggregate from archived_order
    let total_orders: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM archived_order WHERE end_time >= ? AND end_time < ?",
    )
    .bind(start_millis)
    .bind(end_millis)
    .fetch_one(pool)
    .await?;

    let completed_orders: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM archived_order WHERE end_time >= ? AND end_time < ? AND status = 'COMPLETED'",
    )
    .bind(start_millis)
    .bind(end_millis)
    .fetch_one(pool)
    .await?;

    let void_orders: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM archived_order WHERE end_time >= ? AND end_time < ? AND status = 'VOID'",
    )
    .bind(start_millis)
    .bind(end_millis)
    .fetch_one(pool)
    .await?;

    let total_sales: f64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(total_amount), 0) FROM archived_order WHERE end_time >= ? AND end_time < ? AND status = 'COMPLETED'",
    )
    .bind(start_millis)
    .bind(end_millis)
    .fetch_one(pool)
    .await?;

    let total_paid: f64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(paid_amount), 0) FROM archived_order WHERE end_time >= ? AND end_time < ? AND status = 'COMPLETED'",
    )
    .bind(start_millis)
    .bind(end_millis)
    .fetch_one(pool)
    .await?;

    let void_amount: f64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(total_amount), 0) FROM archived_order WHERE end_time >= ? AND end_time < ? AND status = 'VOID'",
    )
    .bind(start_millis)
    .bind(end_millis)
    .fetch_one(pool)
    .await?;

    let total_tax: f64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(tax), 0) FROM archived_order WHERE end_time >= ? AND end_time < ? AND status = 'COMPLETED'",
    )
    .bind(start_millis)
    .bind(end_millis)
    .fetch_one(pool)
    .await?;

    let total_discount: f64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(discount_amount), 0) FROM archived_order WHERE end_time >= ? AND end_time < ? AND status = 'COMPLETED'",
    )
    .bind(start_millis)
    .bind(end_millis)
    .fetch_one(pool)
    .await?;

    let total_surcharge: f64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(surcharge_amount), 0) FROM archived_order WHERE end_time >= ? AND end_time < ? AND status = 'COMPLETED'",
    )
    .bind(start_millis)
    .bind(end_millis)
    .fetch_one(pool)
    .await?;

    let total_unpaid = total_sales - total_paid;

    // Create the report
    let report_id = sqlx::query_scalar::<_, i64>(
        "INSERT INTO daily_report (business_date, total_orders, completed_orders, void_orders, total_sales, total_paid, total_unpaid, void_amount, total_tax, total_discount, total_surcharge, generated_at, generated_by_id, generated_by_name, note) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15) RETURNING id",
    )
    .bind(&data.business_date)
    .bind(total_orders as i32)
    .bind(completed_orders as i32)
    .bind(void_orders as i32)
    .bind(total_sales)
    .bind(total_paid)
    .bind(total_unpaid)
    .bind(void_amount)
    .bind(total_tax)
    .bind(total_discount)
    .bind(total_surcharge)
    .bind(now)
    .bind(&operator_id)
    .bind(&operator_name)
    .bind(&data.note)
    .fetch_one(pool)
    .await?;

    // Generate tax breakdowns from archived_order_item
    let completed_ids_sql = "SELECT id FROM archived_order WHERE end_time >= ? AND end_time < ? AND status = 'COMPLETED'";

    // Tax breakdown by rate
    let tax_rows: Vec<(i32, f64, f64, f64, i64)> = sqlx::query_as(
        &format!(
            "SELECT tax_rate, COALESCE(SUM(quantity * unit_price), 0), COALESCE(SUM((quantity * unit_price) * tax_rate / (100 + tax_rate)), 0), COALESCE(SUM(quantity * unit_price) - SUM((quantity * unit_price) * tax_rate / (100 + tax_rate)), 0), COUNT(DISTINCT order_pk) FROM archived_order_item WHERE order_pk IN ({completed_ids_sql}) GROUP BY tax_rate ORDER BY tax_rate DESC"
        ),
    )
    .bind(start_millis)
    .bind(end_millis)
    .fetch_all(pool)
    .await?;

    for (tax_rate, gross, tax_amt, net, order_count) in &tax_rows {
        sqlx::query(
            "INSERT INTO daily_report_tax_breakdown (report_id, tax_rate, net_amount, tax_amount, gross_amount, order_count) VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(report_id)
        .bind(tax_rate)
        .bind(net)
        .bind(tax_amt)
        .bind(gross)
        .bind(*order_count as i32)
        .execute(pool)
        .await?;
    }

    // Payment breakdown by method
    let payment_rows: Vec<(String, f64, i64)> = sqlx::query_as(
        &format!(
            "SELECT method, COALESCE(SUM(amount), 0), COUNT(*) FROM archived_order_payment WHERE order_pk IN ({completed_ids_sql}) AND cancelled = 0 GROUP BY method"
        ),
    )
    .bind(start_millis)
    .bind(end_millis)
    .fetch_all(pool)
    .await?;

    for (method, amount, count) in &payment_rows {
        sqlx::query(
            "INSERT INTO daily_report_payment_breakdown (report_id, method, amount, count) VALUES (?, ?, ?, ?)",
        )
        .bind(report_id)
        .bind(method)
        .bind(amount)
        .bind(*count as i32)
        .execute(pool)
        .await?;
    }

    find_by_id(pool, report_id)
        .await?
        .ok_or_else(|| RepoError::Database("Failed to generate daily report".into()))
}

// ── Breakdowns ──────────────────────────────────────────────────────────

async fn find_tax_breakdowns(pool: &SqlitePool, report_id: i64) -> RepoResult<Vec<TaxBreakdown>> {
    let breakdowns = sqlx::query_as::<_, TaxBreakdown>(
        "SELECT id, report_id, tax_rate, net_amount, tax_amount, gross_amount, order_count FROM daily_report_tax_breakdown WHERE report_id = ? ORDER BY tax_rate DESC",
    )
    .bind(report_id)
    .fetch_all(pool)
    .await?;
    Ok(breakdowns)
}

async fn find_payment_breakdowns(
    pool: &SqlitePool,
    report_id: i64,
) -> RepoResult<Vec<PaymentMethodBreakdown>> {
    let breakdowns = sqlx::query_as::<_, PaymentMethodBreakdown>(
        "SELECT id, report_id, method, amount, count FROM daily_report_payment_breakdown WHERE report_id = ?",
    )
    .bind(report_id)
    .fetch_all(pool)
    .await?;
    Ok(breakdowns)
}
