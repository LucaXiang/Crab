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

    batch_load_breakdowns(pool, &mut reports).await?;
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

    batch_load_breakdowns(pool, &mut reports).await?;
    Ok(reports)
}

/// Generate daily report from archived_order data
pub async fn generate(
    pool: &SqlitePool,
    data: DailyReportGenerate,
    start_millis: i64,
    end_millis: i64,
    operator_id: Option<i64>,
    operator_name: Option<String>,
) -> RepoResult<DailyReport> {
    let now = shared::util::now_millis();

    // Aggregate from archived_order
    let total_orders: i64 = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM archived_order WHERE end_time >= ? AND end_time < ?",
        start_millis,
        end_millis,
    )
    .fetch_one(pool)
    .await?;

    let completed_orders: i64 = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM archived_order WHERE end_time >= ? AND end_time < ? AND status = 'COMPLETED'",
        start_millis,
        end_millis,
    )
    .fetch_one(pool)
    .await?;

    let void_orders: i64 = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM archived_order WHERE end_time >= ? AND end_time < ? AND status = 'VOID'",
        start_millis,
        end_millis,
    )
    .fetch_one(pool)
    .await?;

    let total_sales: f64 = sqlx::query_scalar!(
        "SELECT COALESCE(SUM(total_amount), 0.0) FROM archived_order WHERE end_time >= ? AND end_time < ? AND status = 'COMPLETED'",
        start_millis,
        end_millis,
    )
    .fetch_one(pool)
    .await?;

    let total_paid: f64 = sqlx::query_scalar!(
        "SELECT COALESCE(SUM(paid_amount), 0.0) FROM archived_order WHERE end_time >= ? AND end_time < ? AND status = 'COMPLETED'",
        start_millis,
        end_millis,
    )
    .fetch_one(pool)
    .await?;

    let void_amount: f64 = sqlx::query_scalar!(
        "SELECT COALESCE(SUM(total_amount), 0.0) FROM archived_order WHERE end_time >= ? AND end_time < ? AND status = 'VOID'",
        start_millis,
        end_millis,
    )
    .fetch_one(pool)
    .await?;

    let total_tax: f64 = sqlx::query_scalar!(
        "SELECT COALESCE(SUM(tax), 0.0) FROM archived_order WHERE end_time >= ? AND end_time < ? AND status = 'COMPLETED'",
        start_millis,
        end_millis,
    )
    .fetch_one(pool)
    .await?;

    let total_discount: f64 = sqlx::query_scalar!(
        "SELECT COALESCE(SUM(discount_amount), 0.0) FROM archived_order WHERE end_time >= ? AND end_time < ? AND status = 'COMPLETED'",
        start_millis,
        end_millis,
    )
    .fetch_one(pool)
    .await?;

    let total_surcharge: f64 = sqlx::query_scalar!(
        "SELECT COALESCE(SUM(surcharge_amount), 0.0) FROM archived_order WHERE end_time >= ? AND end_time < ? AND status = 'COMPLETED'",
        start_millis,
        end_millis,
    )
    .fetch_one(pool)
    .await?;

    let total_unpaid = total_sales - total_paid;

    // Create report + breakdowns in a single transaction
    let mut tx = pool.begin().await?;

    let report_id = shared::util::snowflake_id();
    sqlx::query(
        "INSERT INTO daily_report (id, business_date, total_orders, completed_orders, void_orders, total_sales, total_paid, total_unpaid, void_amount, total_tax, total_discount, total_surcharge, generated_at, generated_by_id, generated_by_name, note) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
    )
    .bind(report_id)
    .bind(&data.business_date)
    .bind(total_orders)
    .bind(completed_orders)
    .bind(void_orders)
    .bind(total_sales)
    .bind(total_paid)
    .bind(total_unpaid)
    .bind(void_amount)
    .bind(total_tax)
    .bind(total_discount)
    .bind(total_surcharge)
    .bind(now)
    .bind(operator_id)
    .bind(&operator_name)
    .bind(&data.note)
    .execute(&mut *tx)
    .await?;

    // Tax breakdown by rate (use subquery directly — both strings are compile-time constants)
    let tax_rows: Vec<(i32, f64, f64, f64, i64)> = sqlx::query_as(
        "SELECT tax_rate, COALESCE(SUM(quantity * unit_price), 0.0), COALESCE(SUM((quantity * unit_price) * tax_rate / (100 + tax_rate)), 0.0), COALESCE(SUM(quantity * unit_price) - SUM((quantity * unit_price) * tax_rate / (100 + tax_rate)), 0.0), COUNT(DISTINCT order_pk) FROM archived_order_item WHERE order_pk IN (SELECT id FROM archived_order WHERE end_time >= ? AND end_time < ? AND status = 'COMPLETED') GROUP BY tax_rate ORDER BY tax_rate DESC",
    )
    .bind(start_millis)
    .bind(end_millis)
    .fetch_all(&mut *tx)
    .await?;

    for (tax_rate, gross, tax_amt, net, order_count) in &tax_rows {
        let tb_id = shared::util::snowflake_id();
        sqlx::query(
            "INSERT INTO daily_report_tax_breakdown (id, report_id, tax_rate, net_amount, tax_amount, gross_amount, order_count) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        )
        .bind(tb_id)
        .bind(report_id)
        .bind(tax_rate)
        .bind(net)
        .bind(tax_amt)
        .bind(gross)
        .bind(order_count)
        .execute(&mut *tx)
        .await?;
    }

    // Payment breakdown by method
    let payment_rows: Vec<(String, f64, i64)> = sqlx::query_as(
        "SELECT method, COALESCE(SUM(amount), 0.0), COUNT(*) FROM archived_order_payment WHERE order_pk IN (SELECT id FROM archived_order WHERE end_time >= ? AND end_time < ? AND status = 'COMPLETED') AND cancelled = 0 GROUP BY method",
    )
    .bind(start_millis)
    .bind(end_millis)
    .fetch_all(&mut *tx)
    .await?;

    for (method, amount, count) in &payment_rows {
        let pb_id = shared::util::snowflake_id();
        sqlx::query(
            "INSERT INTO daily_report_payment_breakdown (id, report_id, method, amount, count) VALUES (?1, ?2, ?3, ?4, ?5)",
        )
        .bind(pb_id)
        .bind(report_id)
        .bind(method)
        .bind(amount)
        .bind(count)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;

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

/// Batch load tax + payment breakdowns for multiple reports (eliminates N+1)
async fn batch_load_breakdowns(pool: &SqlitePool, reports: &mut [DailyReport]) -> RepoResult<()> {
    if reports.is_empty() {
        return Ok(());
    }
    let ids: Vec<i64> = reports.iter().map(|r| r.id).collect();
    let placeholders = ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");

    // Tax breakdowns
    let tax_sql = format!(
        "SELECT id, report_id, tax_rate, net_amount, tax_amount, gross_amount, order_count FROM daily_report_tax_breakdown WHERE report_id IN ({placeholders}) ORDER BY tax_rate DESC"
    );
    let mut tax_query = sqlx::query_as::<_, TaxBreakdown>(&tax_sql);
    for id in &ids {
        tax_query = tax_query.bind(id);
    }
    let all_tax = tax_query.fetch_all(pool).await?;

    // Payment breakdowns
    let pay_sql = format!(
        "SELECT id, report_id, method, amount, count FROM daily_report_payment_breakdown WHERE report_id IN ({placeholders})"
    );
    let mut pay_query = sqlx::query_as::<_, PaymentMethodBreakdown>(&pay_sql);
    for id in &ids {
        pay_query = pay_query.bind(id);
    }
    let all_pay = pay_query.fetch_all(pool).await?;

    let mut tax_map: std::collections::HashMap<i64, Vec<TaxBreakdown>> =
        std::collections::HashMap::new();
    for t in all_tax {
        tax_map.entry(t.report_id).or_default().push(t);
    }
    let mut pay_map: std::collections::HashMap<i64, Vec<PaymentMethodBreakdown>> =
        std::collections::HashMap::new();
    for p in all_pay {
        pay_map.entry(p.report_id).or_default().push(p);
    }
    for r in reports.iter_mut() {
        r.tax_breakdowns = tax_map.remove(&r.id).unwrap_or_default();
        r.payment_breakdowns = pay_map.remove(&r.id).unwrap_or_default();
    }
    Ok(())
}
