//! Store Info API Handlers

use axum::{
    Json,
    extract::{Extension, State},
};

use crate::audit::{create_diff, AuditAction};
use crate::audit_log;
use crate::auth::CurrentUser;
use crate::core::ServerState;
use crate::db::repository::store_info;
use crate::utils::AppResult;
use crate::utils::validation::{validate_optional_text, MAX_NAME_LEN, MAX_ADDRESS_LEN, MAX_SHORT_TEXT_LEN, MAX_URL_LEN, MAX_EMAIL_LEN};
use shared::models::{StoreInfo, StoreInfoUpdate};

const RESOURCE: &str = "store_info";

fn validate_update(payload: &StoreInfoUpdate) -> AppResult<()> {
    validate_optional_text(&payload.name, "name", MAX_NAME_LEN)?;
    validate_optional_text(&payload.address, "address", MAX_ADDRESS_LEN)?;
    validate_optional_text(&payload.nif, "nif", MAX_SHORT_TEXT_LEN)?;
    validate_optional_text(&payload.logo_url, "logo_url", MAX_URL_LEN)?;
    validate_optional_text(&payload.phone, "phone", MAX_SHORT_TEXT_LEN)?;
    validate_optional_text(&payload.email, "email", MAX_EMAIL_LEN)?;
    validate_optional_text(&payload.website, "website", MAX_URL_LEN)?;
    validate_optional_text(&payload.business_day_cutoff, "business_day_cutoff", MAX_SHORT_TEXT_LEN)?;
    Ok(())
}

/// Get current store info
pub async fn get(State(state): State<ServerState>) -> AppResult<Json<StoreInfo>> {
    let store_info = store_info::get_or_create(&state.pool).await?;
    Ok(Json(store_info))
}

/// Update store info
pub async fn update(
    State(state): State<ServerState>,
    Extension(current_user): Extension<CurrentUser>,
    Json(payload): Json<StoreInfoUpdate>,
) -> AppResult<Json<StoreInfo>> {
    validate_update(&payload)?;

    let old_store_info = store_info::get_or_create(&state.pool).await?;
    let store_info = store_info::update(&state.pool, payload).await?;

    audit_log!(
        state.audit_service,
        AuditAction::StoreInfoChanged,
        "store_info", "store_info:main",
        operator_id = Some(current_user.id),
        operator_name = Some(current_user.display_name.clone()),
        details = create_diff(&old_store_info, &store_info, "store_info")
    );

    state
        .broadcast_sync(RESOURCE, "updated", "main", Some(&store_info))
        .await;

    // 通知依赖配置的调度器（如班次检测器）立即重检
    state.config_notify.notify_waiters();

    Ok(Json(store_info))
}
