//! CloudSyncWorker — background worker that syncs data to crab-cloud
//!
//! Subscribes to MessageBus server broadcast channel to detect resource changes,
//! debounces events, and pushes batches to crab-cloud.

use shared::cloud::{CloudSyncBatch, CloudSyncItem};
use shared::message::{BusMessage, EventType, SyncPayload};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::time::{Duration, Instant};
use tokio_util::sync::CancellationToken;

use crate::cloud_sync::CloudSyncService;
use crate::core::state::ServerState;

/// Debounce window for batching changes
const DEBOUNCE_MS: u64 = 500;
/// Full sync interval
const FULL_SYNC_INTERVAL_SECS: u64 = 3600;
/// Max retry attempts
const MAX_RETRIES: u32 = 3;
/// Initial retry delay
const INITIAL_RETRY_DELAY_SECS: u64 = 5;

pub struct CloudSyncWorker {
    state: ServerState,
    sync_service: Arc<CloudSyncService>,
    shutdown: CancellationToken,
}

impl CloudSyncWorker {
    pub fn new(
        state: ServerState,
        sync_service: Arc<CloudSyncService>,
        shutdown: CancellationToken,
    ) -> Self {
        Self {
            state,
            sync_service,
            shutdown,
        }
    }

    /// Run the cloud sync worker
    ///
    /// 1. Full sync on startup
    /// 2. Listen for broadcast events, debounce and push
    /// 3. Periodic full sync every hour
    pub async fn run(self) {
        tracing::info!("CloudSyncWorker started");

        // Full sync on startup
        if let Err(e) = self.full_sync().await {
            tracing::error!("Initial full sync failed: {e}");
        }

        // Subscribe to server broadcast
        let mut broadcast_rx = self.state.message_bus().subscribe();
        let mut full_sync_interval =
            tokio::time::interval(Duration::from_secs(FULL_SYNC_INTERVAL_SECS));
        full_sync_interval.tick().await; // skip immediate tick

        // Debounce buffer: resource_type -> (resource_id -> CloudSyncItem)
        let mut pending: HashMap<String, HashMap<String, CloudSyncItem>> = HashMap::new();
        let mut debounce_deadline: Option<Instant> = None;

        loop {
            let sleep_until =
                debounce_deadline.unwrap_or_else(|| Instant::now() + Duration::from_secs(3600));

            tokio::select! {
                _ = self.shutdown.cancelled() => {
                    tracing::info!("CloudSyncWorker shutting down");
                    if !pending.is_empty() {
                        self.flush_pending(&mut pending).await;
                    }
                    break;
                }

                _ = tokio::time::sleep_until(sleep_until), if debounce_deadline.is_some() => {
                    self.flush_pending(&mut pending).await;
                    debounce_deadline = None;
                }

                _ = full_sync_interval.tick() => {
                    if let Err(e) = self.full_sync().await {
                        tracing::error!("Periodic full sync failed: {e}");
                    }
                }

                result = broadcast_rx.recv() => {
                    match result {
                        Ok(msg) => {
                            if let Some(item) = self.extract_sync_item(&msg) {
                                let resource = item.resource.clone();
                                let resource_id = item.resource_id.clone();
                                pending
                                    .entry(resource)
                                    .or_default()
                                    .insert(resource_id, item);

                                debounce_deadline = Some(Instant::now() + Duration::from_millis(DEBOUNCE_MS));
                            }
                        }
                        Err(broadcast::error::RecvError::Lagged(n)) => {
                            tracing::warn!("CloudSyncWorker lagged {n} messages, scheduling full sync");
                            debounce_deadline = None;
                            pending.clear();
                            if let Err(e) = self.full_sync().await {
                                tracing::error!("Recovery full sync failed: {e}");
                            }
                        }
                        Err(broadcast::error::RecvError::Closed) => {
                            tracing::info!("Broadcast channel closed, CloudSyncWorker stopping");
                            break;
                        }
                    }
                }
            }
        }

        tracing::info!("CloudSyncWorker stopped");
    }

    /// Extract a CloudSyncItem from a BusMessage if it's a Sync event
    fn extract_sync_item(&self, msg: &BusMessage) -> Option<CloudSyncItem> {
        if msg.event_type != EventType::Sync {
            return None;
        }

        // Deserialize SyncPayload from binary payload
        let payload: SyncPayload = serde_json::from_slice(&msg.payload).ok()?;

        // Skip order_sync (real-time client events, not cloud sync material)
        if payload.resource == "order_sync" {
            return None;
        }

        Some(CloudSyncItem {
            resource: payload.resource,
            version: payload.version,
            action: payload.action,
            resource_id: payload.id,
            data: payload.data.unwrap_or(serde_json::Value::Null),
        })
    }

