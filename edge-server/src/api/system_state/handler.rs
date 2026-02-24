//! System State API Handlers

use axum::{Json, extract::State};

use crate::core::ServerState;
use crate::db::repository::system_state;
use crate::utils::AppResult;
use shared::models::{SystemState, SystemStateUpdate};

use shared::cloud::SyncResource;
const RESOURCE: SyncResource = SyncResource::SystemState;

/// Get current system state
pub async fn get(State(state): State<ServerState>) -> AppResult<Json<SystemState>> {
    let system_state = system_state::get_or_create(&state.pool).await?;
    Ok(Json(system_state))
}

/// Update system state
pub async fn update(
    State(state): State<ServerState>,
    Json(payload): Json<SystemStateUpdate>,
) -> AppResult<Json<SystemState>> {
    let system_state = system_state::update(&state.pool, payload).await?;

    state
        .broadcast_sync(RESOURCE, "updated", "main", Some(&system_state))
        .await;

    Ok(Json(system_state))
}

/// Initialize genesis hash
pub async fn init_genesis(
    State(state): State<ServerState>,
    Json(payload): Json<shared::models::InitGenesisRequest>,
) -> AppResult<Json<SystemState>> {
    let system_state = system_state::init_genesis(&state.pool, payload.genesis_hash).await?;

    state
        .broadcast_sync(RESOURCE, "genesis_initialized", "main", Some(&system_state))
        .await;

    Ok(Json(system_state))
}

/// Update last order request
#[derive(Debug, serde::Deserialize)]
pub struct UpdateLastOrderRequest {
    pub order_id: String,
    pub order_hash: String,
}

/// Update last order info
pub async fn update_last_order(
    State(state): State<ServerState>,
    Json(payload): Json<UpdateLastOrderRequest>,
) -> AppResult<Json<SystemState>> {
    let system_state =
        system_state::update_last_order(&state.pool, &payload.order_id, payload.order_hash).await?;

    state
        .broadcast_sync(RESOURCE, "last_order_updated", "main", Some(&system_state))
        .await;

    Ok(Json(system_state))
}

/// Update sync state request
#[derive(Debug, serde::Deserialize)]
pub struct UpdateSyncStateRequest {
    pub synced_up_to_id: String,
    pub synced_up_to_hash: String,
}

/// Update sync state
pub async fn update_sync_state(
    State(state): State<ServerState>,
    Json(payload): Json<UpdateSyncStateRequest>,
) -> AppResult<Json<SystemState>> {
    let system_state = system_state::update_sync_state(
        &state.pool,
        &payload.synced_up_to_id,
        payload.synced_up_to_hash,
    )
    .await?;

    state
        .broadcast_sync(RESOURCE, "sync_state_updated", "main", Some(&system_state))
        .await;

    Ok(Json(system_state))
}
