//! CloudWorker — background worker with WebSocket duplex + HTTP sync
//!
//! 1. Connect WebSocket to crab-cloud (mTLS)
//! 2. Wait for Welcome{cursors} → compare with local ResourceVersions → incremental sync
//! 3. Catch-up sync via HTTP: archived orders, credit notes, invoices, anulaciones (chain_entry order)
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
use crate::db::repository::{chain_entry, credit_note, invoice, order};

/// Debounce window for batching changes
const DEBOUNCE_MS: u64 = 500;
/// Max retry attempts for HTTP fallback
const MAX_RETRIES: u32 = 3;
/// Initial retry delay (1s for fast first reconnect, then exponential backoff)
const INITIAL_RETRY_DELAY_SECS: u64 = 1;
/// Max reconnect delay for transient errors
const MAX_RECONNECT_DELAY_SECS: u64 = 120;
/// Max reconnect delay when cloud rejects authentication (permanent failure until restart/reactivation)
const MAX_AUTH_FAIL_DELAY_SECS: u64 = 1800; // 30 minutes
/// Archived order sync batch size
const ARCHIVED_ORDER_BATCH_SIZE: i64 = 50;
/// Archived order sync interval (aggregate before pushing)
const ARCHIVED_ORDER_SYNC_INTERVAL_SECS: u64 = 300; // 5 minutes
/// WebSocket keepalive ping interval
const WS_PING_INTERVAL_SECS: u64 = 30;

