//! POST /api/edge/sync — receive data batches from edge-servers

use axum::{Extension, Json, extract::State};
use shared::cloud::{CloudCommand, CloudSyncBatch, CloudSyncError, CloudSyncResponse};
use shared::error::{AppError, ErrorCode};

use crate::auth::EdgeIdentity;
use crate::db::{commands, sync_store};
use crate::state::AppState;

/// Handle sync batch from edge-server
///
/// 1. Extract EdgeIdentity from middleware
/// 2. Auto-register edge-server if new
/// 3. Process command results from previous batch
/// 4. Process each sync item
/// 5. Query pending commands for this edge-server
/// 6. Return response with accepted/rejected counts + pending commands
pub async fn handle_sync(
    State(state): State<AppState>,
    Extension(identity): Extension<EdgeIdentity>,
    Json(batch): Json<CloudSyncBatch>,
) -> Result<Json<CloudSyncResponse>, AppError> {
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
        AppError::new(ErrorCode::InternalError)
    })?;

    // Update last_sync_at
    sync_store::update_last_sync(&state.pool, edge_server_id, now)
        .await
        .map_err(|e| {
            tracing::error!("Failed to update last_sync: {e}");
            AppError::new(ErrorCode::InternalError)
        })?;

    // Process command results from edge-server
    if !batch.command_results.is_empty() {
        if let Err(e) = commands::complete_commands(&state.pool, &batch.command_results, now).await
        {
            tracing::warn!("Failed to process command results: {e}");
        } else {
            tracing::info!(
                count = batch.command_results.len(),
                "Processed command results from edge"
            );
        }
    }

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

    // Query pending commands for this edge-server
    let pending_commands = match commands::get_pending(&state.pool, edge_server_id, 10).await {
        Ok(pending) => {
            if !pending.is_empty() {
                let ids: Vec<i64> = pending.iter().map(|c| c.id).collect();
                if let Err(e) = commands::mark_delivered(&state.pool, &ids).await {
                    tracing::warn!("Failed to mark commands as delivered: {e}");
                }
            }
            pending
                .into_iter()
                .map(|c| CloudCommand {
                    id: c.id.to_string(),
                    command_type: c.command_type,
                    payload: c.payload,
                    created_at: c.created_at,
                })
                .collect()
        }
        Err(e) => {
            tracing::warn!("Failed to query pending commands: {e}");
            vec![]
        }
    };

    tracing::info!(
        edge_id = %identity.entity_id,
        tenant_id = %identity.tenant_id,
        accepted,
        rejected,
        total = batch.items.len(),
        pending_cmds = pending_commands.len(),
        "Sync batch processed"
    );

    Ok(Json(CloudSyncResponse {
        accepted,
        rejected,
        errors,
        pending_commands,
    }))
}
