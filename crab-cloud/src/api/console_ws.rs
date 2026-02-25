//! Console WebSocket endpoint — 实时活跃订单推送
//!
//! GET /api/tenant/live-orders/ws?token=<JWT>
//! Auth: JWT 通过 query parameter 传递（浏览器 WebSocket 不支持自定义 headers）
//!
//! 协议:
//! - Cloud → Console: ConsoleMessage (Ready, OrderUpdated, OrderRemoved, EdgeStatus)
//! - Console → Cloud: ConsoleCommand (Subscribe)

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Query, State};
use axum::response::IntoResponse;
use futures::{SinkExt, StreamExt};
use serde::Deserialize;
use shared::console::{ConsoleCommand, ConsoleMessage};
use shared::error::{AppError, ErrorCode};
use std::collections::HashSet;
use tokio::sync::broadcast;
use tokio::time::Duration;

use crate::auth::tenant_auth;
use crate::live::LiveHubEvent;
use crate::state::AppState;

/// Maximum concurrent console WS connections per tenant
const MAX_CONSOLE_WS_PER_TENANT: usize = 10;

#[derive(Deserialize)]
pub struct WsAuthQuery {
    token: String,
}

/// GET /api/tenant/live-orders/ws?token=<JWT>
pub async fn handle_console_ws(
    State(state): State<AppState>,
    Query(query): Query<WsAuthQuery>,
    ws: WebSocketUpgrade,
) -> Result<impl IntoResponse, AppError> {
    // 手动验证 JWT（浏览器 WebSocket 不支持 Authorization header）
    let claims = tenant_auth::verify_token(&query.token, &state.jwt_secret).map_err(|e| {
        tracing::debug!("Console WS JWT validation failed: {e}");
        AppError::new(ErrorCode::TokenExpired)
    })?;

    let tenant_id = claims.sub;

    // Check concurrent connection limit (atomic increment to avoid TOCTOU race)
    {
        let counter = state
            .console_connections
            .entry(tenant_id.clone())
            .or_insert_with(|| std::sync::atomic::AtomicUsize::new(0));
        let prev = counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        if prev >= MAX_CONSOLE_WS_PER_TENANT {
            counter.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
            return Err(AppError::with_message(
                ErrorCode::ResourceLimitExceeded,
                format!("Too many console connections ({prev}/{MAX_CONSOLE_WS_PER_TENANT})"),
            ));
        }
    } // drop RefMut before moving state into on_upgrade closure

    Ok(ws.on_upgrade(move |socket| console_ws_session(socket, state, tenant_id)))
}

