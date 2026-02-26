//! WebSocket handler for edge-server duplex communication
//!
//! Replaces the HTTP POST sync with a persistent WebSocket connection.
//! Commands are pushed to edge in real-time, sync batches processed as they arrive.

use axum::Extension;
use axum::extract::State;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::response::IntoResponse;
use futures::{SinkExt, StreamExt};
use shared::cloud::CloudMessage;
use shared::error::{AppError, ErrorCode};
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::Instant;

use crate::auth::EdgeIdentity;
use crate::db::{audit, sync_store};
use crate::state::AppState;

/// Server-side ping interval (seconds). Cloud proactively pings edge to detect dead connections.
const PING_INTERVAL_SECS: u64 = 30;

/// If no activity (any incoming frame) is received within this duration, the connection is
/// considered dead and will be closed. Set to 3× ping interval so that the edge (which also
/// sends pings every 30s) has ample time to respond even under transient network hiccups.
const HEARTBEAT_TIMEOUT_SECS: u64 = 90;

/// GET /api/edge/ws — upgrade to WebSocket
pub async fn handle_edge_ws(
    State(state): State<AppState>,
    Extension(identity): Extension<EdgeIdentity>,
    ws: WebSocketUpgrade,
) -> Result<impl IntoResponse, AppError> {
    // Only Server entities can use WebSocket
    if identity.entity_type != shared::activation::EntityType::Server {
        return Err(AppError::with_message(
            ErrorCode::PermissionDenied,
            "Only server entities can use WebSocket sync",
        ));
    }

    Ok(ws.on_upgrade(move |socket| handle_ws_connection(socket, state, identity)))
}

