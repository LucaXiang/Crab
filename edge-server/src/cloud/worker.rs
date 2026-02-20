//! CloudWorker — background worker with WebSocket duplex + HTTP sync
//!
//! 1. Connect WebSocket to crab-cloud (mTLS)
//! 2. Full sync on connect (products + categories)
//! 3. Archived order catch-up sync via HTTP (strong consistency)
//! 4. Listen for MessageBus broadcasts → debounced push via WS (products/categories)
//! 5. Listen for WS incoming → Command execution (cloud→edge only)
//! 6. Periodic full sync every hour
//! 7. Reconnect with exponential backoff on disconnect

use futures::{SinkExt, StreamExt};
use shared::cloud::{CloudCommandResult, CloudMessage, CloudSyncBatch, CloudSyncItem};
use shared::message::{BusMessage, EventType, SyncPayload};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::time::{Duration, Instant};
use tokio_tungstenite::tungstenite::Message;
use tokio_util::sync::CancellationToken;

use crate::cloud::command_executor;
use crate::cloud::service::CloudService;
use crate::core::state::ServerState;
use crate::db::repository::order;

/// Debounce window for batching changes
const DEBOUNCE_MS: u64 = 500;
/// Full sync interval
const FULL_SYNC_INTERVAL_SECS: u64 = 3600;
/// Max retry attempts for HTTP fallback
const MAX_RETRIES: u32 = 3;
/// Initial retry delay
const INITIAL_RETRY_DELAY_SECS: u64 = 5;
/// Max reconnect delay
const MAX_RECONNECT_DELAY_SECS: u64 = 120;
/// Archived order sync batch size
const ARCHIVED_ORDER_BATCH_SIZE: i64 = 50;
/// Archived order sync interval (aggregate before pushing)
const ARCHIVED_ORDER_SYNC_INTERVAL_SECS: u64 = 300; // 5 minutes
/// WebSocket keepalive ping interval
const WS_PING_INTERVAL_SECS: u64 = 30;

pub struct CloudWorker {
    state: ServerState,
    cloud_service: Arc<CloudService>,
    shutdown: CancellationToken,
    /// Results from previously executed commands
    pending_results: Vec<CloudCommandResult>,
}

impl CloudWorker {
    pub fn new(
        state: ServerState,
        cloud_service: Arc<CloudService>,
        shutdown: CancellationToken,
    ) -> Self {
        Self {
            state,
            cloud_service,
            shutdown,
            pending_results: Vec::new(),
        }
    }

    /// Main run loop — connect WebSocket, handle messages, reconnect on failure
    pub async fn run(mut self) {
        tracing::info!("CloudWorker started");
        let mut reconnect_delay = Duration::from_secs(INITIAL_RETRY_DELAY_SECS);

        loop {
            // Check shutdown before attempting connection
            if self.shutdown.is_cancelled() {
                break;
            }

            let binding = match self.get_binding().await {
                Ok(b) => b,
                Err(e) => {
                    tracing::error!("CloudWorker: failed to get binding: {e}");
                    tokio::select! {
                        _ = self.shutdown.cancelled() => break,
                        _ = tokio::time::sleep(reconnect_delay) => {},
                    }
                    reconnect_delay =
                        (reconnect_delay * 2).min(Duration::from_secs(MAX_RECONNECT_DELAY_SECS));
                    continue;
                }
            };

            // Attempt WebSocket connection
            match self.cloud_service.connect_ws(&binding).await {
                Ok(ws) => {
                    reconnect_delay = Duration::from_secs(INITIAL_RETRY_DELAY_SECS);
                    self.run_ws_session(ws).await;
                }
                Err(e) => {
                    tracing::warn!(
                        delay_secs = reconnect_delay.as_secs(),
                        "WebSocket connection failed, falling back to HTTP then retry: {e}"
                    );
                    // HTTP fallback: do a full sync via POST
                    if let Err(e) = self.full_sync_http().await {
                        tracing::error!("HTTP fallback full sync failed: {e}");
                    }
                }
            }

            // Wait before reconnecting
            tokio::select! {
                _ = self.shutdown.cancelled() => break,
                _ = tokio::time::sleep(reconnect_delay) => {},
            }
            reconnect_delay =
                (reconnect_delay * 2).min(Duration::from_secs(MAX_RECONNECT_DELAY_SECS));
        }

        tracing::info!("CloudWorker stopped");
    }

