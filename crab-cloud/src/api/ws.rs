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
use tokio::sync::mpsc;

use crate::auth::EdgeIdentity;
use crate::db::{audit, commands, sync_store};
use crate::state::AppState;

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
    let edge_server_id = match sync_store::ensure_edge_server(
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
        edge_server_id,
        "WebSocket connected"
    );

    // Audit: edge connected
    let connect_detail = serde_json::json!({
        "edge_id": identity.entity_id,
        "edge_server_id": edge_server_id,
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

    // Register in connected_edges
    state.connected_edges.insert(edge_server_id, msg_tx.clone());

    // Send Welcome with sync cursors
    match sync_store::get_cursors(&state.pool, edge_server_id).await {
        Ok(cursors) => {
            let welcome = CloudMessage::Welcome { cursors };
            if let Ok(json) = serde_json::to_string(&welcome)
                && ws_sink.send(Message::Text(json.into())).await.is_err()
            {
                tracing::warn!(edge_server_id, "Failed to send Welcome, disconnecting");
                state.connected_edges.remove(&edge_server_id);
                return;
            }
        }
        Err(e) => {
            tracing::error!(edge_server_id, "Failed to get cursors for Welcome: {e}");
            // Non-fatal: edge will do full sync if no Welcome received
        }
    }

    // Check if edge needs initial catalog provisioning
    // (no products synced yet = first-time activation)
    match needs_catalog_provisioning(&state, edge_server_id).await {
        Ok(true) => {
            tracing::info!(
                edge_server_id,
                "Edge needs catalog provisioning, sending FullSync"
            );
            let snapshot = crate::db::catalog_templates::default_snapshot();
            let rpc_msg = CloudMessage::Rpc {
                id: format!("provision-{edge_server_id}"),
                payload: Box::new(shared::cloud::ws::CloudRpc::CatalogOp(Box::new(
                    shared::cloud::catalog::CatalogOp::FullSync { snapshot },
                ))),
            };
            if let Ok(json) = serde_json::to_string(&rpc_msg)
                && ws_sink.send(Message::Text(json.into())).await.is_err()
            {
                tracing::warn!(edge_server_id, "Failed to send FullSync, disconnecting");
                state.connected_edges.remove(&edge_server_id);
                return;
            }
        }
        Ok(false) => {} // already provisioned
        Err(e) => {
            tracing::warn!(edge_server_id, "Failed to check provisioning status: {e}");
        }
    }

    // Send any pending commands immediately on connect
    if let Ok(pending) = commands::get_pending(&state.pool, edge_server_id, 10).await
        && !pending.is_empty()
    {
        let mut sent_ids: Vec<i64> = Vec::new();
        for cmd in pending {
            let cmd_id = cmd.id;
            let cloud_cmd = shared::cloud::CloudCommand {
                id: cmd.id.to_string(),
                command_type: cmd.command_type,
                payload: cmd.payload,
                created_at: cmd.created_at,
            };
            let msg = CloudMessage::Command(cloud_cmd);
            if let Ok(json) = serde_json::to_string(&msg)
                && ws_sink.send(Message::Text(json.into())).await.is_ok()
            {
                sent_ids.push(cmd_id);
            } else {
                break;
            }
        }
        if !sent_ids.is_empty() {
            let _ = commands::mark_delivered(&state.pool, &sent_ids).await;
        }
    }

    // 所有初始化发送完成后，标记 edge 上线（通知正在观看的 console）
    state
        .live_orders
        .mark_edge_online(&identity.tenant_id, edge_server_id);

    // Main select loop
    loop {
        tokio::select! {
            // Incoming message from edge
            msg = ws_stream.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        handle_edge_message(
                            &text,
                            &state,
                            &identity,
                            edge_server_id,
                            &mut ws_sink,
                        )
                        .await;
                    }
                    Some(Ok(Message::Ping(data))) => {
                        let _ = ws_sink.send(Message::Pong(data)).await;
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
                    _ => {} // Binary, Pong — ignore
                }
            }

            // Message to push to edge (Command or Rpc)
            msg = msg_rx.recv() => {
                match msg {
                    Some(cloud_msg) => {
                        if let Ok(json) = serde_json::to_string(&cloud_msg)
                            && ws_sink.send(Message::Text(json.into())).await.is_err() {
                                tracing::warn!("Failed to push message via WS");
                                break;
                            }
                    }
                    None => break, // channel closed
                }
            }
        }
    }

    // Send Close frame (best-effort)
    let _ = ws_sink.close().await;

    // Cleanup: remove from connected edges
    state.connected_edges.remove(&edge_server_id);

    // 通知 console 订阅者 edge 已离线
    state
        .live_orders
        .clear_edge(&identity.tenant_id, edge_server_id);

    // Rollback delivered→pending so commands can be re-sent on reconnect
    match commands::rollback_delivered(&state.pool, edge_server_id).await {
        Ok(n) if n > 0 => {
            tracing::info!(
                edge_id = %identity.entity_id,
                rolled_back = n,
                "Rolled back delivered commands to pending on disconnect"
            );
        }
        Err(e) => {
            tracing::warn!(
                edge_id = %identity.entity_id,
                "Failed to rollback delivered commands: {e}"
            );
        }
        _ => {}
    }

    // Audit: edge disconnected
    let disconnect_now = shared::util::now_millis();
    let disconnect_detail = serde_json::json!({
        "edge_id": identity.entity_id,
        "edge_server_id": edge_server_id,
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
    edge_server_id: i64,
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
        CloudMessage::SyncBatch {
            items,
            command_results,
            ..
        } => {
            // Update last_sync_at
            if let Err(e) = sync_store::update_last_sync(&state.pool, edge_server_id, now).await {
                tracing::warn!(edge_server_id, "Failed to update last_sync_at: {e}");
            }

            // Process command results
            if !command_results.is_empty()
                && let Err(e) =
                    commands::complete_commands(&state.pool, &command_results, now).await
            {
                tracing::warn!("Failed to process command results: {e}");
            }

            let mut accepted = 0u32;
            let mut rejected = 0u32;
            let mut errors = Vec::new();

            for (idx, item) in items.iter().enumerate() {
                match sync_store::upsert_resource(
                    &state.pool,
                    edge_server_id,
                    &identity.tenant_id,
                    item,
                    now,
                )
                .await
                {
                    Ok(()) => {
                        accepted += 1;
                        if let Err(e) = sync_store::update_cursor(
                            &state.pool,
                            edge_server_id,
                            &item.resource,
                            i64::try_from(item.version).unwrap_or(i64::MAX),
                            now,
                        )
                        .await
                        {
                            tracing::warn!(
                                resource = %item.resource,
                                "Failed to update sync cursor: {e}"
                            );
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

            // Send SyncAck
            let ack = CloudMessage::SyncAck {
                accepted,
                rejected,
                errors,
            };
            if let Ok(json) = serde_json::to_string(&ack) {
                let _ = ws_sink.send(Message::Text(json.into())).await;
            }

            tracing::info!(
                edge_id = %identity.entity_id,
                accepted,
                rejected,
                "WS sync batch processed"
            );
        }

        CloudMessage::CommandResult { results } => {
            // Separate ephemeral on-demand results from persistent command results
            let mut persistent_results = Vec::new();
            for result in results {
                if let Some((_, (_, sender))) = state.pending_requests.remove(&result.command_id) {
                    let _ = sender.send(result);
                } else {
                    persistent_results.push(result);
                }
            }

            if !persistent_results.is_empty() {
                if let Err(e) =
                    commands::complete_commands(&state.pool, &persistent_results, now).await
                {
                    tracing::warn!("Failed to process WS command results: {e}");
                } else {
                    // Audit each command result
                    for r in &persistent_results {
                        let cmd_detail = serde_json::json!({
                            "command_id": r.command_id,
                            "success": r.success,
                            "error": r.error,
                            "edge_server_id": edge_server_id,
                        });
                        let action = if r.success {
                            "command_completed"
                        } else {
                            "command_failed"
                        };
                        let _ = audit::log(
                            &state.pool,
                            &identity.tenant_id,
                            action,
                            Some(&cmd_detail),
                            None,
                            now,
                        )
                        .await;
                    }
                    tracing::info!(
                        count = persistent_results.len(),
                        "Processed command results from WS"
                    );
                }
            }
        }

        CloudMessage::RpcResult { id, result } => {
            if let Some((_, (_, sender))) = state.pending_rpcs.remove(&id) {
                let _ = sender.send(result);
            } else {
                tracing::warn!(rpc_id = %id, "RpcResult for unknown or expired request");
            }
        }

        CloudMessage::ActiveOrderSnapshot { snapshot, events } => {
            let live_snapshot = shared::console::LiveOrderSnapshot {
                edge_server_id,
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
                .publish_remove(&identity.tenant_id, &order_id, edge_server_id);
        }

        _ => {
            tracing::debug!("Ignoring unexpected CloudMessage from edge");
        }
    }
}

/// Check if an edge server needs initial catalog provisioning.
///
/// Returns true if no products have been synced for this edge yet.
async fn needs_catalog_provisioning(
    state: &AppState,
    edge_server_id: i64,
) -> Result<bool, sqlx::Error> {
    let count: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM catalog_products WHERE edge_server_id = $1")
            .bind(edge_server_id)
            .fetch_one(&state.pool)
            .await?;
    Ok(count.0 == 0)
}
