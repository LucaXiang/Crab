//! Daily Report Repository

use super::{RepoError, RepoResult};
use shared::models::{DailyReport, DailyReportGenerate, ShiftBreakdown};
use sqlx::SqlitePool;

type ShiftAggRow = (Option<i64>, i64, i64, i64, f64, f64, f64, f64, f64, f64);
type ShiftMetaRow = (
    i64,
    String,
    String,
    i64,
    Option<i64>,
    f64,
    f64,
    Option<f64>,
    Option<f64>,
    bool,
);

const SELECT_COLUMNS: &str = "SELECT id, business_date, net_revenue, total_orders, refund_amount, refund_count, auto_generated, generated_at, generated_by_id, generated_by_name, note FROM daily_report";

pub async fn find_by_id(pool: &SqlitePool, id: i64) -> RepoResult<Option<DailyReport>> {
    let sql = format!("{SELECT_COLUMNS} WHERE id = ?");
    let mut report = sqlx::query_as::<_, DailyReport>(&sql)
        .bind(id)
        .fetch_optional(pool)
        .await?;

    if let Some(ref mut r) = report {
        r.shift_breakdowns = find_shift_breakdowns(pool, r.id).await?;
    }
    Ok(report)
}

pub async fn find_by_date(pool: &SqlitePool, date: &str) -> RepoResult<Option<DailyReport>> {
    let sql = format!("{SELECT_COLUMNS} WHERE business_date = ? LIMIT 1");
    let mut report = sqlx::query_as::<_, DailyReport>(&sql)
        .bind(date)
        .fetch_optional(pool)
        .await?;

    if let Some(ref mut r) = report {
        r.shift_breakdowns = find_shift_breakdowns(pool, r.id).await?;
    }
    Ok(report)
}

pub async fn find_all(pool: &SqlitePool, limit: i32, offset: i32) -> RepoResult<Vec<DailyReport>> {
    let sql = format!("{SELECT_COLUMNS} ORDER BY business_date DESC LIMIT ? OFFSET ?");
    let mut reports = sqlx::query_as::<_, DailyReport>(&sql)
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
    let sql = format!(
        "{SELECT_COLUMNS} WHERE business_date >= ? AND business_date <= ? ORDER BY business_date DESC"
    );
    let mut reports = sqlx::query_as::<_, DailyReport>(&sql)
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
    auto_generated: bool,
) -> RepoResult<DailyReport> {
    let now = shared::util::now_millis();

    // 1. Count completed orders + sum total_amount
    let (total_orders, total_sales): (i64, f64) = sqlx::query_as(
        "SELECT COUNT(*), COALESCE(SUM(total_amount), 0.0) FROM archived_order WHERE end_time >= ? AND end_time < ? AND status = 'COMPLETED' AND is_voided = 0",
    )
    .bind(start_millis)
    .bind(end_millis)
    .fetch_one(pool)
    .await?;

    // 2. Count + sum credit_note refunds
    let (refund_count, refund_amount): (i64, f64) = sqlx::query_as(
        "SELECT COUNT(*), COALESCE(SUM(total_credit), 0.0) FROM credit_note WHERE created_at >= ? AND created_at < ?",
    )
    .bind(start_millis)
    .bind(end_millis)
    .fetch_one(pool)
    .await?;

    // 3. net_revenue = total_sales - refund_amount
    let net_revenue = total_sales - refund_amount;

    // Create report + shift breakdowns in a single transaction
    let mut tx = pool.begin().await?;

    let report_id = shared::util::snowflake_id();
    sqlx::query(
        "INSERT INTO daily_report (id, business_date, net_revenue, total_orders, refund_amount, refund_count, auto_generated, generated_at, generated_by_id, generated_by_name, note) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
    )
    .bind(report_id)
    .bind(&data.business_date)
    .bind(net_revenue)
    .bind(total_orders)
    .bind(refund_amount)
    .bind(refund_count)
    .bind(auto_generated)
    .bind(now)
    .bind(operator_id)
    .bind(&operator_name)
    .bind(&data.note)
    .execute(&mut *tx)
    .await?;

    // Shift breakdown: aggregate archived_order stats by shift_id, join shift table for metadata
    let shift_rows: Vec<ShiftAggRow> = sqlx::query_as(
        "SELECT ao.shift_id, \
         COUNT(*) as total_orders, \
         COUNT(CASE WHEN ao.status = 'COMPLETED' AND ao.is_voided = 0 THEN 1 END), \
         COUNT(CASE WHEN ao.status = 'VOID' THEN 1 END), \
         COALESCE(SUM(CASE WHEN ao.status = 'COMPLETED' AND ao.is_voided = 0 THEN ao.total_amount ELSE 0.0 END), 0.0), \
         COALESCE(SUM(CASE WHEN ao.status = 'COMPLETED' AND ao.is_voided = 0 THEN ao.paid_amount ELSE 0.0 END), 0.0), \
         COALESCE(SUM(CASE WHEN ao.status = 'VOID' THEN ao.total_amount ELSE 0.0 END), 0.0), \
         COALESCE(SUM(CASE WHEN ao.status = 'COMPLETED' AND ao.is_voided = 0 THEN ao.tax ELSE 0.0 END), 0.0), \
         COALESCE(SUM(CASE WHEN ao.status = 'COMPLETED' AND ao.is_voided = 0 THEN ao.discount_amount ELSE 0.0 END), 0.0), \
         COALESCE(SUM(CASE WHEN ao.status = 'COMPLETED' AND ao.is_voided = 0 THEN ao.surcharge_amount ELSE 0.0 END), 0.0) \
         FROM archived_order ao \
         WHERE ao.end_time >= ? AND ao.end_time < ? \
         GROUP BY ao.shift_id",
    )
    .bind(start_millis)
    .bind(end_millis)
    .fetch_all(&mut *tx)
    .await?;

    for (shift_id_opt, total, completed, voided, sales, paid, void_amt, tax, discount, surcharge) in
        &shift_rows
    {
        let sb_id = shared::util::snowflake_id();

        if let Some(sid) = shift_id_opt {
            // 有关联班次 — 从 shift 表获取元信息
            let shift_meta: Option<ShiftMetaRow> = sqlx::query_as(
                "SELECT operator_id, operator_name, status, start_time, end_time, starting_cash, expected_cash, actual_cash, cash_variance, abnormal_close FROM shift WHERE id = ?"
            )
            .bind(sid)
            .fetch_optional(&mut *tx)
            .await?;

            if let Some((
                op_id,
                op_name,
                status,
                start,
                end,
                starting,
                expected,
                actual,
                variance,
                abnormal,
            )) = shift_meta
            {
                sqlx::query(
                    "INSERT INTO daily_report_shift_breakdown (id, report_id, shift_id, operator_id, operator_name, status, start_time, end_time, starting_cash, expected_cash, actual_cash, cash_variance, abnormal_close, total_orders, completed_orders, void_orders, total_sales, total_paid, void_amount, total_tax, total_discount, total_surcharge) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22)"
                )
                .bind(sb_id).bind(report_id).bind(sid)
                .bind(op_id).bind(&op_name).bind(&status)
                .bind(start).bind(end)
                .bind(starting).bind(expected).bind(actual).bind(variance)
                .bind(abnormal)
                .bind(total).bind(completed).bind(voided)
                .bind(sales).bind(paid).bind(void_amt)
                .bind(tax).bind(discount).bind(surcharge)
                .execute(&mut *tx)
                .await?;
            }
        } else {
            // 未关联班次 — 归档重试场景下没有开放班次
            sqlx::query(
                "INSERT INTO daily_report_shift_breakdown (id, report_id, shift_id, operator_id, operator_name, status, start_time, end_time, starting_cash, expected_cash, actual_cash, cash_variance, abnormal_close, total_orders, completed_orders, void_orders, total_sales, total_paid, void_amount, total_tax, total_discount, total_surcharge) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22)"
            )
            .bind(sb_id).bind(report_id).bind(0i64)
            .bind(0i64).bind("UNLINKED").bind("CLOSED")
            .bind(start_millis).bind(end_millis)
            .bind(0.0f64).bind(0.0f64).bind(None::<f64>).bind(None::<f64>)
            .bind(false)
            .bind(total).bind(completed).bind(voided)
            .bind(sales).bind(paid).bind(void_amt)
            .bind(tax).bind(discount).bind(surcharge)
            .execute(&mut *tx)
            .await?;
        }
    }

    tx.commit().await?;

    find_by_id(pool, report_id)
        .await?
        .ok_or_else(|| RepoError::Database("Failed to generate daily report".into()))
}