    /// Flush all pending items as a batch
    async fn flush_pending(&self, pending: &mut HashMap<String, HashMap<String, CloudSyncItem>>) {
        let items: Vec<CloudSyncItem> = pending
            .drain()
            .flat_map(|(_, items)| items.into_values())
            .collect();

        if items.is_empty() {
            return;
        }

        let count = items.len();
        let batch = CloudSyncBatch {
            edge_id: self.sync_service.edge_id().to_string(),
            items,
            sent_at: shared::util::now_millis(),
        };

        match self.push_with_retry(batch).await {
            Ok(resp) => {
                tracing::debug!(
                    accepted = resp.accepted,
                    rejected = resp.rejected,
                    "Flushed {count} sync items"
                );
            }
            Err(e) => {
                tracing::error!("Failed to push sync batch after retries: {e}");
            }
        }
    }

    /// Full sync — query all local data and push to cloud
    async fn full_sync(&self) -> Result<(), crate::utils::AppError> {
        tracing::info!("Starting full cloud sync");

        let mut items = Vec::new();

        // Sync products from catalog cache
        let products = self.state.catalog_service.list_products();
        for product in &products {
            let data = serde_json::to_value(product).unwrap_or_default();
            items.push(CloudSyncItem {
                resource: "product".to_string(),
                version: self.state.resource_versions.get("product"),
                action: "upsert".to_string(),
                resource_id: product.id.to_string(),
                data,
            });
        }

        // Sync categories from catalog cache
        let categories = self.state.catalog_service.list_categories();
        for category in &categories {
            let data = serde_json::to_value(category).unwrap_or_default();
            items.push(CloudSyncItem {
                resource: "category".to_string(),
                version: self.state.resource_versions.get("category"),
                action: "upsert".to_string(),
                resource_id: category.id.to_string(),
                data,
            });
        }

        // Sync active orders
        if let Ok(orders) = self.state.orders_manager.get_active_orders() {
            for order in &orders {
                let data = serde_json::to_value(order).unwrap_or_default();
                items.push(CloudSyncItem {
                    resource: "active_order".to_string(),
                    version: order.last_sequence,
                    action: "upsert".to_string(),
                    resource_id: order.order_id.clone(),
                    data,
                });
            }
        }

        if items.is_empty() {
            tracing::info!("Full sync: nothing to sync");
            return Ok(());
        }

        let total = items.len();
        let batch = CloudSyncBatch {
            edge_id: self.sync_service.edge_id().to_string(),
            items,
            sent_at: shared::util::now_millis(),
        };

        let response = self.push_with_retry(batch).await?;
        tracing::info!(
            "Full sync complete: {total} items, accepted={}, rejected={}",
            response.accepted,
            response.rejected,
        );

        Ok(())
    }

    /// Push batch with exponential backoff retry
    async fn push_with_retry(
        &self,
        batch: CloudSyncBatch,
    ) -> Result<shared::cloud::CloudSyncResponse, crate::utils::AppError> {
        let binding = self.get_binding().await?;

        let mut delay = Duration::from_secs(INITIAL_RETRY_DELAY_SECS);

        for attempt in 0..MAX_RETRIES {
            match self.sync_service.push_batch(batch.clone(), &binding).await {
                Ok(response) => return Ok(response),
                Err(e) if attempt + 1 < MAX_RETRIES => {
                    tracing::warn!(
                        attempt = attempt + 1,
                        max_retries = MAX_RETRIES,
                        delay_secs = delay.as_secs(),
                        "Cloud sync attempt failed, retrying: {e}"
                    );
                    tokio::time::sleep(delay).await;
                    delay = (delay * 2).min(Duration::from_secs(60));
                }
                Err(e) => return Err(e),
            }
        }

        unreachable!()
    }

    /// Get the current SignedBinding from activation service
    async fn get_binding(
        &self,
    ) -> Result<shared::activation::SignedBinding, crate::utils::AppError> {
        let cred = self
            .state
            .activation
            .get_credential()
            .await?
            .ok_or_else(|| {
                crate::utils::AppError::internal("Not activated, cannot sync to cloud")
            })?;
        Ok(cred.binding)
    }
}
