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

/// Fire-and-forget push StoreOp to edge if it's currently connected.
fn push_to_edge_if_online(state: &AppState, store_id: i64, op: StoreOp) {
    let sender = match state.edges.connected.get(&store_id) {
        Some(s) => s.clone(),
        None => return,
    };

    let msg = CloudMessage::Rpc {
        id: format!("push-{}", uuid::Uuid::new_v4()),
        payload: Box::new(CloudRpc::StoreOp(Box::new(op))),
    };

    let _ = sender.try_send(msg);
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
        payload: Box::new(CloudRpc::StoreOp(Box::new(StoreOp::EnsureImage {
            presigned_url,
            hash: hash.to_string(),
        }))),
    };

    let _ = sender.try_send(msg);
}

use super::tenant::verify_store;