// ── Breakdowns ──────────────────────────────────────────────────────────

async fn find_shift_breakdowns(
    pool: &SqlitePool,
    report_id: i64,
) -> RepoResult<Vec<ShiftBreakdown>> {
    let breakdowns = sqlx::query_as::<_, ShiftBreakdown>(
        "SELECT id, report_id, shift_id, operator_id, operator_name, status, start_time, end_time, starting_cash, expected_cash, actual_cash, cash_variance, abnormal_close, total_orders, completed_orders, void_orders, total_sales, total_paid, void_amount, total_tax, total_discount, total_surcharge FROM daily_report_shift_breakdown WHERE report_id = ? ORDER BY start_time ASC",
    )
    .bind(report_id)
    .fetch_all(pool)
    .await?;
    Ok(breakdowns)
}

/// Batch load shift breakdowns for multiple reports (eliminates N+1)
async fn batch_load_breakdowns(pool: &SqlitePool, reports: &mut [DailyReport]) -> RepoResult<()> {
    if reports.is_empty() {
        return Ok(());
    }
    let ids: Vec<i64> = reports.iter().map(|r| r.id).collect();
    let placeholders = ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");

    // Shift breakdowns
    let shift_sql = format!(
        "SELECT id, report_id, shift_id, operator_id, operator_name, status, start_time, end_time, starting_cash, expected_cash, actual_cash, cash_variance, abnormal_close, total_orders, completed_orders, void_orders, total_sales, total_paid, void_amount, total_tax, total_discount, total_surcharge FROM daily_report_shift_breakdown WHERE report_id IN ({placeholders}) ORDER BY start_time ASC"
    );
    let mut shift_query = sqlx::query_as::<_, ShiftBreakdown>(&shift_sql);
    for id in &ids {
        shift_query = shift_query.bind(id);
    }
    let all_shift = shift_query.fetch_all(pool).await?;

    let mut shift_map: std::collections::HashMap<i64, Vec<ShiftBreakdown>> =
        std::collections::HashMap::new();
    for s in all_shift {
        shift_map.entry(s.report_id).or_default().push(s);
    }

    for r in reports.iter_mut() {
        r.shift_breakdowns = shift_map.remove(&r.id).unwrap_or_default();
    }
    Ok(())
}

/// Delete daily reports older than retention_days (and their shift breakdowns via CASCADE)
pub async fn cleanup_old_reports(pool: &SqlitePool, retention_days: i32) -> RepoResult<u64> {
    let cutoff_date = chrono::Utc::now()
        .checked_sub_signed(chrono::Duration::days(retention_days as i64))
        .unwrap_or(chrono::Utc::now())
        .format("%Y-%m-%d")
        .to_string();

    let result = sqlx::query("DELETE FROM daily_report WHERE business_date < ?")
        .bind(&cutoff_date)
        .execute(pool)
        .await?;

    Ok(result.rows_affected())
}
