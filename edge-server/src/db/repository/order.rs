//! Order Repository (Graph Model)
//!
//! Read-only access to archived orders in SurrealDB.
//! All order mutations go through OrderManager event sourcing.

use super::{BaseRepository, RepoError, RepoResult};
use crate::db::models::OrderDetail;
use surrealdb::engine::local::Db;
use surrealdb::{RecordId, Surreal};

#[derive(Clone)]
pub struct OrderRepository {
    base: BaseRepository,
}

impl OrderRepository {
    pub fn new(db: Surreal<Db>) -> Self {
        Self {
            base: BaseRepository::new(db),
        }
    }

    /// Get full order detail using graph traversal
    pub async fn get_order_detail(&self, order_id: &str) -> RepoResult<OrderDetail> {
        let record_id: RecordId = order_id.parse().map_err(|_| {
            RepoError::NotFound(format!("Invalid order ID format: {}", order_id))
        })?;

        // Query order with graph traversal for items, payments, events
        let mut result = self
            .base
            .db()
            .query(r#"
                SELECT
                    <string>id AS order_id,
                    receipt_number,
                    table_name,
                    zone_name,
                    string::uppercase(status) AS status,
                    is_retail,
                    guest_count,
                    total_amount AS total,
                    paid_amount,
                    discount_amount AS total_discount,
                    surcharge_amount AS total_surcharge,
                    comp_total_amount,
                    order_manual_discount_amount,
                    order_manual_surcharge_amount,
                    start_time,
                    end_time,
                    operator_name,
                    void_type,
                    loss_reason,
                    loss_amount,
                    void_note,
                    (
                        SELECT
                            <string>id AS id,
                            instance_id,
                            name,
                            spec_name,
                            price,
                            quantity,
                            unpaid_quantity,
                            unit_price,
                            line_total,
                            discount_amount,
                            surcharge_amount,
                            note,
                            (
                                SELECT
                                    attribute_name,
                                    option_name,
                                    price AS price_modifier
                                FROM ->has_option->order_item_option
                            ) AS selected_options
                        FROM ->has_item->order_item
                    ) AS items,
                    (
                        SELECT
                            seq,
                            payment_id,
                            method,
                            amount,
                            time AS timestamp,
                            cancelled,
                            cancel_reason,
                            split_type,
                            split_items,
                            aa_shares,
                            aa_total_shares
                        FROM ->has_payment->order_payment
                        ORDER BY seq, time
                    ) AS payments,
                    (
                        SELECT
                            seq,
                            <string>id AS event_id,
                            string::uppercase(event_type) AS event_type,
                            timestamp,
                            data AS payload
                        FROM ->has_event->order_event
                        ORDER BY seq, timestamp
                    ) AS timeline
                FROM order WHERE id = $id
            "#)
            .bind(("id", record_id))
            .await?;

        let details: Vec<OrderDetail> = result.take(0)?;
        details
            .into_iter()
            .next()
            .ok_or_else(|| RepoError::NotFound(format!("Order {} not found", order_id)))
    }
}
