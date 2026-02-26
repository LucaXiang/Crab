//! POST /api/edge/sync — receive data batches from edge-servers

use axum::{Extension, Json, extract::State};
use shared::cloud::{CloudSyncBatch, CloudSyncError, CloudSyncResponse, MAX_SYNC_BATCH_ITEMS};
use shared::error::{AppError, ErrorCode};

use crate::auth::EdgeIdentity;
use crate::db::{audit, sync_store};
use crate::state::AppState;

/// Handle sync batch from edge-server
///
/// 1. Extract EdgeIdentity from middleware
/// 2. Auto-register edge-server if new
/// 3. Process each sync item
/// 4. Return response with accepted/rejected counts
pub async fn handle_sync(
    State(state): State<AppState>,
    Extension(identity): Extension<EdgeIdentity>,
    Json(batch): Json<CloudSyncBatch>,
) -> Result<Json<CloudSyncResponse>, AppError> {
    // Only Server entities can sync — Client devices must not use this endpoint
    if identity.entity_type != shared::activation::EntityType::Server {
        tracing::warn!(
            entity_id = %identity.entity_id,
            entity_type = ?identity.entity_type,
            "Non-server entity attempted cloud sync"
        );
        return Err(AppError::with_message(
            ErrorCode::PermissionDenied,
            "Only server entities can sync to cloud",
        ));
    }

    // Reject oversized batches
    if batch.items.len() > MAX_SYNC_BATCH_ITEMS {
        return Err(AppError::with_message(
            ErrorCode::ValidationFailed,
            format!(
                "Batch too large: {} items (max {MAX_SYNC_BATCH_ITEMS})",
                batch.items.len()
            ),
        ));
    }

    let now = shared::util::now_millis();

    // Auto-register edge-server
    let store_id = sync_store::ensure_store(
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
    sync_store::update_last_sync(&state.pool, store_id, now)
        .await
        .map_err(|e| {
            tracing::error!("Failed to update last_sync: {e}");
            AppError::new(ErrorCode::InternalError)
        })?;

    let mut accepted = 0u32;
    let mut rejected = 0u32;
    let mut errors = Vec::new();

    // Process each item
    for (idx, item) in batch.items.iter().enumerate() {
        match sync_store::upsert_resource(&state.pool, store_id, &identity.tenant_id, item, now)
            .await
        {
            Ok(()) => {
                accepted += 1;

                // Update sync cursor
                if let Err(e) = sync_store::update_cursor(
                    &state.pool,
                    store_id,
                    item.resource,
                    i64::try_from(item.version).unwrap_or(i64::MAX),
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
                    index: u32::try_from(idx).unwrap_or(u32::MAX),
                    resource_id: item.resource_id.clone(),
                    message: e.to_string(),
                });
            }
        }
    }

    // Audit
    let sync_detail = serde_json::json!({
        "edge_id": identity.entity_id,
        "store_id": store_id,
        "accepted": accepted,
        "rejected": rejected,
        "total": batch.items.len(),
    });
    let _ = audit::log(
        &state.pool,
        &identity.tenant_id,
        "sync_batch",
        Some(&sync_detail),
        None,
        now,
    )
    .await;

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
    }))
}
