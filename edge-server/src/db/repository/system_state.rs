//! System State Repository (Singleton)

use super::{make_thing, BaseRepository, RepoError, RepoResult};
use crate::db::models::{SystemState, SystemStateUpdate};
use surrealdb::engine::local::Db;
use surrealdb::Surreal;

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

        // Create new singleton
        let state = SystemState {
            id: Some(make_thing(TABLE, SINGLETON_ID)),
            genesis_hash: None,
            last_order: None,
            last_order_hash: None,
            synced_up_to: None,
            synced_up_to_hash: None,
            last_sync_time: None,
            order_count: 0,
            created_at: None,
            updated_at: None,
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

        // Update timestamp first
        let _ = self
            .base
            .db()
            .query("UPDATE system_state SET updated_at = time::now() WHERE id = $id RETURN AFTER")
            .bind(("id", make_thing(TABLE, SINGLETON_ID)))
            .await?;

        // Merge the actual data
        let updated: Option<SystemState> = self
            .base
            .db()
            .update((TABLE, SINGLETON_ID))
            .merge(data)
            .await?;
        updated.ok_or_else(|| RepoError::Database("Failed to update system state".to_string()))
    }

    /// Initialize genesis hash
    pub async fn init_genesis(&self, genesis_hash: String) -> RepoResult<SystemState> {
        self.update(SystemStateUpdate {
            genesis_hash: Some(genesis_hash),
            ..Default::default()
        })
        .await
    }

    /// Update last order info
    pub async fn update_last_order(&self, order_id: &str, order_hash: String) -> RepoResult<SystemState> {
        let order_thing = make_thing("order", order_id);
        self.update(SystemStateUpdate {
            last_order: Some(order_thing),
            last_order_hash: Some(order_hash),
            order_count: Some(self.get().await?.map(|s| s.order_count + 1).unwrap_or(1)),
            ..Default::default()
        })
        .await
    }

    /// Update sync state
    pub async fn update_sync_state(
        &self,
        synced_up_to_id: &str,
        synced_up_to_hash: String,
    ) -> RepoResult<SystemState> {
        let order_thing = make_thing("order", synced_up_to_id);
        self.update(SystemStateUpdate {
            synced_up_to: Some(order_thing),
            synced_up_to_hash: Some(synced_up_to_hash),
            last_sync_time: Some(chrono::Utc::now().to_rfc3339()),
            ..Default::default()
        })
        .await
    }

    /// Get pending orders for sync (orders after synced_up_to)
    pub async fn get_pending_sync_orders(&self) -> RepoResult<Vec<crate::db::models::Order>> {
        let state = self.get_or_create().await?;

        let query = match state.synced_up_to {
            Some(synced_order) => {
                // Get all orders created after the synced order
                format!(
                    r#"
                    LET $synced_time = (SELECT created_at FROM order WHERE id = {})[0].created_at;
                    SELECT * FROM order WHERE created_at > $synced_time ORDER BY created_at;
                    "#,
                    synced_order
                )
            }
            None => {
                // No sync yet, return all orders
                "SELECT * FROM order ORDER BY created_at".to_string()
            }
        };

        let mut result = self.base.db().query(&query).await?;
        let orders: Vec<crate::db::models::Order> = result.take(0)?;
        Ok(orders)
    }
}
