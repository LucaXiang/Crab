//! System State Repository (Singleton)

use super::{BaseRepository, RepoError, RepoResult};
use crate::db::models::{SystemState, SystemStateUpdate};
use surrealdb::engine::local::Db;
use surrealdb::{RecordId, Surreal};

const TABLE: &str = "system_state";
const SINGLETON_ID: &str = "main";

#[derive(Clone)]
pub struct SystemStateRepository {
    base: BaseRepository,
}

impl SystemStateRepository {
    pub fn new(db: Surreal<Db>) -> Self {
        Self {
            base: BaseRepository::new(db),
        }
    }

    /// Get or create the singleton system state
    pub async fn get_or_create(&self) -> RepoResult<SystemState> {
        // Try to get existing
        if let Some(state) = self.get().await? {
            return Ok(state);
        }

        // Create new singleton (直接使用 SystemState，无需 workaround)
        let now = shared::util::now_millis();
        let state = SystemState {
            id: None,
            genesis_hash: None,
            last_order: None,
            last_order_hash: None,
            synced_up_to: None,
            synced_up_to_hash: None,
            last_sync_time: None,
            order_count: 0,
            created_at: now,
            updated_at: now,
        };

        let created: Option<SystemState> = self
            .base
            .db()
            .create((TABLE, SINGLETON_ID))
            .content(state)
            .await?;
        created.ok_or_else(|| RepoError::Database("Failed to create system state".to_string()))
    }

    /// Get the singleton system state
    pub async fn get(&self) -> RepoResult<Option<SystemState>> {
        let state: Option<SystemState> = self.base.db().select((TABLE, SINGLETON_ID)).await?;
        Ok(state)
    }

    /// Update system state
    pub async fn update(&self, data: SystemStateUpdate) -> RepoResult<SystemState> {
        // Ensure singleton exists
        self.get_or_create().await?;

        let singleton_id = RecordId::from_table_key(TABLE, SINGLETON_ID);
        let mut merge_data = serde_json::to_value(&data)
            .map_err(|e| RepoError::Database(format!("Serialize error: {}", e)))?;
        if let Some(obj) = merge_data.as_object_mut() {
            obj.insert("updated_at".to_string(), serde_json::json!(shared::util::now_millis()));
        }
        let mut result = self
            .base
            .db()
            .query("UPDATE $id MERGE $data RETURN AFTER")
            .bind(("id", singleton_id))
            .bind(("data", merge_data))
            .await?;

        result
            .take::<Option<SystemState>>(0)?
            .ok_or_else(|| RepoError::Database("Failed to update system state".to_string()))
    }

    /// Initialize genesis hash
    pub async fn init_genesis(&self, genesis_hash: String) -> RepoResult<SystemState> {
        self.update(SystemStateUpdate {
            genesis_hash: Some(genesis_hash),
            ..Default::default()
        })
        .await
    }

    /// Atomically increment order_count and return the new value
    /// Used for generating receipt numbers
    pub async fn get_next_order_number(&self) -> RepoResult<i32> {
        // Ensure singleton exists
        self.get_or_create().await?;

        let singleton_id = RecordId::from_table_key(TABLE, SINGLETON_ID);
        let mut result = self
            .base
            .db()
            .query(
                "UPDATE system_state SET \
                    order_count = order_count + 1, \
                    updated_at = $now \
                WHERE id = $id RETURN AFTER.order_count",
            )
            .bind(("id", singleton_id))
            .bind(("now", shared::util::now_millis()))
            .await?;

        let new_count: Option<i32> = result.take(0)?;
        new_count.ok_or_else(|| RepoError::Database("Failed to get next order number".to_string()))
    }

    /// Update last order info with atomic order_count increment
    /// order_id should be in "order:xxx" format
    pub async fn update_last_order(
        &self,
        order_id: &str,
        order_hash: String,
    ) -> RepoResult<SystemState> {
        let order_thing = order_id
            .parse::<RecordId>()
            .map_err(|_| RepoError::Validation(format!("Invalid order ID: {}", order_id)))?;

        // Ensure singleton exists
        self.get_or_create().await?;

        // Use atomic increment for order_count to avoid race conditions
        let singleton_id = RecordId::from_table_key(TABLE, SINGLETON_ID);
        let mut result = self
            .base
            .db()
            .query(
                "UPDATE system_state SET \
                    last_order = $order_id, \
                    last_order_hash = $hash, \
                    order_count = order_count + 1, \
                    updated_at = $now \
                WHERE id = $id RETURN AFTER",
            )
            .bind(("order_id", order_thing))
            .bind(("hash", order_hash))
            .bind(("id", singleton_id))
            .bind(("now", shared::util::now_millis()))
            .await?;

        let updated: Option<SystemState> = result.take(0)?;
        updated.ok_or_else(|| RepoError::Database("Failed to update system state".to_string()))
    }

    /// Update sync state
    /// synced_up_to_id should be in "order:xxx" format
    pub async fn update_sync_state(
        &self,
        synced_up_to_id: &str,
        synced_up_to_hash: String,
    ) -> RepoResult<SystemState> {
        let synced_thing = synced_up_to_id
            .parse::<RecordId>()
            .map_err(|_| RepoError::Validation(format!("Invalid order ID: {}", synced_up_to_id)))?;

        self.update(SystemStateUpdate {
            synced_up_to: Some(synced_thing),
            synced_up_to_hash: Some(synced_up_to_hash),
            last_sync_time: Some(shared::util::now_millis()),
            ..Default::default()
        })
        .await
    }

    /// Get pending orders for sync (orders after synced_up_to)
    pub async fn get_pending_sync_orders(&self) -> RepoResult<Vec<crate::db::models::Order>> {
        let state = self.get_or_create().await?;

        match state.synced_up_to {
            Some(synced_order) => {
                // Get all orders created after the synced order
                let mut result = self
                    .base
                    .db()
                    .query(
                        r#"
                        LET $synced_time = (SELECT created_at FROM order WHERE id = $synced_id)[0].created_at;
                        SELECT * FROM order WHERE created_at > $synced_time ORDER BY created_at;
                        "#,
                    )
                    .bind(("synced_id", synced_order))
                    .await?;
                let orders: Vec<crate::db::models::Order> = result.take(1)?;
                Ok(orders)
            }
            None => {
                // No sync yet, return all orders
                let mut result = self
                    .base
                    .db()
                    .query("SELECT * FROM order ORDER BY created_at")
                    .await?;
                let orders: Vec<crate::db::models::Order> = result.take(0)?;
                Ok(orders)
            }
        }
    }
}