async fn handle_ws_connection(socket: WebSocket, state: AppState, identity: EdgeIdentity) {
    let now = shared::util::now_millis();

    // Auto-register edge-server
    let store_id = match sync_store::ensure_store(
        &state.pool,
        &identity.entity_id,
        &identity.tenant_id,
        &identity.device_id,
        now,
    )
    .await
    {
        Ok(id) => id,
        Err(e) => {
            tracing::error!("Failed to register edge-server for WS: {e}");
            return;
        }
    };

    tracing::info!(
        edge_id = %identity.entity_id,
        tenant_id = %identity.tenant_id,
        store_id,
        "WebSocket connected"
    );

    // Audit: edge connected
    let connect_detail = serde_json::json!({
        "edge_id": identity.entity_id,
        "store_id": store_id,
        "device_id": identity.device_id,
    });
    let _ = audit::log(
        &state.pool,
        &identity.tenant_id,
        "edge_connected",
        Some(&connect_detail),
        None,
        now,
    )
    .await;

    let (mut ws_sink, mut ws_stream) = socket.split();

    // Create message channel for real-time push (carries CloudMessage directly)
    let (msg_tx, mut msg_rx) = mpsc::channel::<CloudMessage>(32);

    // Register in connected_edges (replaces old connection if any — old sender drops → old loop exits)
    if let Some((_, old_tx)) = state.edges.connected.remove(&store_id) {
        tracing::warn!(store_id, "Replacing existing WS connection for this edge");
        drop(old_tx); // old msg_rx.recv() returns None → old loop breaks
    }
    state.edges.connected.insert(store_id, msg_tx.clone());

    // Send Welcome with sync cursors
    match sync_store::get_cursors(&state.pool, store_id).await {
        Ok(cursors) => {
            let welcome = CloudMessage::Welcome { cursors };
            match serde_json::to_string(&welcome) {
                Ok(json) => {
                    if ws_sink.send(Message::Text(json.into())).await.is_err() {
                        tracing::warn!(store_id, "Failed to send Welcome, disconnecting");
                        state.edges.connected.remove(&store_id);
                        return;
                    }
                }
                Err(e) => {
                    tracing::error!(store_id, "Failed to serialize Welcome: {e}");
                }
            }
        }
        Err(e) => {
            tracing::error!(store_id, "Failed to get cursors for Welcome: {e}");
            // Non-fatal: edge will do full sync if no Welcome received
        }
    }

    // Replay pending ops (Console edits queued while edge was offline)
    // Fetch ordered, send one-by-one, delete after each successful send.
    // On send failure: unsent ops remain in DB for next reconnect.
    match crate::db::store::pending_ops::fetch_ordered(&state.pool, store_id).await {
        Ok(ops) if !ops.is_empty() => {
            let total = ops.len();
            let mut sent = 0usize;
            for (row_id, op, changed_at) in ops {
                // For product ops with images, send EnsureImage with fresh presigned URL
                if let Some(hash) = op.image_hash()
                    && let Ok(presigned_url) =
                        super::image::presigned_get_url(&state, &identity.tenant_id, hash).await
                {
                    let ensure_msg = CloudMessage::Rpc {
                        id: format!("img-{hash}"),
                        payload: Box::new(shared::cloud::CloudRpc::StoreOp {
                            op: Box::new(shared::cloud::store_op::StoreOp::EnsureImage {
                                presigned_url,
                                hash: hash.to_string(),
                            }),
                            changed_at: None,
                        }),
                    };
                    if let Ok(json) = serde_json::to_string(&ensure_msg) {
                        let _ = ws_sink.send(Message::Text(json.into())).await;
                    }
                }

                let msg = CloudMessage::Rpc {
                    id: format!("catchup-{}", uuid::Uuid::new_v4()),
                    payload: Box::new(shared::cloud::CloudRpc::StoreOp {
                        op: Box::new(op),
                        changed_at: Some(changed_at),
                    }),
                };
                let json = match serde_json::to_string(&msg) {
                    Ok(j) => j,
                    Err(e) => {
                        tracing::warn!(store_id, row_id, "Failed to serialize pending op: {e}");
                        continue;
                    }
                };
                if ws_sink.send(Message::Text(json.into())).await.is_err() {
                    tracing::warn!(
                        store_id,
                        sent,
                        remaining = total - sent,
                        "Failed to send pending op, disconnecting (unsent ops preserved)"
                    );
                    state.edges.connected.remove(&store_id);
                    return;
                }
                // Successfully sent — delete from queue
                if let Err(e) = crate::db::store::pending_ops::delete_one(&state.pool, row_id).await
                {
                    tracing::warn!(store_id, row_id, "Failed to delete sent pending op: {e}");
                }
                sent += 1;
            }
            tracing::info!(store_id, count = sent, "Pending ops replayed to edge");
        }
        Ok(_) => {} // no pending ops
        Err(e) => {
            tracing::error!(store_id, "Failed to fetch pending ops: {e}");
        }
    }

    // 所有初始化发送完成后，标记 edge 上线（通知正在观看的 console）
    state
        .live_orders
        .mark_edge_online(&identity.tenant_id, store_id);

    // Server-side heartbeat: ping edge and detect dead connections
    let mut ping_interval = tokio::time::interval(Duration::from_secs(PING_INTERVAL_SECS));
    ping_interval.tick().await; // skip immediate first tick
    let mut last_activity = Instant::now();
    let heartbeat_timeout = Duration::from_secs(HEARTBEAT_TIMEOUT_SECS);

    // Main select loop
    loop {
        tokio::select! {
            // Server-side ping + heartbeat timeout check
            _ = ping_interval.tick() => {
                if last_activity.elapsed() > heartbeat_timeout {
                    tracing::warn!(
                        edge_id = %identity.entity_id,
                        elapsed_secs = last_activity.elapsed().as_secs(),
                        "Edge heartbeat timeout, disconnecting"
                    );
                    break;
                }
                if ws_sink.send(Message::Ping(vec![].into())).await.is_err() {
                    tracing::warn!(
                        edge_id = %identity.entity_id,
                        "Failed to send ping, disconnecting"
                    );
                    break;
                }
            }

            // Incoming message from edge
            msg = ws_stream.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        last_activity = Instant::now();
                        handle_edge_message(
                            &text,
                            &state,
                            &identity,
                            store_id,
                            &mut ws_sink,
                        )
                        .await;
                    }
                    Some(Ok(Message::Ping(data))) => {
                        last_activity = Instant::now();
                        let _ = ws_sink.send(Message::Pong(data)).await;
                    }
                    Some(Ok(Message::Pong(_))) => {
                        last_activity = Instant::now();
                    }
                    Some(Ok(Message::Close(_))) | None => {
                        tracing::info!(
                            edge_id = %identity.entity_id,
                            "WebSocket disconnected"
                        );
                        break;
                    }
                    Some(Err(e)) => {
                        tracing::warn!(
                            edge_id = %identity.entity_id,
                            "WebSocket error: {e}"
                        );
                        break;
                    }
                    _ => {}
                }
            }

            // Message to push to edge (Command or Rpc)
            msg = msg_rx.recv() => {
                match msg {
                    Some(cloud_msg) => {
                        match serde_json::to_string(&cloud_msg) {
                            Ok(json) => {
                                if ws_sink.send(Message::Text(json.into())).await.is_err() {
                                    tracing::warn!(store_id, "Failed to push message via WS");
                                    break;
                                }
                            }
                            Err(e) => {
                                tracing::error!(store_id, "Failed to serialize push message: {e}");
                            }
                        }
                    }
                    None => {
                        tracing::info!(store_id, "Push channel closed (connection replaced), exiting");
                        break;
                    }
                }
            }
        }
    }

    // Send Close frame (best-effort)
    let _ = ws_sink.close().await;

    // Cleanup: remove from connected edges
    state.edges.connected.remove(&store_id);

    // 通知 console 订阅者 edge 已离线
    state.live_orders.clear_edge(&identity.tenant_id, store_id);

    // Audit: edge disconnected
    let disconnect_now = shared::util::now_millis();
    let disconnect_detail = serde_json::json!({
        "edge_id": identity.entity_id,
        "store_id": store_id,
    });
    let _ = audit::log(
        &state.pool,
        &identity.tenant_id,
        "edge_disconnected",
        Some(&disconnect_detail),
        None,
        disconnect_now,
    )
    .await;

    tracing::info!(
        edge_id = %identity.entity_id,
        "WebSocket session cleaned up"
    );
}

