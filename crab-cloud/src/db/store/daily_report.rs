//! Daily report database operations (edge → cloud sync only)

use shared::models::daily_report::DailyReport;
use sqlx::PgPool;

use super::BoxError;

// ── Edge Sync ──

pub async fn upsert_daily_report_from_sync(
    pool: &PgPool,
    store_id: i64,
    source_id: i64,
    data: &serde_json::Value,
    now: i64,
) -> Result<(), BoxError> {
    let report: DailyReport = serde_json::from_value(data.clone())?;
    let mut tx = pool.begin().await?;

    let (pg_id,): (i64,) = sqlx::query_as(
        r#"
        INSERT INTO store_daily_reports (
            store_id, source_id, business_date,
            total_orders, completed_orders, void_orders,
            total_sales, total_paid, total_unpaid, void_amount,
            total_tax, total_discount, total_surcharge,
            generated_at, generated_by_id, generated_by_name, note, updated_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18)
        ON CONFLICT (store_id, source_id)
        DO UPDATE SET
            business_date = EXCLUDED.business_date,
            total_orders = EXCLUDED.total_orders, completed_orders = EXCLUDED.completed_orders,
            void_orders = EXCLUDED.void_orders,
            total_sales = EXCLUDED.total_sales, total_paid = EXCLUDED.total_paid,
            total_unpaid = EXCLUDED.total_unpaid, void_amount = EXCLUDED.void_amount,
            total_tax = EXCLUDED.total_tax, total_discount = EXCLUDED.total_discount,
            total_surcharge = EXCLUDED.total_surcharge,
            generated_at = EXCLUDED.generated_at, generated_by_id = EXCLUDED.generated_by_id,
            generated_by_name = EXCLUDED.generated_by_name, note = EXCLUDED.note,
            updated_at = EXCLUDED.updated_at
        RETURNING id
        "#,
    )
    .bind(store_id)
    .bind(source_id)
    .bind(&report.business_date)
    .bind(report.total_orders)
    .bind(report.completed_orders)
    .bind(report.void_orders)
    .bind(report.total_sales)
    .bind(report.total_paid)
    .bind(report.total_unpaid)
    .bind(report.void_amount)
    .bind(report.total_tax)
    .bind(report.total_discount)
    .bind(report.total_surcharge)
    .bind(report.generated_at)
    .bind(report.generated_by_id)
    .bind(&report.generated_by_name)
    .bind(&report.note)
    .bind(now)
    .fetch_one(&mut *tx)
    .await?;

    // Replace tax breakdowns
    sqlx::query("DELETE FROM store_daily_report_tax_breakdown WHERE report_id = $1")
        .bind(pg_id)
        .execute(&mut *tx)
        .await?;

    if !report.tax_breakdowns.is_empty() {
        let rids: Vec<i64> = report.tax_breakdowns.iter().map(|_| pg_id).collect();
        let rates: Vec<i32> = report.tax_breakdowns.iter().map(|t| t.tax_rate).collect();
        let nets: Vec<f64> = report.tax_breakdowns.iter().map(|t| t.net_amount).collect();
        let taxes: Vec<f64> = report.tax_breakdowns.iter().map(|t| t.tax_amount).collect();
        let grosses: Vec<f64> = report
            .tax_breakdowns
            .iter()
            .map(|t| t.gross_amount)
            .collect();
        let counts: Vec<i64> = report
            .tax_breakdowns
            .iter()
            .map(|t| t.order_count)
            .collect();
        sqlx::query(
            r#"INSERT INTO store_daily_report_tax_breakdown (
                report_id, tax_rate, net_amount, tax_amount, gross_amount, order_count
            ) SELECT * FROM UNNEST(
                $1::bigint[], $2::integer[], $3::double precision[],
                $4::double precision[], $5::double precision[], $6::bigint[]
            )"#,
        )
        .bind(&rids)
        .bind(&rates)
        .bind(&nets)
        .bind(&taxes)
        .bind(&grosses)
        .bind(&counts)
        .execute(&mut *tx)
        .await?;
    }

    // Replace payment breakdowns
    sqlx::query("DELETE FROM store_daily_report_payment_breakdown WHERE report_id = $1")
        .bind(pg_id)
        .execute(&mut *tx)
        .await?;

    if !report.payment_breakdowns.is_empty() {
        let rids: Vec<i64> = report.payment_breakdowns.iter().map(|_| pg_id).collect();
        let methods: Vec<String> = report
            .payment_breakdowns
            .iter()
            .map(|p| p.method.clone())
            .collect();
        let amounts: Vec<f64> = report.payment_breakdowns.iter().map(|p| p.amount).collect();
        let counts: Vec<i64> = report.payment_breakdowns.iter().map(|p| p.count).collect();
        sqlx::query(
            r#"INSERT INTO store_daily_report_payment_breakdown (
                report_id, method, amount, count
            ) SELECT * FROM UNNEST(
                $1::bigint[], $2::text[], $3::double precision[], $4::bigint[]
            )"#,
        )
        .bind(&rids)
        .bind(&methods)
        .bind(&amounts)
        .bind(&counts)
        .execute(&mut *tx)
        .await?;
    }

    // Replace shift breakdowns
    sqlx::query("DELETE FROM store_daily_report_shift_breakdown WHERE report_id = $1")
        .bind(pg_id)
        .execute(&mut *tx)
        .await?;

    if !report.shift_breakdowns.is_empty() {
        let rids: Vec<i64> = report.shift_breakdowns.iter().map(|_| pg_id).collect();
        let shift_ids: Vec<i64> = report.shift_breakdowns.iter().map(|s| s.shift_id).collect();
        let operator_ids: Vec<i64> = report
            .shift_breakdowns
            .iter()
            .map(|s| s.operator_id)
            .collect();
        let operator_names: Vec<String> = report
            .shift_breakdowns
            .iter()
            .map(|s| s.operator_name.clone())
            .collect();
        let statuses: Vec<String> = report
            .shift_breakdowns
            .iter()
            .map(|s| s.status.clone())
            .collect();
        let start_times: Vec<i64> = report
            .shift_breakdowns
            .iter()
            .map(|s| s.start_time)
            .collect();
        let end_times: Vec<Option<i64>> =
            report.shift_breakdowns.iter().map(|s| s.end_time).collect();
        let starting_cashes: Vec<f64> = report
            .shift_breakdowns
            .iter()
            .map(|s| s.starting_cash)
            .collect();
        let expected_cashes: Vec<f64> = report
            .shift_breakdowns
            .iter()
            .map(|s| s.expected_cash)
            .collect();
        let actual_cashes: Vec<Option<f64>> = report
            .shift_breakdowns
            .iter()
            .map(|s| s.actual_cash)
            .collect();
        let cash_variances: Vec<Option<f64>> = report
            .shift_breakdowns
            .iter()
            .map(|s| s.cash_variance)
            .collect();
        let abnormal_closes: Vec<bool> = report
            .shift_breakdowns
            .iter()
            .map(|s| s.abnormal_close)
            .collect();
        let total_orders: Vec<i64> = report
            .shift_breakdowns
            .iter()
            .map(|s| s.total_orders)
            .collect();
        let completed_orders: Vec<i64> = report
            .shift_breakdowns
            .iter()
            .map(|s| s.completed_orders)
            .collect();
        let void_orders: Vec<i64> = report
            .shift_breakdowns
            .iter()
            .map(|s| s.void_orders)
            .collect();
        let total_sales: Vec<f64> = report
            .shift_breakdowns
            .iter()
            .map(|s| s.total_sales)
            .collect();
        let total_paid: Vec<f64> = report
            .shift_breakdowns
            .iter()
            .map(|s| s.total_paid)
            .collect();
        let void_amounts: Vec<f64> = report
            .shift_breakdowns
            .iter()
            .map(|s| s.void_amount)
            .collect();
        let total_tax: Vec<f64> = report
            .shift_breakdowns
            .iter()
            .map(|s| s.total_tax)
            .collect();
        let total_discount: Vec<f64> = report
            .shift_breakdowns
            .iter()
            .map(|s| s.total_discount)
            .collect();
        let total_surcharge: Vec<f64> = report
            .shift_breakdowns
            .iter()
            .map(|s| s.total_surcharge)
            .collect();
        sqlx::query(
            r#"INSERT INTO store_daily_report_shift_breakdown (
                report_id, shift_source_id, operator_id, operator_name, status,
                start_time, end_time, starting_cash, expected_cash,
                actual_cash, cash_variance, abnormal_close,
                total_orders, completed_orders, void_orders,
                total_sales, total_paid, void_amount,
                total_tax, total_discount, total_surcharge
            ) SELECT * FROM UNNEST(
                $1::bigint[], $2::bigint[], $3::bigint[], $4::text[], $5::text[],
                $6::bigint[], $7::bigint[], $8::double precision[], $9::double precision[],
                $10::double precision[], $11::double precision[], $12::boolean[],
                $13::bigint[], $14::bigint[], $15::bigint[],
                $16::double precision[], $17::double precision[], $18::double precision[],
                $19::double precision[], $20::double precision[], $21::double precision[]
            )"#,
        )
        .bind(&rids)
        .bind(&shift_ids)
        .bind(&operator_ids)
        .bind(&operator_names)
        .bind(&statuses)
        .bind(&start_times)
        .bind(&end_times)
        .bind(&starting_cashes)
        .bind(&expected_cashes)
        .bind(&actual_cashes)
        .bind(&cash_variances)
        .bind(&abnormal_closes)
        .bind(&total_orders)
        .bind(&completed_orders)
        .bind(&void_orders)
        .bind(&total_sales)
        .bind(&total_paid)
        .bind(&void_amounts)
        .bind(&total_tax)
        .bind(&total_discount)
        .bind(&total_surcharge)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    Ok(())
}