async fn console_ws_session(socket: WebSocket, state: AppState, tenant_id: String) {
    let (mut sink, mut stream) = socket.split();

    tracing::info!(tenant_id = %tenant_id, "Console WS connected");

    // Connection count already incremented in handler (atomic TOCTOU fix)

    // 订阅 LiveOrderHub
    let mut hub_rx = state.live_orders.subscribe(&tenant_id);

    // 默认订阅全部门店（空 = 不过滤）
    let mut subscribed_edges: Option<HashSet<i64>> = None;

    // 发送初始全量快照 + 在线 edge 列表
    let initial = state.live_orders.get_all_active(&tenant_id, &[]);
    let online_edge_ids = state.live_orders.get_online_edges(&tenant_id, &[]);
    let ready = ConsoleMessage::Ready {
        snapshots: initial,
        online_edge_ids,
    };
    if send_message(&mut sink, &ready).await.is_err() {
        return;
    }

    let mut ping_interval = tokio::time::interval(Duration::from_secs(30));
    ping_interval.tick().await; // skip immediate

    loop {
        tokio::select! {
            _ = ping_interval.tick() => {
                if sink.send(Message::Ping(vec![].into())).await.is_err() {
                    break;
                }
            }

            event = hub_rx.recv() => {
                match event {
                    Ok(hub_event) => {
                        if let Some(msg) = convert_hub_event(hub_event, &subscribed_edges)
                            && send_message(&mut sink, &msg).await.is_err()
                        {
                            break;
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        tracing::warn!(tenant_id = %tenant_id, lagged = n, "Console subscriber lagged, resending full snapshot");
                        // 重新订阅以获取从当前位置开始的新 receiver，避免事件间隙
                        hub_rx = state.live_orders.subscribe(&tenant_id);
                        let edges: Vec<i64> = subscribed_edges
                            .as_ref()
                            .map(|s| s.iter().copied().collect())
                            .unwrap_or_default();
                        let all = state.live_orders.get_all_active(&tenant_id, &edges);
                        let online = state.live_orders.get_online_edges(&tenant_id, &edges);
                        let msg = ConsoleMessage::Ready {
                            snapshots: all,
                            online_edge_ids: online,
                        };
                        if send_message(&mut sink, &msg).await.is_err() {
                            break;
                        }
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }

            msg = stream.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        if let Ok(cmd) = serde_json::from_str::<ConsoleCommand>(&text) {
                            match cmd {
                                ConsoleCommand::Subscribe { edge_server_ids } => {
                                    subscribed_edges = if edge_server_ids.is_empty() {
                                        None
                                    } else {
                                        Some(edge_server_ids.iter().copied().collect())
                                    };

                                    // 重发过滤后的全量快照
                                    let edges: Vec<i64> = subscribed_edges
                                        .as_ref()
                                        .map(|s| s.iter().copied().collect())
                                        .unwrap_or_default();
                                    let filtered = state.live_orders.get_all_active(&tenant_id, &edges);
                                    let online = state.live_orders.get_online_edges(&tenant_id, &edges);
                                    let msg = ConsoleMessage::Ready {
                                        snapshots: filtered,
                                        online_edge_ids: online,
                                    };
                                    if send_message(&mut sink, &msg).await.is_err() {
                                        break;
                                    }
                                }
                            }
                        }
                    }
                    Some(Ok(Message::Pong(_))) => {}
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Err(_)) => break,
                    _ => {}
                }
            }
        }
    }

    // Decrement connection count
    if let Some(counter) = state.console_connections.get(&tenant_id) {
        counter.fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
    }

    tracing::info!(tenant_id = %tenant_id, "Console WS disconnected");
}

/// 将 LiveHubEvent 转换为 ConsoleMessage，应用订阅过滤
fn convert_hub_event(
    event: LiveHubEvent,
    subscribed: &Option<HashSet<i64>>,
) -> Option<ConsoleMessage> {
    match event {
        LiveHubEvent::OrderUpdated(snapshot) => {
            if !passes_filter(subscribed, snapshot.edge_server_id) {
                return None;
            }
            Some(ConsoleMessage::OrderUpdated { snapshot })
        }
        LiveHubEvent::OrderRemoved {
            order_id,
            edge_server_id,
        } => {
            if !passes_filter(subscribed, edge_server_id) {
                return None;
            }
            Some(ConsoleMessage::OrderRemoved {
                order_id,
                edge_server_id,
            })
        }
        LiveHubEvent::EdgeOnline { edge_server_id } => {
            if !passes_filter(subscribed, edge_server_id) {
                return None;
            }
            Some(ConsoleMessage::EdgeStatus {
                edge_server_id,
                online: true,
                cleared_order_ids: vec![],
            })
        }
        LiveHubEvent::EdgeOffline {
            edge_server_id,
            cleared_order_ids,
        } => {
            if !passes_filter(subscribed, edge_server_id) {
                return None;
            }
            Some(ConsoleMessage::EdgeStatus {
                edge_server_id,
                online: false,
                cleared_order_ids,
            })
        }
    }
}

fn passes_filter(subscribed: &Option<HashSet<i64>>, edge_server_id: i64) -> bool {
    match subscribed {
        None => true,
        Some(set) => set.contains(&edge_server_id),
    }
}

async fn send_message<S>(sink: &mut S, msg: &ConsoleMessage) -> Result<(), ()>
where
    S: futures::Sink<Message, Error = axum::Error> + Unpin,
{
    let json = serde_json::to_string(msg).map_err(|_| ())?;
    sink.send(Message::Text(json.into())).await.map_err(|_| ())
}