/// Add random jitter (0..50% of delay) to prevent thundering herd
fn with_jitter(delay: Duration) -> Duration {
    use rand::Rng;
    let jitter_ms = rand::thread_rng().gen_range(0..=delay.as_millis() as u64 / 2);
    delay + Duration::from_millis(jitter_ms)
}

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
                        _ = tokio::time::sleep(with_jitter(reconnect_delay)) => {},
                    }
                    reconnect_delay =
                        (reconnect_delay * 2).min(Duration::from_secs(MAX_RECONNECT_DELAY_SECS));
                    continue;
                }
            };

            // Attempt WebSocket connection
            let max_delay = match self.cloud_service.connect_ws(&binding).await {
                Ok(ws) => {
                    reconnect_delay = Duration::from_secs(INITIAL_RETRY_DELAY_SECS);
                    self.run_ws_session(ws).await;
                    MAX_RECONNECT_DELAY_SECS
                }
                Err(e) if e.code == shared::error::ErrorCode::NotAuthenticated => {
                    // Permanent auth failure — back off aggressively (up to 30 min)
                    tracing::error!(
                        delay_secs = reconnect_delay.as_secs(),
                        "Cloud authentication failed (credentials may be stale): {e}"
                    );
                    MAX_AUTH_FAIL_DELAY_SECS
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
                    MAX_RECONNECT_DELAY_SECS
                }
            };

            // Wait before reconnecting (with jitter to prevent thundering herd)
            tokio::select! {
                _ = self.shutdown.cancelled() => break,
                _ = tokio::time::sleep(with_jitter(reconnect_delay)) => {},
            }
            reconnect_delay = (reconnect_delay * 2).min(Duration::from_secs(max_delay));
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

        // 1b. If local catalog is empty, request full catalog from Cloud (re-bind scenario)
        if self.state.catalog_service.list_products().is_empty() {
            tracing::info!("Local catalog is empty, requesting CatalogSyncData from cloud");
            let req = CloudMessage::RequestCatalogSync;
            if let Ok(json) = serde_json::to_string(&req) {
                if ws_sink.send(Message::Text(json.into())).await.is_err() {
                    tracing::error!("Failed to send RequestCatalogSync");
                    return;
                }
                // Wait for CatalogSyncData response (up to 30s)
                let deadline = Instant::now() + Duration::from_secs(30);
                loop {
                    let remaining = deadline.saturating_duration_since(Instant::now());
                    if remaining.is_zero() {
                        tracing::warn!("Timeout waiting for CatalogSyncData");
                        break;
                    }
                    tokio::select! {
                        _ = tokio::time::sleep(remaining) => {
                            tracing::warn!("Timeout waiting for CatalogSyncData");
                            break;
                        }
                        msg = ws_stream.next() => {
                            match msg {
                                Some(Ok(Message::Text(text))) => {
                                    if let Ok(CloudMessage::CatalogSyncData { catalog }) = serde_json::from_str(&text) {
                                        match crate::cloud::ops::provisioning::apply_catalog_sync_data(&self.state, &catalog).await {
                                            Ok(()) => tracing::info!("CatalogSyncData applied (re-bind)"),
                                            Err(e) => tracing::error!("Failed to apply CatalogSyncData: {e}"),
                                        }
                                        break;
                                    } else {
                                        // Handle other messages (e.g. RPCs for EnsureImage) inline
                                        self.handle_ws_message(&text, &mut ws_sink).await;
                                    }
                                }
                                Some(Ok(Message::Ping(data))) => {
                                    let _ = ws_sink.send(Message::Pong(data)).await;
                                }
                                Some(Ok(Message::Close(_))) | None => {
                                    tracing::warn!("WS closed while waiting for CatalogSyncData");
                                    return;
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
        }

        // 2. Incremental sync based on cursors
        if let Err(e) = self.send_initial_sync(&cursors, &mut ws_sink).await {
            tracing::error!("Initial sync failed: {e}");
            return;
        }

        // 2b. Upload pending catalog changelog entries (Edge→Cloud)
        if let Err(e) = self.send_catalog_changelog(&mut ws_sink).await {
            tracing::error!("Catalog changelog sync failed: {e}");
            // Non-fatal: will retry on next reconnect
        }

        // 3. Catch-up sync via HTTP (archived orders + credit notes + invoices)
        self.sync_archives_http("catch-up").await;

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

        let mut pending: HashMap<SyncResource, HashMap<i64, CloudSyncItem>> = HashMap::new();
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

                // Periodic archive sync via HTTP (5 min interval, fallback scan)
                _ = archived_order_sync_interval.tick() => {
                    self.sync_archives_http("periodic").await;
                }

                // Immediate push on archive completion
                _ = self.state.archive_notify.notified() => {
                    self.sync_archives_http("archive-triggered").await;
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
                                let resource_id = item.resource_id;
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

                let reply = CloudMessage::RpcResult {
                    id: id.clone(),
                    result,
                };
                match serde_json::to_string(&reply) {
                    Ok(json) => {
                        if let Err(e) = ws_sink.send(Message::Text(json.into())).await {
                            tracing::warn!(rpc_id = %id, "Failed to send RPC result via WS: {e}");
                        }
                    }
                    Err(e) => {
                        tracing::error!(rpc_id = %id, "Failed to serialize RPC result: {e}");
                    }
                }
            }
            CloudMessage::CatalogSyncData { catalog } => {
                tracing::info!("Received CatalogSyncData from cloud");
                match crate::cloud::ops::provisioning::apply_catalog_sync_data(
                    &self.state,
                    &catalog,
                )
                .await
                {
                    Ok(()) => {
                        tracing::info!("CatalogSyncData applied successfully");
                    }
                    Err(e) => {
                        tracing::error!("Failed to apply CatalogSyncData: {e}");
                    }
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
            shared::cloud::CloudRpc::GetOrderDetail { order_id } => {
                match sqlx::query_scalar::<_, i64>(
                    "SELECT id FROM archived_order WHERE id = ? LIMIT 1",
                )
                .bind(order_id)
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
                        error: Some(format!("Order not found: {order_id}")),
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

    /// Upload pending catalog_changelog entries to Cloud via WS SyncBatch.
    /// On success, marks entries as cloud_synced = 1.
    async fn send_catalog_changelog<S>(
        &mut self,
        ws_sink: &mut S,
    ) -> Result<(), crate::utils::AppError>
    where
        S: futures::Sink<Message, Error = tokio_tungstenite::tungstenite::Error> + Unpin,
    {
        let rows: Vec<(i64, String, i64, String, Option<String>, i64)> = sqlx::query_as(
            "SELECT id, resource, resource_id, action, data, updated_at FROM catalog_changelog WHERE cloud_synced = 0 ORDER BY id",
        )
        .fetch_all(&self.state.pool)
        .await
        .map_err(|e| crate::utils::AppError::internal(format!("Read catalog_changelog: {e}")))?;

        if rows.is_empty() {
            return Ok(());
        }

        let mut items: Vec<CloudSyncItem> = Vec::with_capacity(rows.len());
        let mut changelog_ids: Vec<i64> = Vec::with_capacity(rows.len());

        for (id, resource, resource_id, action, data, updated_at) in &rows {
            let sync_resource = match serde_json::from_value::<SyncResource>(
                serde_json::Value::String(resource.clone()),
            ) {
                Ok(r) => r,
                Err(_) => {
                    tracing::warn!(resource, "Unknown catalog_changelog resource, skipping");
                    continue;
                }
            };
            let sync_action = match action.as_str() {
                "upsert" => shared::cloud::SyncAction::Upsert,
                "delete" => shared::cloud::SyncAction::Delete,
                _ => {
                    tracing::warn!(action, "Unknown catalog_changelog action, skipping");
                    continue;
                }
            };
            let mut data_value: serde_json::Value = data
                .as_ref()
                .and_then(|d| serde_json::from_str(d).ok())
                .unwrap_or(serde_json::Value::Null);

            // Inject updated_at into data so Cloud can use it for LWW comparison
            if let serde_json::Value::Object(ref mut map) = data_value {
                map.insert(
                    "updated_at".to_string(),
                    serde_json::Value::Number((*updated_at).into()),
                );
            }

            items.push(CloudSyncItem {
                resource: sync_resource,
                version: *id as u64, // use changelog row id as version for cursor tracking
                action: sync_action,
                resource_id: *resource_id,
                data: data_value,
            });
            changelog_ids.push(*id);
        }

        if !items.is_empty() {
            let total = items.len();
            let msg = CloudMessage::SyncBatch {
                items,
                sent_at: shared::util::now_millis(),
            };
            let json = serde_json::to_string(&msg).map_err(|e| {
                crate::utils::AppError::internal(format!("Serialize catalog changelog: {e}"))
            })?;
            ws_sink
                .send(Message::Text(json.into()))
                .await
                .map_err(|e| {
                    crate::utils::AppError::internal(format!("WS send catalog changelog: {e}"))
                })?;
            tracing::info!("Catalog changelog sent: {total} items via WS");
        }

        // Mark all processed entries as synced
        for id in &changelog_ids {
            if let Err(e) =
                sqlx::query("UPDATE catalog_changelog SET cloud_synced = 1 WHERE id = ?")
                    .bind(id)
                    .execute(&self.state.pool)
                    .await
            {
                tracing::error!(changelog_id = %id, "Failed to mark catalog_changelog entry as synced: {e}");
            }
        }

        Ok(())
    }

    /// Sync archives to cloud via HTTP.
    ///
    /// Order layer: unified chain_entry sync (ORDER + CREDIT_NOTE + ANULACION + UPGRADE + BREAK)
    /// Invoice layer: independent invoice sync (huella chain)
    async fn sync_archives_http(&mut self, trigger: &str) {
        if let Err(e) = self.sync_chain_entries_http().await {
            tracing::warn!("{trigger}: chain entry sync failed: {e}");
        }
        if let Err(e) = self.sync_invoices_http().await {
            tracing::warn!("{trigger}: invoice sync failed: {e}");
        }
    }

    /// Unified chain entry sync — processes all chain_entry types in strict id order.
    ///
    /// This is the single source of truth for order-layer sync ordering.
    /// ORDER and CREDIT_NOTE entries build their full payloads (OrderDetailSync, CreditNoteSync).
    /// ANULACION and UPGRADE entries sync as ChainEntry metadata only (data already on the order via re-sync).
    /// BREAK entries sync the break marker to cloud.
    ///
    /// On build failure: inserts a BREAK chain_entry + marks the failed entry as processed.
    /// This preserves chain continuity (cloud sees explicit breaks) without blocking sync.
    async fn sync_chain_entries_http(&mut self) -> Result<(), crate::utils::AppError> {
        let binding = self.get_binding().await?;

        loop {
            let entries = chain_entry::list_unsynced(&self.state.pool, ARCHIVED_ORDER_BATCH_SIZE)
                .await
                .map_err(|e| {
                    crate::utils::AppError::internal(format!("List unsynced chain entries: {e}"))
                })?;

            if entries.is_empty() {
                break;
            }

            let mut items: Vec<CloudSyncItem> = Vec::with_capacity(entries.len());
            let mut synced_entry_ids: Vec<i64> = Vec::with_capacity(entries.len());

            for entry in &entries {
                // Every chain entry is synced as ChainEntry metadata
                let ce_sync = shared::cloud::ChainEntrySync {
                    id: entry.id,
                    entry_type: entry.entry_type.clone(),
                    entry_pk: entry.entry_pk,
                    prev_hash: entry.prev_hash.clone(),
                    curr_hash: entry.curr_hash.clone(),
                    created_at: entry.created_at,
                };
                match serde_json::to_value(&ce_sync) {
                    Ok(ce_data) => {
                        items.push(CloudSyncItem {
                            resource: SyncResource::ChainEntry,
                            version: entry.id as u64,
                            action: shared::cloud::SyncAction::Upsert,
                            resource_id: entry.id,
                            data: ce_data,
                        });
                    }
                    Err(e) => {
                        tracing::warn!(
                            chain_entry_id = entry.id,
                            entry_type = %entry.entry_type,
                            "Failed to serialize chain_entry for cloud sync, skipping: {e}"
                        );
                    }
                }

                // Build resource-specific payload for types that carry data
                match entry.entry_type.as_str() {
                    "ORDER" => match self.build_order_sync_item(entry).await {
                        Ok(item) => {
                            items.push(item);
                            synced_entry_ids.push(entry.id);
                        }
                        Err(e) => {
                            tracing::error!(
                                chain_entry_id = entry.id,
                                entry_pk = entry.entry_pk,
                                "Failed to build OrderDetailSync: {e}"
                            );
                            self.handle_chain_break(entry, &e.to_string()).await;
                            synced_entry_ids.push(entry.id);
                        }
                    },
                    "CREDIT_NOTE" => match self.build_credit_note_sync_item(entry).await {
                        Ok(item) => {
                            items.push(item);
                            synced_entry_ids.push(entry.id);
                        }
                        Err(e) => {
                            tracing::error!(
                                chain_entry_id = entry.id,
                                entry_pk = entry.entry_pk,
                                "Failed to build CreditNoteSync: {e}"
                            );
                            self.handle_chain_break(entry, &e.to_string()).await;
                            synced_entry_ids.push(entry.id);
                        }
                    },
                    // ANULACION/UPGRADE: re-send full order payload (with updated
                    // is_voided/is_upgraded/customer fields) so cloud can later
                    // generate Verifactu invoices (baja / F3 sustitutiva).
                    "ANULACION" | "UPGRADE" => match self.build_order_sync_item(entry).await {
                        Ok(item) => {
                            items.push(item);
                            synced_entry_ids.push(entry.id);
                        }
                        Err(e) => {
                            tracing::error!(
                                chain_entry_id = entry.id,
                                entry_type = %entry.entry_type,
                                entry_pk = entry.entry_pk,
                                "Failed to build OrderDetailSync for {}: {e}",
                                entry.entry_type
                            );
                            self.handle_chain_break(entry, &e.to_string()).await;
                            synced_entry_ids.push(entry.id);
                        }
                    },
                    // BREAK — chain entry metadata is sufficient
                    _ => {
                        synced_entry_ids.push(entry.id);
                    }
                }
            }

            if items.is_empty() {
                // All entries were metadata-only, just mark synced
                if let Err(e) = chain_entry::mark_synced(&self.state.pool, &synced_entry_ids).await
                {
                    tracing::error!("Failed to mark chain entries as synced: {e}");
                    break;
                }
                if entries.len() < ARCHIVED_ORDER_BATCH_SIZE as usize {
                    break;
                }
                continue;
            }

            let batch_count = items.len();
            let batch = CloudSyncBatch {
                edge_id: self.cloud_service.edge_id().to_string(),
                items,
                sent_at: shared::util::now_millis(),
            };

            let response = self
                .cloud_service
                .push_batch(batch, &binding)
                .await
                .map_err(|e| {
                    crate::utils::AppError::internal(format!("HTTP sync chain entries: {e}"))
                })?;

            if response.rejected > 0 {
                let mut has_real_errors = false;
                for err in &response.errors {
                    if err.message.contains("duplicate key") {
                        tracing::info!(
                            resource_id = %err.resource_id,
                            "Already exists in cloud (duplicate key)"
                        );
                    } else {
                        has_real_errors = true;
                        tracing::warn!(
                            resource_id = %err.resource_id,
                            "Rejected: {}", err.message
                        );
                    }
                }
                if has_real_errors {
                    // Still mark successfully-synced entries before stopping,
                    // so we don't re-send them in the next cycle (infinite retry loop).
                    if !synced_entry_ids.is_empty() {
                        if let Err(e) =
                            chain_entry::mark_synced(&self.state.pool, &synced_entry_ids).await
                        {
                            tracing::error!("Failed to mark accepted chain entries as synced: {e}");
                        }
                        self.mark_resource_tables_synced(&entries, &synced_entry_ids)
                            .await;
                    }
                    tracing::warn!(
                        accepted = response.accepted,
                        rejected = response.rejected,
                        "Chain entry sync has real rejections, stopping catch-up"
                    );
                    break;
                }
            }

            // Mark chain entries as synced + also mark resource tables for consistency
            if let Err(e) = chain_entry::mark_synced(&self.state.pool, &synced_entry_ids).await {
                tracing::error!("Failed to mark chain entries as synced: {e}");
                break;
            }
            // Keep resource-level cloud_synced in sync for backward compatibility
            self.mark_resource_tables_synced(&entries, &synced_entry_ids)
                .await;

            tracing::info!(
                batch_size = batch_count,
                accepted = response.accepted,
                "Chain entries synced via HTTP"
            );

            if entries.len() < ARCHIVED_ORDER_BATCH_SIZE as usize {
                break;
            }
        }

        Ok(())
    }

    /// Build an ArchivedOrder CloudSyncItem from a chain entry
    async fn build_order_sync_item(
        &self,
        entry: &chain_entry::ChainEntryRow,
    ) -> Result<CloudSyncItem, crate::utils::AppError> {
        let detail_sync = order::build_order_detail_sync(&self.state.pool, entry.entry_pk)
            .await
            .map_err(|e| {
                crate::utils::AppError::internal(format!("build_order_detail_sync: {e}"))
            })?;
        let data = serde_json::to_value(&detail_sync).map_err(|e| {
            crate::utils::AppError::internal(format!("serialize OrderDetailSync: {e}"))
        })?;
        Ok(CloudSyncItem {
            resource: SyncResource::ArchivedOrder,
            version: entry.entry_pk as u64,
            action: shared::cloud::SyncAction::Upsert,
            resource_id: entry.entry_pk,
            data,
        })
    }

    /// Build a CreditNote CloudSyncItem from a chain entry
    async fn build_credit_note_sync_item(
        &self,
        entry: &chain_entry::ChainEntryRow,
    ) -> Result<CloudSyncItem, crate::utils::AppError> {
        let cn_sync = credit_note::build_sync(&self.state.pool, entry.entry_pk)
            .await
            .map_err(|e| {
                crate::utils::AppError::internal(format!("build_credit_note_sync: {e}"))
            })?;
        let data = serde_json::to_value(&cn_sync).map_err(|e| {
            crate::utils::AppError::internal(format!("serialize CreditNoteSync: {e}"))
        })?;
        Ok(CloudSyncItem {
            resource: SyncResource::CreditNote,
            version: entry.entry_pk as u64,
            action: shared::cloud::SyncAction::Upsert,
            resource_id: entry.entry_pk,
            data,
        })
    }

    /// Insert a BREAK chain_entry when a resource fails to build for sync.
    /// Must acquire hash_chain_lock to prevent TOCTOU races with concurrent archiving.
    async fn handle_chain_break(&self, failed_entry: &chain_entry::ChainEntryRow, reason: &str) {
        // Acquire the shared hash chain lock to serialize with archive/credit_note writers
        let hash_lock = match self.state.orders_manager.archive_service() {
            Some(svc) => svc.hash_chain_lock().clone(),
            None => {
                tracing::error!(
                    chain_entry_id = failed_entry.id,
                    "Cannot insert BREAK: no archive_service (hash_chain_lock unavailable)"
                );
                return;
            }
        };
        let _lock = hash_lock.lock().await;

        let break_id = shared::util::snowflake_id();
        let now = shared::util::now_millis();

        // Use a transaction to atomically insert BREAK + update last_chain_hash
        let mut tx = match sqlx::pool::Pool::begin(&self.state.pool).await {
            Ok(tx) => tx,
            Err(e) => {
                tracing::error!(
                    chain_entry_id = failed_entry.id,
                    "Failed to begin transaction for BREAK: {e}"
                );
                return;
            }
        };

        // INSERT OR IGNORE: prevent duplicate BREAK for the same failed entry
        let result = sqlx::query(
            "INSERT OR IGNORE INTO chain_entry (id, entry_type, entry_pk, prev_hash, curr_hash, created_at, cloud_synced) \
             VALUES (?1, 'BREAK', ?2, ?3, 'CHAIN_BREAK', ?4, 0)",
        )
        .bind(break_id)
        .bind(failed_entry.id)
        .bind(&failed_entry.curr_hash)
        .bind(now)
        .execute(&mut *tx)
        .await;

        match result {
            Ok(r) if r.rows_affected() > 0 => {
                // Update system_state.last_chain_hash so the next real entry links correctly
                if let Err(e) = sqlx::query(
                    "UPDATE system_state SET last_chain_hash = 'CHAIN_BREAK', updated_at = ?1 WHERE id = ?2",
                )
                .bind(now)
                .bind(1_i64)
                .execute(&mut *tx)
                .await
                {
                    tracing::error!(
                        chain_entry_id = failed_entry.id,
                        "Failed to update last_chain_hash for BREAK: {e}"
                    );
                    if let Err(rb_err) = tx.rollback().await {
                        tracing::warn!(chain_entry_id = failed_entry.id, "Rollback failed: {rb_err}");
                    }
                    return;
                }

                if let Err(e) = tx.commit().await {
                    tracing::error!(
                        chain_entry_id = failed_entry.id,
                        "Failed to commit BREAK transaction: {e}"
                    );
                    return;
                }

                tracing::error!(
                    chain_entry_id = failed_entry.id,
                    entry_type = %failed_entry.entry_type,
                    entry_pk = failed_entry.entry_pk,
                    break_id,
                    reason,
                    "CHAIN BREAK: inserted break marker"
                );
            }
            Ok(_) => {
                // rows_affected == 0: BREAK already exists for this entry (INSERT OR IGNORE)
                if let Err(rb_err) = tx.rollback().await {
                    tracing::warn!(
                        chain_entry_id = failed_entry.id,
                        "Rollback failed: {rb_err}"
                    );
                }
                tracing::warn!(
                    chain_entry_id = failed_entry.id,
                    "BREAK already exists for this chain entry, skipping"
                );
            }
            Err(e) => {
                if let Err(rb_err) = tx.rollback().await {
                    tracing::warn!(
                        chain_entry_id = failed_entry.id,
                        "Rollback failed: {rb_err}"
                    );
                }
                tracing::error!(
                    chain_entry_id = failed_entry.id,
                    "Failed to insert BREAK chain_entry: {e}"
                );
            }
        }
    }

    /// Keep resource-level cloud_synced flags in sync with chain_entry.cloud_synced
    async fn mark_resource_tables_synced(
        &self,
        entries: &[chain_entry::ChainEntryRow],
        synced_ids: &[i64],
    ) {
        let mut order_ids = Vec::new();
        let mut cn_ids = Vec::new();
        for entry in entries {
            if synced_ids.contains(&entry.id) {
                match entry.entry_type.as_str() {
                    "ORDER" | "ANULACION" | "UPGRADE" => order_ids.push(entry.entry_pk),
                    "CREDIT_NOTE" => cn_ids.push(entry.entry_pk),
                    _ => {}
                }
            }
        }
        if !order_ids.is_empty()
            && let Err(e) = order::mark_cloud_synced(&self.state.pool, &order_ids).await
        {
            tracing::warn!("Failed to mark archived_order.cloud_synced: {e}");
        }
        if !cn_ids.is_empty()
            && let Err(e) = credit_note::mark_synced_batch(&self.state.pool, &cn_ids).await
        {
            tracing::warn!("Failed to mark credit_note.cloud_synced: {e}");
        }
    }

    /// Sync unsynced Verifactu invoices (F2/R5) to cloud via HTTP batch.
    async fn sync_invoices_http(&mut self) -> Result<(), crate::utils::AppError> {
        let binding = self.get_binding().await?;

        loop {
            let ids = invoice::list_unsynced_ids(&self.state.pool, ARCHIVED_ORDER_BATCH_SIZE)
                .await
                .map_err(|e| {
                    crate::utils::AppError::internal(format!("List unsynced invoices: {e}"))
                })?;

            if ids.is_empty() {
                break;
            }

            let mut items: Vec<CloudSyncItem> = Vec::with_capacity(ids.len());
            let mut synced_ids: Vec<i64> = Vec::with_capacity(ids.len());
            let mut skipped_ids: Vec<i64> = Vec::new();

            for &id in &ids {
                match invoice::build_sync(&self.state.pool, id).await {
                    Ok(inv_sync) => {
                        let data = match serde_json::to_value(&inv_sync) {
                            Ok(v) => v,
                            Err(e) => {
                                tracing::error!(
                                    invoice_id = id,
                                    "Failed to serialize InvoiceSync, skipping: {e}"
                                );
                                skipped_ids.push(id);
                                continue;
                            }
                        };
                        items.push(CloudSyncItem {
                            resource: SyncResource::Invoice,
                            version: id as u64,
                            action: shared::cloud::SyncAction::Upsert,
                            resource_id: id,
                            data,
                        });
                        synced_ids.push(id);
                    }
                    Err(e) => {
                        tracing::error!(
                            invoice_id = id,
                            "Failed to build InvoiceSync, skipping: {e}"
                        );
                        skipped_ids.push(id);
                    }
                }
            }

            // Mark permanently failed invoices as synced to unblock the queue
            if !skipped_ids.is_empty() {
                tracing::warn!(
                    count = skipped_ids.len(),
                    ids = ?skipped_ids,
                    "Skipped unbuildable invoices, marking as synced"
                );
                if let Err(e) = invoice::mark_synced(&self.state.pool, &skipped_ids).await {
                    tracing::error!("Failed to mark skipped invoices as synced: {e}");
                }
            }

            if items.is_empty() {
                if skipped_ids.is_empty() {
                    break;
                }
                continue;
            }

            let batch_count = items.len();
            let batch = CloudSyncBatch {
                edge_id: self.cloud_service.edge_id().to_string(),
                items,
                sent_at: shared::util::now_millis(),
            };

            let response = self
                .cloud_service
                .push_batch(batch, &binding)
                .await
                .map_err(|e| {
                    crate::utils::AppError::internal(format!("HTTP sync invoices: {e}"))
                })?;

            if response.rejected > 0 {
                let mut dup_db_ids: Vec<i64> = Vec::new();
                let mut real_errors = Vec::new();
                for err in &response.errors {
                    if err.message.contains("duplicate key") {
                        if let Some(&db_id) = synced_ids.get(err.index as usize) {
                            dup_db_ids.push(db_id);
                        }
                        tracing::info!(
                            resource_id = %err.resource_id,
                            "Invoice already exists in cloud, marking as synced"
                        );
                    } else {
                        real_errors.push(err);
                        tracing::warn!(
                            resource_id = %err.resource_id,
                            "Invoice rejected: {}", err.message
                        );
                    }
                }

                if !dup_db_ids.is_empty()
                    && let Err(e) = invoice::mark_synced(&self.state.pool, &dup_db_ids).await
                {
                    tracing::error!("Failed to mark duplicate invoices as synced: {e}");
                }

                if !real_errors.is_empty() {
                    // Mark permanently rejected invoices (e.g. huella mismatch) as synced
                    // to unblock the queue — these can't be fixed by retrying
                    let rejected_db_ids: Vec<i64> = real_errors
                        .iter()
                        .filter_map(|err| synced_ids.get(err.index as usize).copied())
                        .collect();
                    if !rejected_db_ids.is_empty() {
                        tracing::error!(
                            count = rejected_db_ids.len(),
                            ids = ?rejected_db_ids,
                            errors = ?real_errors.iter().map(|e| &e.message).collect::<Vec<_>>(),
                            "Invoices permanently rejected by cloud, marking as synced to unblock queue"
                        );
                        if let Err(e) =
                            invoice::mark_synced(&self.state.pool, &rejected_db_ids).await
                        {
                            tracing::error!("Failed to mark rejected invoices as synced: {e}");
                        }
                    }
                }
            }

            if let Err(e) = invoice::mark_synced(&self.state.pool, &synced_ids).await {
                tracing::error!("Failed to mark invoices as cloud_synced, stopping catch-up: {e}");
                break;
            }

            tracing::info!(
                batch_size = batch_count,
                accepted = response.accepted,
                "Invoices synced and confirmed via HTTP"
            );

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
        pending: &mut HashMap<SyncResource, HashMap<i64, CloudSyncItem>>,
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
                match serde_json::to_value(record) {
                    Ok(data) => {
                        items.push(CloudSyncItem {
                            resource,
                            version,
                            action: shared::cloud::SyncAction::Upsert,
                            resource_id: id_fn(record),
                            data,
                        });
                    }
                    Err(e) => {
                        tracing::warn!(
                            resource = %resource,
                            id = id_fn(record),
                            "Failed to serialize record for sync: {e}"
                        );
                    }
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
            SyncResource::Tag => match tag::find_all(&self.state.pool).await {
                Ok(v) => push_many(&v, resource, version, |t| t.id, items),
                Err(e) => tracing::warn!(resource = %resource, "Failed to collect for sync: {e}"),
            },
            SyncResource::Attribute => match attribute::find_all(&self.state.pool).await {
                Ok(v) => push_many(&v, resource, version, |a| a.id, items),
                Err(e) => tracing::warn!(resource = %resource, "Failed to collect for sync: {e}"),
            },
            SyncResource::AttributeBinding => {
                match sqlx::query_as::<_, shared::models::attribute::AttributeBinding>(
                    "SELECT id, owner_type, owner_id, attribute_id, is_required, display_order, \
                     COALESCE(default_option_ids, 'null') as default_option_ids \
                     FROM attribute_binding ORDER BY display_order",
                )
                .fetch_all(&self.state.pool)
                .await
                {
                    Ok(v) => push_many(&v, resource, version, |b| b.id, items),
                    Err(e) => {
                        tracing::warn!(resource = %resource, "Failed to collect for sync: {e}")
                    }
                }
            }
            SyncResource::Zone => match zone::find_all(&self.state.pool).await {
                Ok(v) => push_many(&v, resource, version, |z| z.id, items),
                Err(e) => tracing::warn!(resource = %resource, "Failed to collect for sync: {e}"),
            },
            SyncResource::DiningTable => match dining_table::find_all(&self.state.pool).await {
                Ok(v) => push_many(&v, resource, version, |t| t.id, items),
                Err(e) => tracing::warn!(resource = %resource, "Failed to collect for sync: {e}"),
            },
            SyncResource::Employee => {
                match employee::find_all_with_inactive(&self.state.pool).await {
                    Ok(v) => push_many(&v, resource, version, |e| e.id, items),
                    Err(e) => {
                        tracing::warn!(resource = %resource, "Failed to collect for sync: {e}")
                    }
                }
            }
            SyncResource::PriceRule => match price_rule::find_all(&self.state.pool).await {
                Ok(v) => push_many(&v, resource, version, |r| r.id, items),
                Err(e) => tracing::warn!(resource = %resource, "Failed to collect for sync: {e}"),
            },
            SyncResource::LabelTemplate => match label_template::list_all(&self.state.pool).await {
                Ok(v) => push_many(&v, resource, version, |t| t.id, items),
                Err(e) => tracing::warn!(resource = %resource, "Failed to collect for sync: {e}"),
            },
            SyncResource::StoreInfo => {
                match store_info::get(&self.state.pool).await {
                    Ok(Some(info)) => match serde_json::to_value(&info) {
                        Ok(data) => {
                            items.push(CloudSyncItem {
                                resource,
                                version,
                                action: shared::cloud::SyncAction::Upsert,
                                resource_id: 0,
                                data,
                            });
                        }
                        Err(e) => {
                            tracing::warn!(resource = %resource, "Failed to serialize StoreInfo: {e}")
                        }
                    },
                    Ok(None) => {} // no store info configured yet
                    Err(e) => {
                        tracing::warn!(resource = %resource, "Failed to collect for sync: {e}")
                    }
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
            let events = match self
                .state
                .orders_manager
                .get_events_for_order(order.order_id)
            {
                Ok(events) => events,
                Err(e) => {
                    tracing::warn!(order_id = %order.order_id, "Failed to get events for active order push: {e}");
                    Vec::new()
                }
            };
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

        let payload: SyncPayload = match serde_json::from_slice(&msg.payload) {
            Ok(p) => p,
            Err(_) => return None, // non-sync payloads are normal
        };
        if payload.resource != SyncResource::OrderSync {
            return None;
        }

        let order_id = payload.id;

        // deleted = 订单终结 (completed/voided/merged)
        if payload.action == SyncChangeType::Deleted {
            return Some(CloudMessage::ActiveOrderRemoved { order_id });
        }

        // created/updated = 活跃订单变更，读取最新快照 + 事件历史
        match self.state.orders_manager.get_snapshot(order_id) {
            Ok(Some(snap)) if snap.is_active() => {
                let events = match self.state.orders_manager.get_events_for_order(order_id) {
                    Ok(events) => events,
                    Err(e) => {
                        tracing::warn!(order_id, "Failed to get events for order push: {e}");
                        Vec::new()
                    }
                };
                Some(CloudMessage::ActiveOrderSnapshot {
                    snapshot: Box::new(snap),
                    events,
                })
            }
            Ok(Some(_)) => {
                // 非活跃状态（刚完成/作废），发送移除通知
                Some(CloudMessage::ActiveOrderRemoved { order_id })
            }
            Ok(None) => {
                // 快照不存在（已被清理），发送移除通知
                Some(CloudMessage::ActiveOrderRemoved { order_id })
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
