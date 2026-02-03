//! Store Info API Handlers

use axum::{
    Json,
    extract::{Extension, State},
};

use crate::audit::AuditAction;
use crate::audit_log;
use crate::auth::CurrentUser;
use crate::core::ServerState;
use crate::db::models::{StoreInfo, StoreInfoUpdate};
use crate::db::repository::StoreInfoRepository;
use crate::utils::AppResult;

const RESOURCE: &str = "store_info";

/// Get current store info
pub async fn get(State(state): State<ServerState>) -> AppResult<Json<StoreInfo>> {
    let repo = StoreInfoRepository::new(state.db.clone());
    let store_info = repo
        .get_or_create()
        .await
        ?;
    Ok(Json(store_info))
}

/// Update store info
pub async fn update(
    State(state): State<ServerState>,
    Extension(current_user): Extension<CurrentUser>,
    Json(payload): Json<StoreInfoUpdate>,
) -> AppResult<Json<StoreInfo>> {
    let repo = StoreInfoRepository::new(state.db.clone());
    let store_info = repo
        .update(payload)
        .await
        ?;

    audit_log!(
        state.audit_service,
        AuditAction::StoreInfoChanged,
        "store_info", "store_info:main",
        operator_id = Some(current_user.id.clone()),
        operator_name = Some(current_user.display_name.clone()),
        details = serde_json::json!({"name": &store_info.name})
    );

    state
        .broadcast_sync(RESOURCE, "updated", "main", Some(&store_info))
        .await;

    // 通知依赖配置的调度器（如班次检测器）立即重检
    state.config_notify.notify_waiters();

    Ok(Json(store_info))
}
