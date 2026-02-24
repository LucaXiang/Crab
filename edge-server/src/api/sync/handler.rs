//! Sync API Handlers

use axum::{Json, extract::State};
use shared::cloud::SyncResource;
use shared::models::SyncStatus;
use std::collections::HashMap;

use crate::core::ServerState;

/// GET /api/sync/status - 获取同步状态
///
/// 返回服务器 epoch 和各资源类型的当前版本号
/// 客户端重连时调用此接口检查是否需要刷新
pub async fn get_sync_status(State(state): State<ServerState>) -> Json<SyncStatus> {
    let mut versions = HashMap::new();

    for &resource in SyncResource::CLIENT_VISIBLE {
        versions.insert(resource.to_string(), state.resource_versions.get(resource));
    }

    Json(SyncStatus {
        epoch: state.epoch.clone(),
        versions,
    })
}
