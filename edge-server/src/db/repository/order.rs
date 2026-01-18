//! Order Repository (Graph/Document hybrid)

use super::{make_thing, BaseRepository, RepoError, RepoResult};
use crate::db::models::{
    Order, OrderCreate, OrderStatus, OrderItem, OrderPayment,
    OrderEvent, OrderEventType, OrderAddItem, OrderAddPayment,
};
use surrealdb::engine::local::Db;
use surrealdb::Surreal;

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

    /// Find open orders
    pub async fn find_open(&self) -> RepoResult<Vec<Order>> {
        self.find_by_status(OrderStatus::Open).await
    }

    /// Find order by id
    pub async fn find_by_id(&self, id: &str) -> RepoResult<Option<Order>> {
        let order: Option<Order> = self.base.db().select((TABLE, id)).await?;
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

    /// Create a new order
    pub async fn create(&self, data: OrderCreate, curr_hash: String) -> RepoResult<Order> {
        // Check duplicate receipt number
        if self.find_by_receipt(&data.receipt_number).await?.is_some() {
            return Err(RepoError::Duplicate(format!(
                "Order with receipt '{}' already exists",
                data.receipt_number
            )));
        }

        let order = Order {
            id: None,
            receipt_number: data.receipt_number,
            zone_name: data.zone_name,
            table_name: data.table_name,
            status: OrderStatus::Open,
            start_time: chrono::Utc::now().to_rfc3339(),
            end_time: None,
            guest_count: data.guest_count,
            total_amount: 0,
            paid_amount: 0,
            discount_amount: 0,
            surcharge_amount: 0,
            items: vec![],
            payments: vec![],
            prev_hash: data.prev_hash,
            curr_hash,
            created_at: None,
        };

        let created: Option<Order> = self.base.db().create(TABLE).content(order).await?;
        created.ok_or_else(|| RepoError::Database("Failed to create order".to_string()))
    }

    /// Add item to order
    pub async fn add_item(&self, order_id: &str, item: OrderAddItem) -> RepoResult<Order> {
        let order_thing = make_thing(TABLE, order_id);
        let new_item = OrderItem {
            spec: item.spec,
            name: item.name,
            spec_name: item.spec_name,
            price: item.price,
            quantity: item.quantity,
            attributes: item.attributes.unwrap_or_default(),
            discount_amount: 0,
            surcharge_amount: 0,
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
        let order_thing = make_thing(TABLE, order_id);
        let mut result = self
            .base
            .db()
            .query("UPDATE order SET items = array::remove(items, $idx) WHERE id = $id RETURN AFTER")
            .bind(("id", order_thing))
            .bind(("idx", item_idx))
            .await?;
        let orders: Vec<Order> = result.take(0)?;
        orders
            .into_iter()
            .next()
            .ok_or_else(|| RepoError::NotFound(format!("Order {} not found", order_id)))
    }

    /// Add payment to order
    pub async fn add_payment(&self, order_id: &str, payment: OrderAddPayment) -> RepoResult<Order> {
        let order_thing = make_thing(TABLE, order_id);
        let new_payment = OrderPayment {
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
        let order_thing = make_thing(TABLE, order_id);
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

    /// Update order status
    pub async fn update_status(&self, order_id: &str, status: OrderStatus) -> RepoResult<Order> {
        let order_thing = make_thing(TABLE, order_id);
        let end_time = if status != OrderStatus::Open {
            Some(chrono::Utc::now().to_rfc3339())
        } else {
            None
        };

        let mut result = self
            .base
            .db()
            .query("UPDATE order SET status = $status, end_time = $end WHERE id = $id RETURN AFTER")
            .bind(("id", order_thing))
            .bind(("status", status))
            .bind(("end", end_time))
            .await?;
        let orders: Vec<Order> = result.take(0)?;
        orders
            .into_iter()
            .next()
            .ok_or_else(|| RepoError::NotFound(format!("Order {} not found", order_id)))
    }

    /// Update order hash
    pub async fn update_hash(&self, order_id: &str, prev_hash: String, curr_hash: String) -> RepoResult<Order> {
        let order_thing = make_thing(TABLE, order_id);
        let mut result = self
            .base
            .db()
            .query("UPDATE order SET prev_hash = $prev, curr_hash = $curr WHERE id = $id RETURN AFTER")
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
        let order_thing = make_thing(TABLE, order_id);

        // Create event
        let event = OrderEvent {
            id: None,
            event_type,
            timestamp: chrono::Utc::now().to_rfc3339(),
            data,
            prev_hash,
            curr_hash,
        };

        let created: Option<OrderEvent> = self.base.db().create(EVENT_TABLE).content(event).await?;
        let event = created.ok_or_else(|| RepoError::Database("Failed to create event".to_string()))?;

        // Create edge relation
        let event_thing = event.id.clone().ok_or_else(|| RepoError::Database("Event has no ID".to_string()))?;
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
        let order_thing = make_thing(TABLE, order_id);
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
