//! Order Repository (Graph Model)

use super::{BaseRepository, RepoError, RepoResult};
use crate::db::models::{
    Order, OrderAddItem, OrderAddPayment, OrderDetail, OrderEvent, OrderEventType, OrderStatus,
};
use surrealdb::engine::local::Db;
use surrealdb::{RecordId, Surreal};

const TABLE: &str = "order";
const EVENT_TABLE: &str = "order_event";

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

    // =========================================================================
    // Order CRUD
    // =========================================================================

    /// Find all orders (paginated)
    pub async fn find_all(&self, limit: i32, offset: i32) -> RepoResult<Vec<Order>> {
        let mut result = self
            .base
            .db()
            .query("SELECT * FROM order ORDER BY created_at DESC LIMIT $limit START $offset")
            .bind(("limit", limit))
            .bind(("offset", offset))
            .await?;
        let orders: Vec<Order> = result.take(0)?;
        Ok(orders)
    }

    /// Find orders by status
    pub async fn find_by_status(&self, status: OrderStatus) -> RepoResult<Vec<Order>> {
        let orders: Vec<Order> = self
            .base
            .db()
            .query("SELECT * FROM order WHERE status = $status ORDER BY created_at DESC")
            .bind(("status", status))
            .await?
            .take(0)?;
        Ok(orders)
    }

    /// Find order by id (expects "order:abc123" format)
    pub async fn find_by_id(&self, id: &str) -> RepoResult<Option<Order>> {
        let record_id: RecordId = id.parse().map_err(|_| {
            RepoError::NotFound(format!("Invalid order ID format: {}", id))
        })?;
        let order: Option<Order> = self.base.db().select(record_id).await?;
        Ok(order)
    }

    /// Find order by receipt number
    pub async fn find_by_receipt(&self, receipt_number: &str) -> RepoResult<Option<Order>> {
        let receipt_owned = receipt_number.to_string();
        let mut result = self
            .base
            .db()
            .query("SELECT * FROM order WHERE receipt_number = $receipt LIMIT 1")
            .bind(("receipt", receipt_owned))
            .await?;
        let orders: Vec<Order> = result.take(0)?;
        Ok(orders.into_iter().next())
    }

    /// Check if order exists by receipt number (for idempotency check)
    pub async fn exists_by_receipt(&self, receipt_number: &str) -> RepoResult<bool> {
        let receipt_owned = receipt_number.to_string();
        let mut result = self
            .base
            .db()
            .query("SELECT count() FROM order WHERE receipt_number = $receipt GROUP ALL")
            .bind(("receipt", receipt_owned))
            .await?;
        let counts: Vec<serde_json::Value> = result.take(0)?;
        let exists = counts.first()
            .and_then(|v| v.get("count"))
            .and_then(|c| c.as_i64())
            .unwrap_or(0) > 0;
        Ok(exists)
    }

    /// Find order by hash
    pub async fn find_by_hash(&self, hash: &str) -> RepoResult<Option<Order>> {
        let hash_owned = hash.to_string();
        let mut result = self
            .base
            .db()
            .query("SELECT * FROM order WHERE curr_hash = $hash LIMIT 1")
            .bind(("hash", hash_owned))
            .await?;
        let orders: Vec<Order> = result.take(0)?;
        Ok(orders.into_iter().next())
    }

    /// Add item to order (for active orders - not used for archived)
    pub async fn add_item(&self, order_id: &str, item: OrderAddItem) -> RepoResult<Order> {
        let order_thing = RecordId::from_table_key(TABLE, order_id);
        let line_total = item.price * item.quantity as f64;
        let unit_price = item.price;

        #[derive(serde::Serialize)]
        struct InternalOrderItemAttribute {
            attr_id: String,
            option_idx: i32,
            name: String,
            price: f64,
        }

        #[derive(serde::Serialize)]
        struct InternalOrderItem {
            spec: String,
            name: String,
            spec_name: Option<String>,
            price: f64,
            quantity: i32,
            attributes: Vec<InternalOrderItemAttribute>,
            discount_amount: f64,
            surcharge_amount: f64,
            unit_price: f64,
            line_total: f64,
            note: Option<String>,
            is_sent: bool,
        }

        let attrs: Vec<InternalOrderItemAttribute> = item
            .attributes
            .unwrap_or_default()
            .into_iter()
            .map(|a| InternalOrderItemAttribute {
                attr_id: a.attr_id,
                option_idx: a.option_idx,
                name: a.name,
                price: a.price,
            })
            .collect();

        let new_item = InternalOrderItem {
            spec: item.spec,
            name: item.name,
            spec_name: item.spec_name,
            price: item.price,
            quantity: item.quantity,
            attributes: attrs,
            discount_amount: 0.0,
            surcharge_amount: 0.0,
            unit_price,
            line_total,
            note: item.note,
            is_sent: false,
        };

        let mut result = self
            .base
            .db()
            .query("UPDATE order SET items += $item WHERE id = $id RETURN AFTER")
            .bind(("id", order_thing))
            .bind(("item", new_item))
            .await?;
        let orders: Vec<Order> = result.take(0)?;
        orders
            .into_iter()
            .next()
            .ok_or_else(|| RepoError::NotFound(format!("Order {} not found", order_id)))
    }

    /// Remove item from order by index
    pub async fn remove_item(&self, order_id: &str, item_idx: usize) -> RepoResult<Order> {
        let order_thing = RecordId::from_table_key(TABLE, order_id);
        let mut result = self
            .base
            .db()
            .query(
                "UPDATE order SET items = array::remove(items, $idx) WHERE id = $id RETURN AFTER",
            )
            .bind(("id", order_thing))
            .bind(("idx", item_idx))
            .await?;
        let orders: Vec<Order> = result.take(0)?;
        orders
            .into_iter()
            .next()
            .ok_or_else(|| RepoError::NotFound(format!("Order {} not found", order_id)))
    }

    /// Add payment to order (for active orders - not used for archived)
    pub async fn add_payment(&self, order_id: &str, payment: OrderAddPayment) -> RepoResult<Order> {
        let order_thing = RecordId::from_table_key(TABLE, order_id);

        // For active orders, use a simplified payment struct
        #[derive(serde::Serialize)]
        struct SimplePayment {
            method: String,
            amount: f64,
            time: String,
            reference: Option<String>,
        }

        let new_payment = SimplePayment {
            method: payment.method,
            amount: payment.amount,
            time: chrono::Utc::now().to_rfc3339(),
            reference: payment.reference,
        };

        let mut result = self
            .base
            .db()
            .query(
                "UPDATE order SET payments += $pay, paid_amount += $amount WHERE id = $id RETURN AFTER",
            )
            .bind(("id", order_thing))
            .bind(("pay", new_payment))
            .bind(("amount", payment.amount))
            .await?;
        let orders: Vec<Order> = result.take(0)?;
        orders
            .into_iter()
            .next()
            .ok_or_else(|| RepoError::NotFound(format!("Order {} not found", order_id)))
    }

    /// Update order totals
    pub async fn update_totals(
        &self,
        order_id: &str,
        total_amount: i32,
        discount_amount: i32,
        surcharge_amount: i32,
    ) -> RepoResult<Order> {
        let order_thing = RecordId::from_table_key(TABLE, order_id);
        let mut result = self
            .base
            .db()
            .query(
                "UPDATE order SET total_amount = $total, discount_amount = $disc, surcharge_amount = $sur WHERE id = $id RETURN AFTER",
            )
            .bind(("id", order_thing))
            .bind(("total", total_amount))
            .bind(("disc", discount_amount))
            .bind(("sur", surcharge_amount))
            .await?;
        let orders: Vec<Order> = result.take(0)?;
        orders
            .into_iter()
            .next()
            .ok_or_else(|| RepoError::NotFound(format!("Order {} not found", order_id)))
    }

    /// Update order hash
    pub async fn update_hash(
        &self,
        order_id: &str,
        prev_hash: String,
        curr_hash: String,
    ) -> RepoResult<Order> {
        let order_thing = RecordId::from_table_key(TABLE, order_id);
        let mut result = self
            .base
            .db()
            .query(
                "UPDATE order SET prev_hash = $prev, curr_hash = $curr WHERE id = $id RETURN AFTER",
            )
            .bind(("id", order_thing))
            .bind(("prev", prev_hash))
            .bind(("curr", curr_hash))
            .await?;
        let orders: Vec<Order> = result.take(0)?;
        orders
            .into_iter()
            .next()
            .ok_or_else(|| RepoError::NotFound(format!("Order {} not found", order_id)))
    }

    // =========================================================================
    // Order Events (Graph)
    // =========================================================================

    /// Add event to order using RELATE
    pub async fn add_event(
        &self,
        order_id: &str,
        event_type: OrderEventType,
        data: Option<serde_json::Value>,
        prev_hash: String,
        curr_hash: String,
    ) -> RepoResult<OrderEvent> {
        let order_thing = RecordId::from_table_key(TABLE, order_id);

        let event = OrderEvent {
            id: None,
            event_type,
            timestamp: chrono::Utc::now().to_rfc3339(),
            data,
            prev_hash,
            curr_hash,
        };

        let created: Option<OrderEvent> = self.base.db().create(EVENT_TABLE).content(event).await?;
        let event =
            created.ok_or_else(|| RepoError::Database("Failed to create event".to_string()))?;

        let event_thing = event
            .id
            .clone()
            .ok_or_else(|| RepoError::Database("Event has no ID".to_string()))?;
        self.base
            .db()
            .query("RELATE $from->has_event->$to")
            .bind(("from", order_thing))
            .bind(("to", event_thing))
            .await?;

        Ok(event)
    }

    /// Get all events for an order (graph traversal)
    pub async fn get_events(&self, order_id: &str) -> RepoResult<Vec<OrderEvent>> {
        let order_thing = RecordId::from_table_key(TABLE, order_id);
        let mut result = self
            .base
            .db()
            .query("SELECT ->has_event->order_event.* FROM $order ORDER BY timestamp")
            .bind(("order", order_thing))
            .await?;
        let events: Vec<OrderEvent> = result.take(0)?;
        Ok(events)
    }

    // =========================================================================
    // Archiving
    // =========================================================================

    /// Create an archived order (graph model - no snapshot_json)
    pub async fn create_archived(&self, order: Order) -> RepoResult<Order> {
        tracing::debug!(
            receipt = %order.receipt_number,
            is_retail = order.is_retail,
            "Creating archived order (graph model)"
        );

        let mut result = self
            .base
            .db()
            .query(r#"
                CREATE order SET
                    receipt_number = $receipt_number,
                    zone_name = $zone_name,
                    table_name = $table_name,
                    status = $status,
                    is_retail = $is_retail,
                    guest_count = $guest_count,
                    original_total = $original_total,
                    subtotal = $subtotal,
                    total_amount = $total_amount,
                    paid_amount = $paid_amount,
                    discount_amount = $discount_amount,
                    surcharge_amount = $surcharge_amount,
                    tax = $tax,
                    start_time = <datetime>$start_time,
                    end_time = IF $end_time != NONE THEN <datetime>$end_time ELSE NONE END,
                    operator_id = $operator_id,
                    operator_name = $operator_name,
                    prev_hash = $prev_hash,
                    curr_hash = $curr_hash,
                    related_order_id = $related_order_id
                RETURN AFTER
            "#)
            .bind(("receipt_number", order.receipt_number))
            .bind(("zone_name", order.zone_name))
            .bind(("table_name", order.table_name))
            .bind(("status", format!("{:?}", order.status).to_uppercase()))
            .bind(("is_retail", order.is_retail))
            .bind(("guest_count", order.guest_count))
            .bind(("original_total", order.original_total))
            .bind(("subtotal", order.subtotal))
            .bind(("total_amount", order.total_amount))
            .bind(("paid_amount", order.paid_amount))
            .bind(("discount_amount", order.discount_amount))
            .bind(("surcharge_amount", order.surcharge_amount))
            .bind(("tax", order.tax))
            .bind(("start_time", order.start_time))
            .bind(("end_time", order.end_time))
            .bind(("operator_id", order.operator_id))
            .bind(("operator_name", order.operator_name))
            .bind(("prev_hash", order.prev_hash))
            .bind(("curr_hash", order.curr_hash))
            .bind(("related_order_id", order.related_order_id.map(|id| id.to_string())))
            .await?;

        let orders: Vec<Order> = result.take(0)?;
        orders
            .into_iter()
            .next()
            .ok_or_else(|| RepoError::Database("Failed to create archived order".to_string()))
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
                    time::millis(start_time) AS start_time,
                    time::millis(end_time) AS end_time,
                    operator_name,
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
                            method,
                            amount,
                            time::millis(time) AS timestamp,
                            reference AS note,
                            cancelled,
                            cancel_reason,
                            split_items
                        FROM ->has_payment->order_payment
                    ) AS payments,
                    (
                        SELECT
                            <string>id AS event_id,
                            string::uppercase(event_type) AS event_type,
                            time::millis(timestamp) AS timestamp,
                            data AS payload
                        FROM ->has_event->order_event
                        ORDER BY timestamp
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

    /// Find orders by date range (for history query)
    pub async fn find_by_date_range(
        &self,
        start_date: &str,
        end_date: &str,
        limit: i32,
        offset: i32,
        search: Option<&str>,
    ) -> RepoResult<Vec<Order>> {
        let start_datetime = format!("{}T00:00:00Z", start_date);
        let end_datetime = format!("{}T23:59:59Z", end_date);

        let query = if let Some(search_term) = search {
            let mut result = self
                .base
                .db()
                .query("SELECT * FROM order WHERE end_time >= <datetime>$start AND end_time <= <datetime>$end AND string::lowercase(receipt_number) CONTAINS $search ORDER BY end_time DESC LIMIT $limit START $offset")
                .bind(("start", start_datetime))
                .bind(("end", end_datetime))
                .bind(("search", search_term.to_lowercase()))
                .bind(("limit", limit))
                .bind(("offset", offset))
                .await?;
            result.take(0)?
        } else {
            let mut result = self
                .base
                .db()
                .query("SELECT * FROM order WHERE end_time >= <datetime>$start AND end_time <= <datetime>$end ORDER BY end_time DESC LIMIT $limit START $offset")
                .bind(("start", start_datetime))
                .bind(("end", end_datetime))
                .bind(("limit", limit))
                .bind(("offset", offset))
                .await?;
            result.take(0)?
        };
        Ok(query)
    }

    // =========================================================================
    // Hash Chain
    // =========================================================================

    /// Get last order (for hash chain)
    pub async fn get_last_order(&self) -> RepoResult<Option<Order>> {
        let mut result = self
            .base
            .db()
            .query("SELECT * FROM order ORDER BY created_at DESC LIMIT 1")
            .await?;
        let orders: Vec<Order> = result.take(0)?;
        Ok(orders.into_iter().next())
    }

    /// Verify hash chain integrity
    pub async fn verify_chain(&self, from_hash: Option<String>) -> RepoResult<bool> {
        let start_condition = match from_hash {
            Some(hash) => format!("curr_hash = '{}'", hash),
            None => "true".to_string(),
        };

        let query = format!(
            r#"
            LET $orders = (SELECT * FROM order WHERE {} ORDER BY created_at);
            LET $valid = true;
            FOR $i IN 1..array::len($orders) {{
                IF $orders[$i].prev_hash != $orders[$i - 1].curr_hash {{
                    $valid = false;
                    BREAK;
                }};
            }};
            RETURN $valid;
            "#,
            start_condition
        );

        let mut result = self.base.db().query(&query).await?;
        let valid: Option<bool> = result.take(0)?;
        Ok(valid.unwrap_or(false))
    }
}
