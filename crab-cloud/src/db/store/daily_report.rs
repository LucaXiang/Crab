//! Daily report database operations (edge → cloud sync only)

use shared::models::daily_report::DailyReport;
use sqlx::PgPool;

use super::BoxError;

// ── Edge Sync ──

pub async fn upsert_daily_report_from_sync(
    pool: &PgPool,
    edge_server_id: i64,
    source_id: i64,
    data: &serde_json::Value,
    now: i64,
) -> Result<(), BoxError> {
    let report: DailyReport = serde_json::from_value(data.clone())?;
    let mut tx = pool.begin().await?;

    let (pg_id,): (i64,) = sqlx::query_as(
        r#"
        INSERT INTO store_daily_reports (
            edge_server_id, source_id, business_date,
            total_orders, completed_orders, void_orders,
            total_sales, total_paid, total_unpaid, void_amount,
            total_tax, total_discount, total_surcharge,
            generated_at, generated_by_id, generated_by_name, note, updated_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18)
        ON CONFLICT (edge_server_id, source_id)
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
    .bind(edge_server_id)
    .bind(source_id)
    .bind(&report.business_date)
    .bind(report.total_orders as i32)
    .bind(report.completed_orders as i32)
    .bind(report.void_orders as i32)
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

    for tb in &report.tax_breakdowns {
        sqlx::query(
            r#"
            INSERT INTO store_daily_report_tax_breakdown (
                report_id, tax_rate, net_amount, tax_amount, gross_amount, order_count
            )
            VALUES ($1, $2, $3, $4, $5, $6)
            "#,
        )
        .bind(pg_id)
        .bind(tb.tax_rate)
        .bind(tb.net_amount)
        .bind(tb.tax_amount)
        .bind(tb.gross_amount)
        .bind(tb.order_count as i32)
        .execute(&mut *tx)
        .await?;
    }

    // Replace payment breakdowns
    sqlx::query("DELETE FROM store_daily_report_payment_breakdown WHERE report_id = $1")
        .bind(pg_id)
        .execute(&mut *tx)
        .await?;

    for pb in &report.payment_breakdowns {
        sqlx::query(
            r#"
            INSERT INTO store_daily_report_payment_breakdown (
                report_id, method, amount, count
            )
            VALUES ($1, $2, $3, $4)
            "#,
        )
        .bind(pg_id)
        .bind(&pb.method)
        .bind(pb.amount)
        .bind(pb.count as i32)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    Ok(())
}
