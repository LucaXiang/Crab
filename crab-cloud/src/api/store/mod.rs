//! Store Resource CRUD API handlers
//!
//! Each write operation:
//! 1. Verify store ownership
//! 2. Write directly to PG (authoritative source)
//! 3. Increment catalog version
//! 4. Fire-and-forget push StoreOp to edge (if online)
//! 5. Return StoreOpResult to console

pub mod attribute;
pub mod category;
pub mod dining_table;
pub mod employee;
pub mod label_template;
pub mod price_rule;
pub mod product;
pub mod tag;
pub mod zone;

pub use attribute::*;
pub use category::*;
pub use dining_table::*;
pub use employee::*;
pub use label_template::*;
pub use price_rule::*;
pub use product::*;
pub use tag::*;
pub use zone::*;

use shared::cloud::store_op::StoreOp;
use shared::cloud::ws::{CloudMessage, CloudRpc};
use shared::error::{AppError, ErrorCode};

use crate::state::AppState;

fn internal(e: impl std::fmt::Display) -> AppError {
    tracing::error!("Store query error: {e}");
    AppError::new(ErrorCode::InternalError)
}

/// Push StoreOp to edge: direct send if online, queue to pending_ops if offline.
async fn push_to_edge(state: &AppState, edge_server_id: i64, op: StoreOp) {
    let now = shared::util::now_millis();

    if let Some(sender) = state.edges.connected.get(&edge_server_id) {
        let msg = CloudMessage::Rpc {
            id: format!("push-{}", uuid::Uuid::new_v4()),
            payload: Box::new(CloudRpc::StoreOp {
                op: Box::new(op),
                changed_at: Some(now),
            }),
        };
        let _ = sender.try_send(msg);
    } else if let Err(e) =
        crate::db::store::pending_ops::insert(&state.pool, edge_server_id, &op, now).await
    {
        tracing::error!(edge_server_id, "Failed to queue pending op: {e}");
    }
}

/// Fire-and-forget: send EnsureImage RPC to edge so it downloads the image from S3.
async fn fire_ensure_image(
    state: &AppState,
    store_id: i64,
    tenant_id: &str,
    image_hash: Option<&str>,
) {
    let hash = match image_hash {
        Some(h) if !h.is_empty() => h,
        _ => return,
    };

    let presigned_url = match super::image::presigned_get_url(state, tenant_id, hash).await {
        Ok(url) => url,
        Err(e) => {
            tracing::warn!(hash = %hash, error = %e, "Failed to generate presigned URL for image");
            return;
        }
    };

    let sender = match state.edges.connected.get(&store_id) {
        Some(s) => s.clone(),
        None => return,
    };

    let msg = CloudMessage::Rpc {
        id: format!("img-{hash}"),
        payload: Box::new(CloudRpc::StoreOp {
            op: Box::new(StoreOp::EnsureImage {
                presigned_url,
                hash: hash.to_string(),
            }),
            changed_at: None,
        }),
    };

    let _ = sender.try_send(msg);
}

use super::tenant::verify_store;
