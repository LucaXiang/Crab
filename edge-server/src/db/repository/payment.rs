//! Payment Repository
//!
//! 独立 payment 表 CRUD，归档时从 OrderSnapshot 写入。
//! payment_id UNIQUE 索引保证幂等。

use super::RepoResult;
use shared::order::{OrderSnapshot, SplitType};
use sqlx::SqlitePool;

/// Payment record (query result)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
pub struct PaymentRow {
    pub id: i64,
    pub payment_id: String,
    pub order_id: String,
    pub method: String,
    pub amount: f64,
    pub tendered: Option<f64>,
    pub change_amount: Option<f64>,
    pub note: Option<String>,
    pub split_type: Option<String>,
    pub aa_shares: Option<i32>,
    pub split_items: Option<String>,
    pub operator_id: Option<i64>,
    pub operator_name: Option<String>,
    pub cancelled: bool,
    pub cancel_reason: Option<String>,
    pub timestamp: i64,
    pub created_at: i64,
}

/// Batch create payment records from OrderSnapshot (idempotent via UNIQUE index)
pub async fn create_from_snapshot(
    pool: &SqlitePool,
    snapshot: &OrderSnapshot,
    operator_id: Option<i64>,
    operator_name: Option<&str>,
) -> RepoResult<usize> {
    let now = shared::util::now_millis();
    let mut count = 0;

    for payment in &snapshot.payments {
        let split_type_str = payment.split_type.as_ref().map(|st| match st {
            SplitType::ItemSplit => "ItemSplit",
            SplitType::AmountSplit => "AmountSplit",
            SplitType::AaSplit => "AaSplit",
        });

        let split_items_json: Option<String> = payment.split_items.as_ref().map(|items| {
            let simple: Vec<serde_json::Value> = items
                .iter()
                .map(|si| {
                    serde_json::json!({
                        "name": si.name,
                        "quantity": si.quantity,
                        "unit_price": if si.unit_price > 0.0 { si.unit_price } else { si.price },
                    })
                })
                .collect();
            serde_json::to_string(&simple).unwrap_or_else(|_| "[]".to_string())
        });

        let result = sqlx::query!(
            "INSERT INTO payment (payment_id, order_id, method, amount, tendered, change_amount, note, split_type, aa_shares, split_items, operator_id, operator_name, cancelled, cancel_reason, timestamp, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
            payment.payment_id,
            snapshot.order_id,
            payment.method,
            payment.amount,
            payment.tendered,
            payment.change,
            payment.note,
            split_type_str,
            payment.aa_shares,
            split_items_json,
            operator_id,
            operator_name,
            payment.cancelled,
            payment.cancel_reason,
            payment.timestamp,
            now
        )
        .execute(pool)
        .await;

        match result {
            Ok(_) => count += 1,
            Err(e) => {
                let msg = e.to_string().to_lowercase();
                if msg.contains("unique") || msg.contains("duplicate") {
                    tracing::debug!(
                        payment_id = %payment.payment_id,
                        "Payment already exists, skipping"
                    );
                } else {
                    tracing::warn!(
                        payment_id = %payment.payment_id,
                        error = %e,
                        "Failed to create payment record"
                    );
                }
            }
        }
    }

    Ok(count)
}

/// List payments by order
pub async fn list_by_order(pool: &SqlitePool, order_id: &str) -> RepoResult<Vec<PaymentRow>> {
    let rows = sqlx::query_as::<_, PaymentRow>(
        "SELECT id, payment_id, order_id, method, amount, tendered, change_amount, note, split_type, aa_shares, split_items, operator_id, operator_name, cancelled, cancel_reason, timestamp, created_at FROM payment WHERE order_id = ? ORDER BY timestamp ASC",
    )
    .bind(order_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// List payments by time range (for statistics)
pub async fn list_by_time_range(
    pool: &SqlitePool,
    from: i64,
    to: i64,
) -> RepoResult<Vec<PaymentRow>> {
    let rows = sqlx::query_as::<_, PaymentRow>(
        "SELECT id, payment_id, order_id, method, amount, tendered, change_amount, note, split_type, aa_shares, split_items, operator_id, operator_name, cancelled, cancel_reason, timestamp, created_at FROM payment WHERE timestamp >= ? AND timestamp <= ? ORDER BY timestamp ASC",
    )
    .bind(from)
    .bind(to)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}
