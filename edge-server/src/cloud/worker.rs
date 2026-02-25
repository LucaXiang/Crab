//! CloudWorker — background worker with WebSocket duplex + HTTP sync
//!
//! 1. Connect WebSocket to crab-cloud (mTLS)
//! 2. Wait for Welcome{cursors} → compare with local ResourceVersions → incremental sync
//! 3. Archived order catch-up sync via HTTP (strong consistency)
//! 4. Listen for MessageBus broadcasts → debounced push via WS (products/categories)
//! 5. Listen for WS incoming → Command execution (cloud→edge only)
//! 6. Reconnect with exponential backoff on disconnect

use futures::{SinkExt, StreamExt};
use shared::cloud::{CloudMessage, CloudSyncBatch, CloudSyncItem, SyncResource};
use shared::message::{BusMessage, EventType, SyncChangeType, SyncPayload};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::time::{Duration, Instant};
use tokio_tungstenite::tungstenite::Message;
use tokio_util::sync::CancellationToken;

use crate::cloud::service::CloudService;
use crate::core::state::ServerState;
use crate::db::repository::order;

/// Debounce window for batching changes
const DEBOUNCE_MS: u64 = 500;
/// Max retry attempts for HTTP fallback
const MAX_RETRIES: u32 = 3;
/// Initial retry delay (1s for fast first reconnect, then exponential backoff)
const INITIAL_RETRY_DELAY_SECS: u64 = 1;
/// Max reconnect delay
const MAX_RECONNECT_DELAY_SECS: u64 = 120;
/// Archived order sync batch size
const ARCHIVED_ORDER_BATCH_SIZE: i64 = 50;
/// Archived order sync interval (aggregate before pushing)
const ARCHIVED_ORDER_SYNC_INTERVAL_SECS: u64 = 300; // 5 minutes
/// WebSocket keepalive ping interval
const WS_PING_INTERVAL_SECS: u64 = 30;

/// Timeout for receiving Welcome message after WS connect
const WELCOME_TIMEOUT_SECS: u64 = 5;

pub struct CloudWorker {
    state: ServerState,
    cloud_service: Arc<CloudService>,
    shutdown: CancellationToken,
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

        // 1. Wait for Welcome{cursors} from cloud (timeout 5s)
        let cursors = match self.wait_for_welcome(&mut ws_stream).await {
            Some(c) => c,
            None => {
                // Timeout or error — fall back to full sync (empty cursors = cloud has nothing)
                tracing::warn!("No Welcome received, falling back to full sync");
                HashMap::new()
            }
        };

        // 2. Incremental sync based on cursors
        if let Err(e) = self.send_initial_sync(&cursors, &mut ws_sink).await {
            tracing::error!("Initial sync failed: {e}");
            return;
        }

        // 3. Archived order catch-up sync via HTTP (request-response, strong consistency)
        if let Err(e) = self.sync_archived_orders_http().await {
            tracing::error!("Archived order catch-up sync failed: {e}");
            // Non-fatal, continue with live sync
        }

        // 4. 订阅 MessageBus（在推送活跃订单之前，确保推送期间的事件不丢失）
        let mut broadcast_rx = self.state.message_bus().subscribe();

        // 5. 推送全量活跃订单到 cloud
        if let Err(e) = self.push_active_orders_full(&mut ws_sink).await {
            tracing::warn!("Failed to push initial active orders: {e}");
            // Non-fatal: console 会在 edge 下一次事件时更新
        }

        let mut ping_interval = tokio::time::interval(Duration::from_secs(WS_PING_INTERVAL_SECS));
        ping_interval.tick().await; // skip immediate tick

        let mut archived_order_sync_interval =
            tokio::time::interval(Duration::from_secs(ARCHIVED_ORDER_SYNC_INTERVAL_SECS));
        archived_order_sync_interval.tick().await; // skip immediate tick (already did catch-up above)

        let mut pending: HashMap<SyncResource, HashMap<String, CloudSyncItem>> = HashMap::new();
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

