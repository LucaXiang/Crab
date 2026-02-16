//! Sync API Handlers

use axum::{Json, extract::State};
use shared::models::SyncStatus;
use std::collections::HashMap;

use crate::core::ServerState;

/// 资源类型列表 (必须与前端 registry 保持一致)
const RESOURCE_TYPES: &[&str] = &[
    "product",
    "category",
    "tag",
    "attribute",
    "zone",
    "dining_table",
    "employee",
    "role",
    "price_rule",
    "print_destination",
];

/// GET /api/sync/status - 获取同步状态
///
/// 返回服务器 epoch 和各资源类型的当前版本号
/// 客户端重连时调用此接口检查是否需要刷新
pub async fn get_sync_status(State(state): State<ServerState>) -> Json<SyncStatus> {
    let mut versions = HashMap::new();

    for resource in RESOURCE_TYPES {
        versions.insert(resource.to_string(), state.resource_versions.get(resource));
    }

    Json(SyncStatus {
        epoch: state.epoch.clone(),
        versions,
    })
}
