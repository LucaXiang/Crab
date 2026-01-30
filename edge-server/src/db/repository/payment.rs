//! Payment Repository
//!
//! 独立 payment 表 CRUD，归档时从 OrderSnapshot 写入。
//! payment_id UNIQUE 索引保证幂等。

use super::{BaseRepository, RepoResult};
use serde::{Deserialize, Serialize};
use shared::order::{OrderSnapshot, SplitType};
use surrealdb::engine::local::Db;
use surrealdb::Surreal;

/// SurrealDB payment 记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentRow {
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
    pub operator_id: Option<String>,
    pub operator_name: Option<String>,
    pub cancelled: bool,
    pub cancel_reason: Option<String>,
    pub timestamp: i64,
    pub created_at: i64,
}

#[derive(Clone)]
pub struct PaymentRepository {
    base: BaseRepository,
}

impl PaymentRepository {
    pub fn new(db: Surreal<Db>) -> Self {
        Self {
            base: BaseRepository::new(db),
        }
    }

    /// 从 OrderSnapshot 批量写入 payment 记录（归档时调用）
    ///
    /// - 幂等：payment_id UNIQUE 索引，重复写入会被忽略
    /// - 包含已取消的支付记录（cancelled=true）
    pub async fn create_from_snapshot(
        &self,
        snapshot: &OrderSnapshot,
        operator_id: Option<&str>,
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

            // Serialize split_items to JSON string
            let split_items_json: Option<String> = payment.split_items.as_ref().map(|items| {
                let simple: Vec<serde_json::Value> = items
                    .iter()
                    .map(|si| {
                        serde_json::json!({
                            "name": si.name,
                            "quantity": si.quantity,
                            "unit_price": si.unit_price.unwrap_or(si.price),
                        })
                    })
                    .collect();
                serde_json::to_string(&simple).unwrap_or_else(|_| "[]".to_string())
            });

            let result: Result<Option<PaymentRow>, _> = self
                .base
                .db()
                .query(
                    r#"
                    CREATE payment SET
                        payment_id    = $payment_id,
                        order_id      = $order_id,
                        method        = $method,
                        amount        = $amount,
                        tendered      = $tendered,
                        change_amount = $change_amount,
                        note          = $note,
                        split_type    = $split_type,
                        aa_shares     = $aa_shares,
                        split_items   = $split_items,
                        operator_id   = $operator_id,
                        operator_name = $operator_name,
                        cancelled     = $cancelled,
                        cancel_reason = $cancel_reason,
                        timestamp     = $timestamp,
                        created_at    = $created_at
                    "#,
                )
                .bind(("payment_id", payment.payment_id.clone()))
                .bind(("order_id", snapshot.order_id.clone()))
                .bind(("method", payment.method.clone()))
                .bind(("amount", payment.amount))
                .bind(("tendered", payment.tendered))
                .bind(("change_amount", payment.change))
                .bind(("note", payment.note.clone()))
                .bind(("split_type", split_type_str.map(|s| s.to_string())))
                .bind(("aa_shares", payment.aa_shares))
                .bind(("split_items", split_items_json))
                .bind(("operator_id", operator_id.map(|s| s.to_string())))
                .bind(("operator_name", operator_name.map(|s| s.to_string())))
                .bind(("cancelled", payment.cancelled))
                .bind(("cancel_reason", payment.cancel_reason.clone()))
                .bind(("timestamp", payment.timestamp))
                .bind(("created_at", now))
                .await
                .map(|mut r| r.take(0).ok().flatten());

            match result {
                Ok(_) => count += 1,
                Err(e) => {
                    let msg = e.to_string().to_lowercase();
                    if msg.contains("unique") || msg.contains("already exists") || msg.contains("duplicate") {
                        // 幂等：重复写入跳过
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

    /// 按订单查询支付记录
    pub async fn list_by_order(&self, order_id: &str) -> RepoResult<Vec<PaymentRow>> {
        let result: Vec<PaymentRow> = self
            .base
            .db()
            .query("SELECT * FROM payment WHERE order_id = $order_id ORDER BY timestamp ASC")
            .bind(("order_id", order_id.to_string()))
            .await
            .map_err(super::RepoError::from)?
            .take(0)
            .map_err(super::RepoError::from)?;
        Ok(result)
    }

    /// 按时间范围查询（统计用）
    pub async fn list_by_time_range(
        &self,
        from: i64,
        to: i64,
    ) -> RepoResult<Vec<PaymentRow>> {
        let result: Vec<PaymentRow> = self
            .base
            .db()
            .query(
                "SELECT * FROM payment WHERE timestamp >= $from AND timestamp <= $to ORDER BY timestamp ASC",
            )
            .bind(("from", from))
            .bind(("to", to))
            .await
            .map_err(super::RepoError::from)?
            .take(0)
            .map_err(super::RepoError::from)?;
        Ok(result)
    }
}