                // Keepalive ping
                _ = ping_interval.tick() => {
                    if ws_sink.send(Message::Ping(vec![].into())).await.is_err() {
                        tracing::warn!("WS ping failed, disconnecting");
                        return;
                    }
                }

                // Periodic archived order sync via HTTP (5 min interval, fallback scan)
                _ = archived_order_sync_interval.tick() => {
                    if let Err(e) = self.sync_archived_orders_http().await {
                        tracing::warn!("Periodic archived order sync failed: {e}");
                    }
                }

                // Immediate push on archive completion (push + periodic scan design)
                _ = self.state.archive_notify.notified() => {
                    if let Err(e) = self.sync_archived_orders_http().await {
                        tracing::warn!("Archive-triggered sync failed: {e}");
                    }
                }

                // MessageBus broadcast → buffer for debounce (products, categories, etc.)
                result = broadcast_rx.recv() => {
                    match result {
                        Ok(msg) => {
                            // 活跃订单事件 → 立即推送快照到 cloud (不走 debounce)
                            if let Some(order_msg) = self.extract_order_push(&msg)
                                && let Ok(json) = serde_json::to_string(&order_msg)
                                && ws_sink.send(Message::Text(json.into())).await.is_err()
                            {
                                tracing::warn!("WS send order push failed, disconnecting");
                                return;
                            }

                            if let Some(item) = Self::extract_sync_item(&msg) {
                                let resource = item.resource;
                                let resource_id = item.resource_id.clone();
                                pending
                                    .entry(resource)
                                    .or_default()
                                    .insert(resource_id, item);
                                debounce_deadline = Some(Instant::now() + Duration::from_millis(DEBOUNCE_MS));
                            }
                        }
                        Err(broadcast::error::RecvError::Lagged(n)) => {
                            tracing::warn!("CloudWorker lagged {n} messages, sending full sync");
                            debounce_deadline = None;
                            pending.clear();
                            if let Err(e) = self.send_initial_sync(&HashMap::new(), &mut ws_sink).await {
                                tracing::error!("Recovery full sync failed: {e}");
                                return;
                            }
                            if let Err(e) = self.push_active_orders_full(&mut ws_sink).await {
                                tracing::warn!("Recovery active order push failed: {e}");
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
            CloudMessage::Rpc { id, payload } => {
                tracing::info!(rpc_id = %id, "Received RPC via WS");
                let result = self.handle_rpc(&payload).await;
                tracing::info!(rpc_id = %id, kind = ?std::mem::discriminant(&result), "RPC executed");

                let reply = CloudMessage::RpcResult { id, result };
                if let Ok(json) = serde_json::to_string(&reply)
                    && let Err(e) = ws_sink.send(Message::Text(json.into())).await
                {
                    tracing::warn!("Failed to send RPC result via WS: {e}");
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

    /// Handle a strongly-typed RPC payload
    async fn handle_rpc(&self, payload: &shared::cloud::CloudRpc) -> shared::cloud::CloudRpcResult {
        use shared::cloud::CloudRpcResult;

        match payload {
            shared::cloud::CloudRpc::GetStatus => {
                let active_orders = self
                    .state
                    .orders_manager
                    .get_active_orders()
                    .map(|o| o.len())
                    .unwrap_or(0);
                let products = self.state.catalog_service.list_products().len();
                let categories = self.state.catalog_service.list_categories().len();
                CloudRpcResult::Json {
                    success: true,
                    data: Some(serde_json::json!({
                        "active_orders": active_orders,
                        "products": products,
                        "categories": categories,
                        "epoch": self.state.epoch,
                    })),
                    error: None,
                }
            }
            shared::cloud::CloudRpc::GetOrderDetail { order_key } => {
                match sqlx::query_scalar::<_, i64>(
                    "SELECT id FROM archived_order WHERE order_key = ? LIMIT 1",
                )
                .bind(order_key)
                .fetch_optional(&self.state.pool)
                .await
                {
                    Ok(Some(pk)) => {
                        match crate::db::repository::order::build_order_detail_sync(
                            &self.state.pool,
                            pk,
                        )
                        .await
                        {
                            Ok(detail) => CloudRpcResult::Json {
                                success: true,
                                data: serde_json::to_value(&detail).ok(),
                                error: None,
                            },
                            Err(e) => CloudRpcResult::Json {
                                success: false,
                                data: None,
                                error: Some(e.to_string()),
                            },
                        }
                    }
                    Ok(None) => CloudRpcResult::Json {
                        success: false,
                        data: None,
                        error: Some(format!("Order not found: {order_key}")),
                    },
                    Err(e) => CloudRpcResult::Json {
                        success: false,
                        data: None,
                        error: Some(e.to_string()),
                    },
                }
            }
            shared::cloud::CloudRpc::RefreshSubscription => {
                self.state.activation.sync_subscription().await;
                CloudRpcResult::Json {
                    success: true,
                    data: Some(serde_json::json!({ "message": "Subscription refresh triggered" })),
                    error: None,
                }
            }
            shared::cloud::CloudRpc::StoreOp { op, changed_at } => {
                let result = crate::cloud::rpc_executor::execute_catalog_op(
                    &self.state,
                    op.as_ref(),
                    *changed_at,
                )
                .await;
                CloudRpcResult::StoreOp(Box::new(result))
            }
        }
    }

    /// Wait for Welcome message from cloud (timeout)
    async fn wait_for_welcome(
        &self,
        ws_stream: &mut futures::stream::SplitStream<crate::cloud::service::WsStream>,
    ) -> Option<HashMap<String, u64>> {
        let timeout = Duration::from_secs(WELCOME_TIMEOUT_SECS);

        match tokio::time::timeout(timeout, async {
            while let Some(msg) = ws_stream.next().await {
                match msg {
                    Ok(Message::Text(text)) => {
                        if let Ok(CloudMessage::Welcome { cursors }) =
                            serde_json::from_str::<CloudMessage>(&text)
                        {
                            return Some(cursors);
                        }
                        // Not a Welcome — unexpected first message
                        tracing::warn!("Expected Welcome but got other message");
                        return None;
                    }
                    Ok(Message::Ping(_)) | Ok(Message::Pong(_)) => continue,
                    _ => return None,
                }
            }
            None
        })
        .await
        {
            Ok(result) => result,
            Err(_) => {
                tracing::warn!("Timed out waiting for Welcome message");
                None
            }
        }
    }

    /// Send initial sync based on cloud cursors — only send resources where local version > cursor
    async fn send_initial_sync<S>(
        &mut self,
        cursors: &HashMap<String, u64>,
        ws_sink: &mut S,
    ) -> Result<(), crate::utils::AppError>
    where
        S: futures::Sink<Message, Error = tokio_tungstenite::tungstenite::Error> + Unpin,
    {
        let items = self.collect_sync_items_incremental(cursors).await;

        if items.is_empty() {
            tracing::info!("Initial sync: all resources up-to-date, zero transfer");
            return Ok(());
        }

        let total = items.len();
        let msg = CloudMessage::SyncBatch {
            items,
            sent_at: shared::util::now_millis(),
        };

        let json = serde_json::to_string(&msg)
            .map_err(|e| crate::utils::AppError::internal(format!("Serialize SyncBatch: {e}")))?;
        ws_sink
            .send(Message::Text(json.into()))
            .await
            .map_err(|e| crate::utils::AppError::internal(format!("WS send failed: {e}")))?;

        tracing::info!("Initial sync sent: {total} items via WS");
        Ok(())
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
            let mut skipped_ids: Vec<i64> = Vec::new();

            for &id in &ids {
                match order::build_order_detail_sync(&self.state.pool, id).await {
                    Ok(detail_sync) => {
                        let data = match serde_json::to_value(&detail_sync) {
                            Ok(v) => v,
                            Err(e) => {
                                tracing::error!(
                                    order_id = id,
                                    "Failed to serialize OrderDetailSync, skipping: {e}"
                                );
                                skipped_ids.push(id);
                                continue;
                            }
                        };
                        items.push(CloudSyncItem {
                            resource: SyncResource::ArchivedOrder,
                            version: id as u64,
                            action: shared::cloud::SyncAction::Upsert,
                            resource_id: detail_sync.order_key,
                            data,
                        });
                        synced_ids.push(id);
                    }
                    Err(e) => {
                        tracing::error!(
                            order_id = id,
                            "Failed to build OrderDetailSync, skipping: {e}"
                        );
                        skipped_ids.push(id);
                    }
                }
            }

            // Mark permanently failed orders as synced to unblock the queue
            if !skipped_ids.is_empty() {
                tracing::warn!(
                    count = skipped_ids.len(),
                    ids = ?skipped_ids,
                    "Skipped unbuildable orders, marking as synced to prevent queue blockage"
                );
                if let Err(e) = order::mark_cloud_synced(&self.state.pool, &skipped_ids).await {
                    tracing::error!("Failed to mark skipped orders as synced: {e}");
                }
            }

            if items.is_empty() {
                if skipped_ids.is_empty() {
                    break; // No more orders
                }
                continue; // All were skipped, try next batch
            }

            let batch_count = items.len();
            let batch = CloudSyncBatch {
                edge_id: self.cloud_service.edge_id().to_string(),
                items,
                sent_at: shared::util::now_millis(),
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

            // Stop if this was the last batch
            if (synced_ids.len() as i64) < ARCHIVED_ORDER_BATCH_SIZE {
                break;
            }
        }

        Ok(())
    }

    /// Flush pending debounced items via WebSocket
    async fn flush_pending_ws<S>(
        &mut self,
        ws_sink: &mut S,
        pending: &mut HashMap<SyncResource, HashMap<String, CloudSyncItem>>,
    ) -> Result<(), crate::utils::AppError>
    where
        S: futures::Sink<Message, Error = tokio_tungstenite::tungstenite::Error> + Unpin,
    {
        let items: Vec<CloudSyncItem> = pending
            .drain()
            .flat_map(|(_, items)| items.into_values())
            .collect();

        if items.is_empty() {
            return Ok(());
        }

        let count = items.len();
        let msg = CloudMessage::SyncBatch {
            items,
            sent_at: shared::util::now_millis(),
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

    /// Collect sync items with cursor-based skip (used by send_initial_sync)
    async fn collect_sync_items_incremental(
        &self,
        cursors: &HashMap<String, u64>,
    ) -> Vec<CloudSyncItem> {
        let mut items = Vec::new();

        for &resource in SyncResource::INITIAL_SYNC {
            let local_version = self.state.resource_versions.get(resource);
            let cloud_cursor = cursors.get(resource.as_str()).copied().unwrap_or(0);

            if local_version == cloud_cursor && local_version > 0 {
                tracing::debug!(resource = %resource, version = local_version, "Skipping sync: up-to-date");
                continue;
            }

            self.collect_resource_items(resource, local_version, &mut items)
                .await;
        }

        items
    }

    /// Collect all sync items (all resources) — used by HTTP fallback
    async fn collect_all_sync_items(&self) -> Vec<CloudSyncItem> {
        let mut items = Vec::new();
        for &resource in SyncResource::INITIAL_SYNC {
            let version = self.state.resource_versions.get(resource);
            self.collect_resource_items(resource, version, &mut items)
                .await;
        }
        items
    }

    /// Collect items for a single resource type from local DB
    async fn collect_resource_items(
        &self,
        resource: SyncResource,
        version: u64,
        items: &mut Vec<CloudSyncItem>,
    ) {
        use crate::db::repository::{
            attribute, dining_table, employee, label_template, price_rule, store_info, tag, zone,
        };

        /// Push serializable records with an `id` field into sync items
        fn push_many<T: serde::Serialize>(
            records: &[T],
            resource: SyncResource,
            version: u64,
            id_fn: impl Fn(&T) -> i64,
            items: &mut Vec<CloudSyncItem>,
        ) {
            for record in records {
                if let Ok(data) = serde_json::to_value(record) {
                    items.push(CloudSyncItem {
                        resource,
                        version,
                        action: shared::cloud::SyncAction::Upsert,
                        resource_id: id_fn(record).to_string(),
                        data,
                    });
                }
            }
        }

        match resource {
            SyncResource::Product => {
                let products = self.state.catalog_service.list_products();
                push_many(&products, resource, version, |p| p.id, items);
            }
            SyncResource::Category => {
                let categories = self.state.catalog_service.list_categories();
                push_many(&categories, resource, version, |c| c.id, items);
            }
            SyncResource::Tag => {
                if let Ok(v) = tag::find_all(&self.state.pool).await {
                    push_many(&v, resource, version, |t| t.id, items);
                }
            }
            SyncResource::Attribute => {
                if let Ok(v) = attribute::find_all(&self.state.pool).await {
                    push_many(&v, resource, version, |a| a.id, items);
                }
            }
            SyncResource::AttributeBinding => {
                if let Ok(v) = sqlx::query_as::<_, shared::models::attribute::AttributeBinding>(
                    "SELECT id, owner_type, owner_id, attribute_id, is_required, display_order, \
                     COALESCE(default_option_ids, 'null') as default_option_ids \
                     FROM attribute_binding ORDER BY display_order",
                )
                .fetch_all(&self.state.pool)
                .await
                {
                    push_many(&v, resource, version, |b| b.id, items);
                }
            }
            SyncResource::Zone => {
                if let Ok(v) = zone::find_all(&self.state.pool).await {
                    push_many(&v, resource, version, |z| z.id, items);
                }
            }
            SyncResource::DiningTable => {
                if let Ok(v) = dining_table::find_all(&self.state.pool).await {
                    push_many(&v, resource, version, |t| t.id, items);
                }
            }
            SyncResource::Employee => {
                if let Ok(v) = employee::find_all_with_inactive(&self.state.pool).await {
                    push_many(&v, resource, version, |e| e.id, items);
                }
            }
            SyncResource::PriceRule => {
                if let Ok(v) = price_rule::find_all(&self.state.pool).await {
                    push_many(&v, resource, version, |r| r.id, items);
                }
            }
            SyncResource::LabelTemplate => {
                if let Ok(v) = label_template::list_all(&self.state.pool).await {
                    push_many(&v, resource, version, |t| t.id, items);
                }
            }
            SyncResource::StoreInfo => {
                if let Ok(Some(info)) = store_info::get(&self.state.pool).await
                    && let Ok(data) = serde_json::to_value(&info)
                {
                    items.push(CloudSyncItem {
                        resource,
                        version,
                        action: shared::cloud::SyncAction::Upsert,
                        resource_id: "1".into(),
                        data,
                    });
                }
            }
            _ => {}
        }
    }

    /// Full sync via HTTP POST (fallback when WS unavailable)
    async fn full_sync_http(&mut self) -> Result<(), crate::utils::AppError> {
        tracing::info!("Starting full cloud sync via HTTP fallback");
        let items = self.collect_all_sync_items().await;

        if items.is_empty() {
            return Ok(());
        }

        let total = items.len();
        let batch = CloudSyncBatch {
            edge_id: self.cloud_service.edge_id().to_string(),
            items,
            sent_at: shared::util::now_millis(),
        };

        let response = self.push_with_retry(batch).await?;
        tracing::info!(
            "HTTP full sync: {total} items, accepted={}, rejected={}",
            response.accepted,
            response.rejected,
        );

        Ok(())
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

    /// 向 cloud 推送全量活跃订单快照（连接建立时调用一次）
    async fn push_active_orders_full<S>(
        &self,
        ws_sink: &mut S,
    ) -> Result<(), crate::utils::AppError>
    where
        S: futures::Sink<Message, Error = tokio_tungstenite::tungstenite::Error> + Unpin,
    {
        let orders = self
            .state
            .orders_manager
            .get_active_orders()
            .map_err(|e| crate::utils::AppError::internal(format!("get_active_orders: {e}")))?;

        if orders.is_empty() {
            return Ok(());
        }

        let count = orders.len();
        for order in orders {
            let events = self
                .state
                .orders_manager
                .get_events_for_order(&order.order_id)
                .unwrap_or_default();
            let msg = CloudMessage::ActiveOrderSnapshot {
                snapshot: Box::new(order),
                events,
            };
            let json = serde_json::to_string(&msg).map_err(|e| {
                crate::utils::AppError::internal(format!("serialize ActiveOrderSnapshot: {e}"))
            })?;
            ws_sink
                .send(Message::Text(json.into()))
                .await
                .map_err(|e| {
                    crate::utils::AppError::internal(format!("WS send ActiveOrderSnapshot: {e}"))
                })?;
        }

        tracing::info!(count, "Active orders pushed to cloud on connect");
        Ok(())
    }

    /// 从 MessageBus 的 Sync 事件中提取活跃订单推送消息
    ///
    /// 订单事件 (resource = "order") → 读取最新快照 → ActiveOrderSnapshot/ActiveOrderRemoved
    fn extract_order_push(&self, msg: &BusMessage) -> Option<CloudMessage> {
        if msg.event_type != EventType::Sync {
            return None;
        }

        let payload: SyncPayload = serde_json::from_slice(&msg.payload).ok()?;
        if payload.resource != SyncResource::OrderSync {
            return None;
        }

        let order_id = &payload.id;

        // deleted = 订单终结 (completed/voided/merged)
        if payload.action == SyncChangeType::Deleted {
            return Some(CloudMessage::ActiveOrderRemoved {
                order_id: order_id.clone(),
            });
        }

        // created/updated = 活跃订单变更，读取最新快照 + 事件历史
        match self.state.orders_manager.get_snapshot(order_id) {
            Ok(Some(snap)) if snap.is_active() => {
                let events = self
                    .state
                    .orders_manager
                    .get_events_for_order(order_id)
                    .unwrap_or_default();
                Some(CloudMessage::ActiveOrderSnapshot {
                    snapshot: Box::new(snap),
                    events,
                })
            }
            Ok(Some(_)) => {
                // 非活跃状态（刚完成/作废），发送移除通知
                Some(CloudMessage::ActiveOrderRemoved {
                    order_id: order_id.clone(),
                })
            }
            Ok(None) => {
                // 快照不存在（已被清理），发送移除通知
                Some(CloudMessage::ActiveOrderRemoved {
                    order_id: order_id.clone(),
                })
            }
            Err(e) => {
                tracing::warn!(order_id, error = %e, "Failed to get snapshot for order push");
                None
            }
        }
    }

    /// Extract a CloudSyncItem from a BusMessage if it's a Sync event
    ///
    /// Returns None for cloud-originated changes to prevent echo back to cloud.
    fn extract_sync_item(msg: &BusMessage) -> Option<CloudSyncItem> {
        if msg.event_type != EventType::Sync {
            return None;
        }

        let payload: SyncPayload = serde_json::from_slice(&msg.payload).ok()?;

        // Skip cloud-originated changes to prevent echo/bounce
        if payload.cloud_origin {
            return None;
        }

        // Only forward resources that crab-cloud knows how to store
        if !payload.resource.is_cloud_synced() {
            return None;
        }

        let action = if payload.action == SyncChangeType::Deleted {
            shared::cloud::SyncAction::Delete
        } else {
            shared::cloud::SyncAction::Upsert
        };

        Some(CloudSyncItem {
            resource: payload.resource,
            version: payload.version,
            action,
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