async fn handle_edge_message<S>(
    text: &str,
    state: &AppState,
    identity: &EdgeIdentity,
    store_id: i64,
    ws_sink: &mut S,
) where
    S: futures::Sink<Message, Error = axum::Error> + Unpin,
{
    let cloud_msg: CloudMessage = match serde_json::from_str(text) {
        Ok(m) => m,
        Err(e) => {
            tracing::warn!(
                edge_id = %identity.entity_id,
                "Invalid CloudMessage: {e}"
            );
            return;
        }
    };

    let now = shared::util::now_millis();

    match cloud_msg {
        CloudMessage::SyncBatch { items, .. } => {
            // Reject oversized batches
            if items.len() > shared::cloud::MAX_SYNC_BATCH_ITEMS {
                tracing::warn!(
                    edge_id = %identity.entity_id,
                    count = items.len(),
                    "WS sync batch too large, ignoring"
                );
                return;
            }

            // Update last_sync_at
            if let Err(e) = sync_store::update_last_sync(&state.pool, store_id, now).await {
                tracing::warn!(store_id, "Failed to update last_sync_at: {e}");
            }

            let mut accepted = 0u32;
            let mut rejected = 0u32;
            let mut errors = Vec::new();
            // Track max version per resource type for batch cursor update
            let mut cursor_maxes: std::collections::HashMap<&str, i64> =
                std::collections::HashMap::new();

            for (idx, item) in items.iter().enumerate() {
                match sync_store::upsert_resource(
                    &state.pool,
                    store_id,
                    &identity.tenant_id,
                    item,
                    now,
                )
                .await
                {
                    Ok(()) => {
                        accepted += 1;
                        let version = i64::try_from(item.version).unwrap_or(i64::MAX);
                        let entry = cursor_maxes
                            .entry(item.resource.as_str())
                            .or_insert(version);
                        if version > *entry {
                            *entry = version;
                        }
                    }
                    Err(e) => {
                        rejected += 1;
                        errors.push(shared::cloud::CloudSyncError {
                            index: u32::try_from(idx).unwrap_or(u32::MAX),
                            resource_id: item.resource_id.clone(),
                            message: e.to_string(),
                        });
                    }
                }
            }

            // Batch cursor update (1 query for all resource types instead of N)
            if !cursor_maxes.is_empty() {
                let cursor_pairs: Vec<(&str, i64)> = cursor_maxes.into_iter().collect();
                if let Err(e) =
                    sync_store::update_cursors_batch(&state.pool, store_id, &cursor_pairs, now)
                        .await
                {
                    tracing::warn!(store_id, "Failed to batch update sync cursors: {e}");
                }
            }

            // Send SyncAck
            let ack = CloudMessage::SyncAck {
                accepted,
                rejected,
                errors,
            };
            match serde_json::to_string(&ack) {
                Ok(json) => {
                    if let Err(e) = ws_sink.send(Message::Text(json.into())).await {
                        tracing::warn!(store_id, "Failed to send SyncAck: {e}");
                    }
                }
                Err(e) => {
                    tracing::error!(store_id, "Failed to serialize SyncAck: {e}");
                }
            }

            tracing::info!(
                edge_id = %identity.entity_id,
                accepted,
                rejected,
                "WS sync batch processed"
            );
        }

        CloudMessage::RpcResult { id, result } => {
            if let Some((_, (_, sender))) = state.edges.pending_rpcs.remove(&id) {
                let _ = sender.send(result);
            } else {
                tracing::warn!(rpc_id = %id, "RpcResult for unknown or expired request");
            }
        }

        CloudMessage::ActiveOrderSnapshot { snapshot, events } => {
            let live_snapshot = shared::console::LiveOrderSnapshot {
                store_id,
                order: *snapshot,
                events,
            };
            state
                .live_orders
                .publish_update(&identity.tenant_id, live_snapshot);
        }

        CloudMessage::ActiveOrderRemoved { order_id } => {
            state
                .live_orders
                .publish_remove(&identity.tenant_id, &order_id, store_id);
        }

        _ => {
            tracing::debug!("Ignoring unexpected CloudMessage from edge");
        }
    }
}
