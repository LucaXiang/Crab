//! POST /api/edge/sync — receive data batches from edge-servers

use axum::{Extension, Json, extract::State};
use shared::cloud::{CloudSyncBatch, CloudSyncError, CloudSyncResponse};

use crate::auth::EdgeIdentity;
use crate::db::sync_store;
use crate::state::AppState;

/// Handle sync batch from edge-server
///
/// 1. Extract EdgeIdentity from middleware
/// 2. Auto-register edge-server if new
/// 3. Process each sync item
/// 4. Update sync cursors
/// 5. Return response with accepted/rejected counts
pub async fn handle_sync(
    State(state): State<AppState>,
    Extension(identity): Extension<EdgeIdentity>,
    Json(batch): Json<CloudSyncBatch>,
) -> Result<Json<CloudSyncResponse>, (http::StatusCode, Json<serde_json::Value>)> {
    let now = shared::util::now_millis();

    // Auto-register edge-server
    let edge_server_id = sync_store::ensure_edge_server(
        &state.pool,
        &identity.entity_id,
        &identity.tenant_id,
        &identity.device_id,
        now,
    )
    .await
    .map_err(|e| {
        tracing::error!("Failed to register edge-server: {e}");
        internal_error("Failed to register edge-server")
    })?;

    // Update last_sync_at
    sync_store::update_last_sync(&state.pool, edge_server_id, now)
        .await
        .map_err(|e| {
            tracing::error!("Failed to update last_sync: {e}");
            internal_error("Database error")
        })?;

    let mut accepted = 0u32;
    let mut rejected = 0u32;
    let mut errors = Vec::new();

    // Process each item
    for (idx, item) in batch.items.iter().enumerate() {
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

                // Update sync cursor
                if let Err(e) = sync_store::update_cursor(
                    &state.pool,
                    edge_server_id,
                    &item.resource,
                    item.version as i64,
                    now,
                )
                .await
                {
                    tracing::warn!(
                        resource = %item.resource,
                        version = item.version,
                        "Failed to update sync cursor: {e}"
                    );
                }
            }
            Err(e) => {
                rejected += 1;
                errors.push(CloudSyncError {
                    index: idx as u32,
                    resource_id: item.resource_id.clone(),
                    message: e.to_string(),
                });
            }
        }
    }

    tracing::info!(
        edge_id = %identity.entity_id,
        tenant_id = %identity.tenant_id,
        accepted,
        rejected,
        total = batch.items.len(),
        "Sync batch processed"
    );

    Ok(Json(CloudSyncResponse {
        accepted,
        rejected,
        errors,
        pending_commands: vec![], // Future: query pending commands
    }))
}

fn internal_error(msg: &str) -> (http::StatusCode, Json<serde_json::Value>) {
    (
        http::StatusCode::INTERNAL_SERVER_ERROR,
        Json(serde_json::json!({ "error": msg })),
    )
}
