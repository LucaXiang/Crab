//! Edge RPC helper — shared pattern for sending commands to edge via WebSocket

use shared::cloud::CloudMessage;
use shared::cloud::ws::{CloudRpc, CloudRpcResult};
use shared::error::{AppError, ErrorCode};

use crate::state::EdgeConnections;

/// Send an RPC to an edge server and wait for the result (10s timeout).
///
/// Handles: channel lookup, pending_rpcs registration, timeout cleanup.
pub async fn call_edge_rpc(
    edges: &EdgeConnections,
    store_id: i64,
    rpc: CloudRpc,
) -> Result<CloudRpcResult, AppError> {
    let sender = edges
        .connected
        .get(&store_id)
        .map(|s| s.clone())
        .ok_or_else(|| AppError::with_message(ErrorCode::NotFound, "Edge server is offline"))?;

    let rpc_id = uuid::Uuid::new_v4().to_string();
    let now = shared::util::now_millis();

    let (tx, rx) = tokio::sync::oneshot::channel();
    edges.pending_rpcs.insert(rpc_id.clone(), (now, tx));

    let msg = CloudMessage::Rpc {
        id: rpc_id.clone(),
        payload: Box::new(rpc),
    };

    if sender.try_send(msg).is_err() {
        edges.pending_rpcs.remove(&rpc_id);
        return Err(AppError::with_message(
            ErrorCode::NotFound,
            "Edge server command queue full",
        ));
    }

    match tokio::time::timeout(std::time::Duration::from_secs(10), rx).await {
        Ok(Ok(result)) => Ok(result),
        Ok(Err(_)) => {
            edges.pending_rpcs.remove(&rpc_id);
            Err(AppError::with_message(
                ErrorCode::NotFound,
                "Edge server disconnected",
            ))
        }
        Err(_) => {
            edges.pending_rpcs.remove(&rpc_id);
            Err(AppError::with_message(ErrorCode::NotFound, "RPC timed out"))
        }
    }
}
