//! System State API Handlers

use axum::{extract::State, Json};

use crate::core::ServerState;
use crate::db::models::{Order, SystemState, SystemStateUpdate};
use crate::db::repository::SystemStateRepository;
use crate::utils::{AppError, AppResult};

const RESOURCE: &str = "system_state";

/// Get current system state
pub async fn get(State(state): State<ServerState>) -> AppResult<Json<SystemState>> {
    let repo = SystemStateRepository::new(state.db.clone());
    let system_state = repo
        .get_or_create()
        .await
        .map_err(|e| AppError::database(e.to_string()))?;
    Ok(Json(system_state))
}

/// Update system state
pub async fn update(
    State(state): State<ServerState>,
    Json(payload): Json<SystemStateUpdate>,
) -> AppResult<Json<SystemState>> {
    let repo = SystemStateRepository::new(state.db.clone());
    let system_state = repo
        .update(payload)
        .await
        .map_err(|e| AppError::database(e.to_string()))?;

    state
        .broadcast_sync(RESOURCE, "updated", "main", Some(&system_state))
        .await;

    Ok(Json(system_state))
}

/// Initialize genesis request
#[derive(Debug, serde::Deserialize)]
pub struct InitGenesisRequest {
    pub genesis_hash: String,
}

/// Initialize genesis hash
pub async fn init_genesis(
    State(state): State<ServerState>,
    Json(payload): Json<InitGenesisRequest>,
) -> AppResult<Json<SystemState>> {
    let repo = SystemStateRepository::new(state.db.clone());
    let system_state = repo
        .init_genesis(payload.genesis_hash)
        .await
        .map_err(|e| AppError::database(e.to_string()))?;

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
    let repo = SystemStateRepository::new(state.db.clone());
    let system_state = repo
        .update_last_order(&payload.order_id, payload.order_hash)
        .await
        .map_err(|e| AppError::database(e.to_string()))?;

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
    let repo = SystemStateRepository::new(state.db.clone());
    let system_state = repo
        .update_sync_state(&payload.synced_up_to_id, payload.synced_up_to_hash)
        .await
        .map_err(|e| AppError::database(e.to_string()))?;

    state
        .broadcast_sync(RESOURCE, "sync_state_updated", "main", Some(&system_state))
        .await;

    Ok(Json(system_state))
}

/// Get pending orders for sync
pub async fn get_pending_sync(State(state): State<ServerState>) -> AppResult<Json<Vec<Order>>> {
    let repo = SystemStateRepository::new(state.db.clone());
    let orders = repo
        .get_pending_sync_orders()
        .await
        .map_err(|e| AppError::database(e.to_string()))?;
    Ok(Json(orders))
}