    /// Run a single WebSocket session until disconnect or shutdown
    async fn run_ws_session(&mut self, ws: crate::cloud::service::WsStream) {
        let (mut ws_sink, mut ws_stream) = ws.split();

        // 1. Full sync (products + categories) via WS
        if let Err(e) = self.send_full_sync(&mut ws_sink).await {
            tracing::error!("Initial full sync via WS failed: {e}");
            return;
        }

        // 2. Archived order catch-up sync via HTTP (request-response, strong consistency)
        if let Err(e) = self.sync_archived_orders_http().await {
            tracing::error!("Archived order catch-up sync failed: {e}");
            // Non-fatal, continue with live sync
        }

        // 3. Enter main select loop
        let mut broadcast_rx = self.state.message_bus().subscribe();
        let mut full_sync_interval =
            tokio::time::interval(Duration::from_secs(FULL_SYNC_INTERVAL_SECS));
        full_sync_interval.tick().await; // skip immediate tick

        let mut ping_interval = tokio::time::interval(Duration::from_secs(WS_PING_INTERVAL_SECS));
        ping_interval.tick().await; // skip immediate tick

        let mut archived_order_sync_interval =
            tokio::time::interval(Duration::from_secs(ARCHIVED_ORDER_SYNC_INTERVAL_SECS));
        archived_order_sync_interval.tick().await; // skip immediate tick (already did catch-up above)

        let mut pending: HashMap<String, HashMap<String, CloudSyncItem>> = HashMap::new();
        let mut debounce_deadline: Option<Instant> = None;

        loop {
            let sleep_until =
                debounce_deadline.unwrap_or_else(|| Instant::now() + Duration::from_secs(3600));

            tokio::select! {
                _ = self.shutdown.cancelled() => {
                    tracing::info!("CloudWorker shutting down, flushing pending");
                    if !pending.is_empty() {
                        let _ = self.flush_pending_ws(&mut ws_sink, &mut pending).await;
                    }
                    let _ = ws_sink.close().await;
                    return;
                }

                // Debounce timer fired → flush
                _ = tokio::time::sleep_until(sleep_until), if debounce_deadline.is_some() => {
                    if let Err(e) = self.flush_pending_ws(&mut ws_sink, &mut pending).await {
                        tracing::error!("WS flush failed, disconnecting: {e}");
                        return;
                    }
                    debounce_deadline = None;
                }

                // Periodic full sync
                _ = full_sync_interval.tick() => {
                    if let Err(e) = self.send_full_sync(&mut ws_sink).await {
                        tracing::error!("Periodic full sync via WS failed: {e}");
                        return;
                    }
                }

                // Keepalive ping
                _ = ping_interval.tick() => {
                    if ws_sink.send(Message::Ping(vec![].into())).await.is_err() {
                        tracing::warn!("WS ping failed, disconnecting");
                        return;
                    }
                }

                // Periodic archived order sync via HTTP (5 min interval)
                _ = archived_order_sync_interval.tick() => {
                    if let Err(e) = self.sync_archived_orders_http().await {
                        tracing::warn!("Periodic archived order sync failed: {e}");
                    }
                }

                // MessageBus broadcast → buffer for debounce (products, categories, etc.)
                result = broadcast_rx.recv() => {
                    match result {
                        Ok(msg) => {
                            if let Some(item) = Self::extract_sync_item(&msg) {
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
                            tracing::warn!("CloudWorker lagged {n} messages, scheduling full sync");
                            debounce_deadline = None;
                            pending.clear();
                            if let Err(e) = self.send_full_sync(&mut ws_sink).await {
                                tracing::error!("Recovery full sync failed: {e}");
                                return;
                            }
                        }
                        Err(broadcast::error::RecvError::Closed) => {
                            tracing::info!("Broadcast channel closed, CloudWorker stopping");
                            return;
                        }
                    }
                }

                // Incoming WS message
                msg = ws_stream.next() => {
                    match msg {
                        Some(Ok(Message::Text(text))) => {
                            self.handle_ws_message(&text, &mut ws_sink).await;
                        }
                        Some(Ok(Message::Ping(data))) => {
                            let _ = ws_sink.send(Message::Pong(data)).await;
                        }
                        Some(Ok(Message::Close(_))) => {
                            tracing::info!("WebSocket closed by server");
                            return;
                        }
                        Some(Err(e)) => {
                            tracing::warn!("WebSocket error: {e}");
                            return;
                        }
                        None => {
                            tracing::info!("WebSocket stream ended");
                            return;
                        }
                        _ => {} // Binary, Pong — ignore
                    }
                }
            }
        }
    }

    /// Handle an incoming WebSocket message from cloud
    async fn handle_ws_message<S>(&mut self, text: &str, ws_sink: &mut S)
    where
        S: futures::Sink<Message, Error = tokio_tungstenite::tungstenite::Error> + Unpin,
    {
        let cloud_msg: CloudMessage = match serde_json::from_str(text) {
            Ok(m) => m,
            Err(e) => {
                tracing::warn!("Invalid CloudMessage from cloud: {e}");
                return;
            }
        };

        match cloud_msg {
            CloudMessage::Command(cmd) => {
                tracing::info!(
                    command_id = %cmd.id,
                    command_type = %cmd.command_type,
                    "Received cloud command via WS"
                );
                let result = command_executor::execute(&self.state, &cmd).await;
                tracing::info!(
                    command_id = %cmd.id,
                    success = result.success,
                    "Cloud command executed"
                );

                // Send result immediately via WS
                let reply = CloudMessage::CommandResult {
                    results: vec![result],
                };
                if let Ok(json) = serde_json::to_string(&reply)
                    && let Err(e) = ws_sink.send(Message::Text(json.into())).await
                {
                    tracing::warn!("Failed to send command result via WS: {e}");
                }
            }
            CloudMessage::SyncAck {
                accepted,
                rejected,
                errors,
            } => {
                if rejected > 0 {
                    tracing::warn!(
                        accepted,
                        rejected,
                        error_count = errors.len(),
                        "SyncAck with rejections"
                    );
                    for err in &errors {
                        tracing::warn!(
                            resource_id = %err.resource_id,
                            "Sync rejected: {}",
                            err.message
                        );
                    }
                } else {
                    tracing::debug!(accepted, "SyncAck OK");
                }
            }
            _ => {
                tracing::debug!("Ignoring unexpected CloudMessage variant from cloud");
            }
        }
    }

    /// Send full sync (products + categories) via WebSocket
    async fn send_full_sync<S>(&mut self, ws_sink: &mut S) -> Result<(), crate::utils::AppError>
    where
        S: futures::Sink<Message, Error = tokio_tungstenite::tungstenite::Error> + Unpin,
    {
        tracing::info!("Starting full cloud sync via WS");
        let items = self.collect_full_sync_items();

        if items.is_empty() && self.pending_results.is_empty() {
            tracing::info!("Full sync: nothing to sync");
            return Ok(());
        }

        let total = items.len();
        let msg = CloudMessage::SyncBatch {
            items,
            sent_at: shared::util::now_millis(),
            command_results: std::mem::take(&mut self.pending_results),
        };

        let json = serde_json::to_string(&msg)
            .map_err(|e| crate::utils::AppError::internal(format!("Serialize SyncBatch: {e}")))?;
        ws_sink
            .send(Message::Text(json.into()))
            .await
            .map_err(|e| crate::utils::AppError::internal(format!("WS send failed: {e}")))?;

        tracing::info!("Full sync sent: {total} items via WS");
        Ok(())
    }

    /// Collect full sync items (products + categories)
    fn collect_full_sync_items(&self) -> Vec<CloudSyncItem> {
        let mut items = Vec::new();

        let products = self.state.catalog_service.list_products();
        for product in &products {
            let data = match serde_json::to_value(product) {
                Ok(v) => v,
                Err(e) => {
                    tracing::warn!(product_id = %product.id, "Failed to serialize product: {e}");
                    continue;
                }
            };
            items.push(CloudSyncItem {
                resource: "product".to_string(),
                version: self.state.resource_versions.get("product"),
                action: "upsert".to_string(),
                resource_id: product.id.to_string(),
                data,
            });
        }

        let categories = self.state.catalog_service.list_categories();
        for category in &categories {
            let data = match serde_json::to_value(category) {
                Ok(v) => v,
                Err(e) => {
                    tracing::warn!(category_id = %category.id, "Failed to serialize category: {e}");
                    continue;
                }
            };
            items.push(CloudSyncItem {
                resource: "category".to_string(),
                version: self.state.resource_versions.get("category"),
                action: "upsert".to_string(),
                resource_id: category.id.to_string(),
                data,
            });
        }

        items
    }

    /// Catch-up sync for unsynced archived orders via HTTP POST (strong consistency).
    ///
    /// **严格有序**: 按 id 顺序处理，遇到构建失败计数，连续失败 3 次则跳过该订单。
    /// **强一致**: HTTP request-response — cloud 确认 accepted 后才标记 cloud_synced = 1。
    /// Hash 链要求上游订单必须先到达 cloud，否则链验证会断裂。
    ///
    /// 定时调用（5 分钟间隔 + 连接时立即执行），不走 WS。
    async fn sync_archived_orders_http(&mut self) -> Result<(), crate::utils::AppError> {
        let binding = self.get_binding().await?;

        loop {
            let ids =
                order::list_unsynced_archived_ids(&self.state.pool, ARCHIVED_ORDER_BATCH_SIZE)
                    .await
                    .map_err(|e| {
                        crate::utils::AppError::internal(format!(
                            "List unsynced archived orders: {e}"
                        ))
                    })?;

            if ids.is_empty() {
                break;
            }

            let mut items: Vec<CloudSyncItem> = Vec::with_capacity(ids.len());
            let mut synced_ids: Vec<i64> = Vec::with_capacity(ids.len());
            let mut build_failed = false;

            for &id in &ids {
                match order::build_order_detail_sync(&self.state.pool, id).await {
                    Ok(detail_sync) => {
                        let data = match serde_json::to_value(&detail_sync) {
                            Ok(v) => v,
                            Err(e) => {
                                tracing::error!(
                                    order_id = id,
                                    "Failed to serialize OrderDetailSync, stopping batch: {e}"
                                );
                                build_failed = true;
                                break;
                            }
                        };
                        items.push(CloudSyncItem {
                            resource: "archived_order".to_string(),
                            version: id as u64,
                            action: "upsert".to_string(),
                            resource_id: detail_sync.order_key,
                            data,
                        });
                        synced_ids.push(id);
                    }
                    Err(e) => {
                        tracing::error!(
                            order_id = id,
                            "Failed to build OrderDetailSync, stopping batch: {e}"
                        );
                        build_failed = true;
                        break;
                    }
                }
            }

            if items.is_empty() {
                tracing::warn!(
                    "Archived order catch-up stopped: first order in batch failed to build"
                );
                break;
            }

            let batch_count = items.len();
            let batch = CloudSyncBatch {
                edge_id: self.cloud_service.edge_id().to_string(),
                items,
                sent_at: shared::util::now_millis(),
                command_results: vec![],
            };

            // HTTP POST — synchronous request-response, cloud confirms storage
            let response = self
                .cloud_service
                .push_batch(batch, &binding)
                .await
                .map_err(|e| {
                    crate::utils::AppError::internal(format!("HTTP sync archived orders: {e}"))
                })?;

            if response.rejected > 0 {
                tracing::warn!(
                    accepted = response.accepted,
                    rejected = response.rejected,
                    "Archived order sync has rejections, stopping catch-up"
                );
                for err in &response.errors {
                    tracing::warn!(
                        resource_id = %err.resource_id,
                        "Rejected: {}", err.message
                    );
                }
                break;
            }

            // Cloud confirmed all accepted — mark as synced
            if let Err(e) = order::mark_cloud_synced(&self.state.pool, &synced_ids).await {
                tracing::error!("Failed to mark orders as cloud_synced, stopping catch-up: {e}");
                break;
            }

            tracing::info!(
                batch_size = batch_count,
                accepted = response.accepted,
                "Archived orders synced and confirmed via HTTP"
            );

            // Stop if we had a build failure or this was the last batch
            if build_failed || (synced_ids.len() as i64) < ARCHIVED_ORDER_BATCH_SIZE {
                break;
            }
        }

        Ok(())
    }

    /// Flush pending debounced items via WebSocket
    async fn flush_pending_ws<S>(
        &mut self,
        ws_sink: &mut S,
        pending: &mut HashMap<String, HashMap<String, CloudSyncItem>>,
    ) -> Result<(), crate::utils::AppError>
    where
        S: futures::Sink<Message, Error = tokio_tungstenite::tungstenite::Error> + Unpin,
    {
        let items: Vec<CloudSyncItem> = pending
            .drain()
            .flat_map(|(_, items)| items.into_values())
            .collect();

        if items.is_empty() && self.pending_results.is_empty() {
            return Ok(());
        }

        let count = items.len();
        let msg = CloudMessage::SyncBatch {
            items,
            sent_at: shared::util::now_millis(),
            command_results: std::mem::take(&mut self.pending_results),
        };

        let json = serde_json::to_string(&msg)
            .map_err(|e| crate::utils::AppError::internal(format!("Serialize SyncBatch: {e}")))?;
        ws_sink
            .send(Message::Text(json.into()))
            .await
            .map_err(|e| crate::utils::AppError::internal(format!("WS send failed: {e}")))?;

        tracing::debug!("Flushed {count} sync items via WS");
        Ok(())
    }

    /// Full sync via HTTP POST (fallback when WS unavailable)
    async fn full_sync_http(&mut self) -> Result<(), crate::utils::AppError> {
        tracing::info!("Starting full cloud sync via HTTP fallback");
        let items = self.collect_full_sync_items();

        if items.is_empty() && self.pending_results.is_empty() {
            return Ok(());
        }

        let total = items.len();
        let batch = CloudSyncBatch {
            edge_id: self.cloud_service.edge_id().to_string(),
            items,
            sent_at: shared::util::now_millis(),
            command_results: std::mem::take(&mut self.pending_results),
        };

        let response = self.push_with_retry(batch).await?;
        tracing::info!(
            "HTTP full sync: {total} items, accepted={}, rejected={}",
            response.accepted,
            response.rejected,
        );

        // Execute any pending commands from HTTP response
        self.handle_http_commands(response.pending_commands).await;

        Ok(())
    }

    /// Execute cloud commands and cache results
    async fn handle_http_commands(&mut self, commands: Vec<shared::cloud::CloudCommand>) {
        if commands.is_empty() {
            return;
        }

        tracing::info!(count = commands.len(), "Executing cloud commands from HTTP");

        for cmd in &commands {
            let result = command_executor::execute(&self.state, cmd).await;
            tracing::info!(
                command_id = %cmd.id,
                command_type = %cmd.command_type,
                success = result.success,
                "Cloud command executed"
            );
            self.pending_results.push(result);
        }
    }

    /// Push batch with exponential backoff retry (HTTP fallback)
    async fn push_with_retry(
        &self,
        batch: CloudSyncBatch,
    ) -> Result<shared::cloud::CloudSyncResponse, crate::utils::AppError> {
        let binding = self.get_binding().await?;
        let mut delay = Duration::from_secs(INITIAL_RETRY_DELAY_SECS);

        for attempt in 0..MAX_RETRIES {
            match self.cloud_service.push_batch(batch.clone(), &binding).await {
                Ok(response) => return Ok(response),
                Err(e) if attempt + 1 < MAX_RETRIES => {
                    tracing::warn!(
                        attempt = attempt + 1,
                        max_retries = MAX_RETRIES,
                        delay_secs = delay.as_secs(),
                        "HTTP sync attempt failed, retrying: {e}"
                    );
                    tokio::time::sleep(delay).await;
                    delay = (delay * 2).min(Duration::from_secs(60));
                }
                Err(e) => return Err(e),
            }
        }

        Err(crate::utils::AppError::internal(
            "push_with_retry: all retries exhausted",
        ))
    }

    /// Extract a CloudSyncItem from a BusMessage if it's a Sync event
    fn extract_sync_item(msg: &BusMessage) -> Option<CloudSyncItem> {
        if msg.event_type != EventType::Sync {
            return None;
        }

        let payload: SyncPayload = serde_json::from_slice(&msg.payload).ok()?;

        // Skip order_sync (real-time client events, not cloud sync material)
        // Skip archived_order (synced via periodic HTTP, not WS broadcast)
        if payload.resource == "order_sync" || payload.resource == "archived_order" {
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
