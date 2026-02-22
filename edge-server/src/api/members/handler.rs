//! Member API Handlers

use axum::{
    Json,
    extract::{Extension, Path, Query, State},
};

use crate::audit::{AuditAction, create_diff, create_snapshot};
use crate::audit_log;
use crate::auth::CurrentUser;
use crate::core::ServerState;
use crate::db::repository::{member, stamp};
use crate::utils::validation::{
    MAX_EMAIL_LEN, MAX_NAME_LEN, MAX_NOTE_LEN, MAX_SHORT_TEXT_LEN, validate_optional_text,
    validate_required_text,
};
use crate::utils::{AppError, AppResult};
use shared::error::ErrorCode;
use shared::models::MemberWithGroup;

const RESOURCE: &str = "member";

fn validate_create(payload: &shared::models::MemberCreate) -> AppResult<()> {
    validate_required_text(&payload.name, "name", MAX_NAME_LEN)?;
    validate_optional_text(&payload.phone, "phone", MAX_SHORT_TEXT_LEN)?;
    validate_optional_text(&payload.card_number, "card_number", MAX_SHORT_TEXT_LEN)?;
    validate_optional_text(&payload.birthday, "birthday", MAX_SHORT_TEXT_LEN)?;
    validate_optional_text(&payload.email, "email", MAX_EMAIL_LEN)?;
    validate_optional_text(&payload.notes, "notes", MAX_NOTE_LEN)?;
    Ok(())
}

fn validate_update(payload: &shared::models::MemberUpdate) -> AppResult<()> {
    if let Some(name) = &payload.name {
        validate_required_text(name, "name", MAX_NAME_LEN)?;
    }
    validate_optional_text(&payload.phone, "phone", MAX_SHORT_TEXT_LEN)?;
    validate_optional_text(&payload.card_number, "card_number", MAX_SHORT_TEXT_LEN)?;
    validate_optional_text(&payload.birthday, "birthday", MAX_SHORT_TEXT_LEN)?;
    validate_optional_text(&payload.email, "email", MAX_EMAIL_LEN)?;
    validate_optional_text(&payload.notes, "notes", MAX_NOTE_LEN)?;
    Ok(())
}

#[derive(serde::Deserialize)]
pub struct SearchQuery {
    pub q: String,
}

/// GET /api/members - 获取所有会员
pub async fn list(State(state): State<ServerState>) -> AppResult<Json<Vec<MemberWithGroup>>> {
    let members = member::find_all(&state.pool).await?;
    Ok(Json(members))
}

/// GET /api/members/search?q=xxx - 搜索会员
pub async fn search(
    State(state): State<ServerState>,
    Query(query): Query<SearchQuery>,
) -> AppResult<Json<Vec<MemberWithGroup>>> {
    let members = member::search(&state.pool, &query.q).await?;
    Ok(Json(members))
}

/// Stamp progress with targets (for frontend dynamic progress calculation)
#[derive(serde::Serialize)]
pub struct StampProgressWithTargets {
    pub stamp_activity_id: i64,
    pub stamp_activity_name: String,
    pub stamps_required: i32,
    pub current_stamps: i32,
    pub completed_cycles: i32,
    pub is_redeemable: bool,
    pub is_cyclic: bool,
    pub reward_strategy: shared::models::RewardStrategy,
    pub reward_quantity: i32,
    pub designated_product_id: Option<i64>,
    pub stamp_targets: Vec<shared::models::StampTarget>,
    pub reward_targets: Vec<shared::models::StampRewardTarget>,
}

/// Member detail response (member + stamp progress)
#[derive(serde::Serialize)]
pub struct MemberDetail {
    #[serde(flatten)]
    pub member: MemberWithGroup,
    pub stamp_progress: Vec<StampProgressWithTargets>,
}

/// GET /api/members/:id - 获取单个会员（含集章进度 + 计章对象）
pub async fn get_by_id(
    State(state): State<ServerState>,
    Path(id): Path<i64>,
) -> AppResult<Json<MemberDetail>> {
    let member = member::find_by_id(&state.pool, id).await?.ok_or_else(|| {
        AppError::with_message(
            ErrorCode::MemberNotFound,
            format!("Member {} not found", id),
        )
    })?;

    let progress_list = stamp::find_progress_details_by_member(&state.pool, id).await?;

    // Enrich each progress with its stamp/reward targets
    let mut stamp_progress = Vec::with_capacity(progress_list.len());
    for p in progress_list {
        let targets = crate::db::repository::marketing_group::find_stamp_targets(
            &state.pool,
            p.stamp_activity_id,
        )
        .await
        .unwrap_or_default();

        let mut reward_targets = crate::db::repository::marketing_group::find_reward_targets(
            &state.pool,
            p.stamp_activity_id,
        )
        .await
        .unwrap_or_default();

        // If no explicit reward targets, fall back to stamp targets
        if reward_targets.is_empty() {
            reward_targets = targets
                .iter()
                .map(|t| shared::models::StampRewardTarget {
                    id: t.id,
                    stamp_activity_id: t.stamp_activity_id,
                    target_type: t.target_type.clone(),
                    target_id: t.target_id,
                })
                .collect();
        }

        stamp_progress.push(StampProgressWithTargets {
            stamp_activity_id: p.stamp_activity_id,
            stamp_activity_name: p.stamp_activity_name,
            stamps_required: p.stamps_required,
            current_stamps: p.current_stamps,
            completed_cycles: p.completed_cycles,
            is_redeemable: p.is_redeemable,
            is_cyclic: p.is_cyclic,
            reward_strategy: p.reward_strategy,
            reward_quantity: p.reward_quantity,
            designated_product_id: p.designated_product_id,
            stamp_targets: targets,
            reward_targets,
        });
    }

    Ok(Json(MemberDetail {
        member,
        stamp_progress,
    }))
}

/// POST /api/members - 创建会员
pub async fn create(
    State(state): State<ServerState>,
    Extension(current_user): Extension<CurrentUser>,
    Json(payload): Json<shared::models::MemberCreate>,
) -> AppResult<Json<MemberWithGroup>> {
    validate_create(&payload)?;

    let member = member::create(&state.pool, payload).await?;

    let id = member.id.to_string();

    audit_log!(
        state.audit_service,
        AuditAction::MemberCreated,
        "member",
        &id,
        operator_id = Some(current_user.id),
        operator_name = Some(current_user.display_name.clone()),
        details = create_snapshot(&member, "member")
    );

    state
        .broadcast_sync(RESOURCE, "created", &id, Some(&member))
        .await;

    Ok(Json(member))
}

/// PUT /api/members/:id - 更新会员
pub async fn update(
    State(state): State<ServerState>,
    Extension(current_user): Extension<CurrentUser>,
    Path(id): Path<i64>,
    Json(payload): Json<shared::models::MemberUpdate>,
) -> AppResult<Json<MemberWithGroup>> {
    validate_update(&payload)?;

    let old_member = member::find_by_id(&state.pool, id).await?.ok_or_else(|| {
        AppError::with_message(
            ErrorCode::MemberNotFound,
            format!("Member {} not found", id),
        )
    })?;

    let member = member::update(&state.pool, id, payload).await?;

    let id_str = id.to_string();

    audit_log!(
        state.audit_service,
        AuditAction::MemberUpdated,
        "member",
        &id_str,
        operator_id = Some(current_user.id),
        operator_name = Some(current_user.display_name.clone()),
        details = create_diff(&old_member, &member, "member")
    );

    state
        .broadcast_sync(RESOURCE, "updated", &id_str, Some(&member))
        .await;

    Ok(Json(member))
}

/// DELETE /api/members/:id - 删除会员（软删除）
pub async fn delete(
    State(state): State<ServerState>,
    Extension(current_user): Extension<CurrentUser>,
    Path(id): Path<i64>,
) -> AppResult<Json<bool>> {
    let name_for_audit = member::find_by_id(&state.pool, id)
        .await
        .ok()
        .flatten()
        .map(|m| m.name.clone())
        .unwrap_or_default();

    let result = member::delete(&state.pool, id).await?;

    let id_str = id.to_string();

    if result {
        audit_log!(
            state.audit_service,
            AuditAction::MemberDeleted,
            "member",
            &id_str,
            operator_id = Some(current_user.id),
            operator_name = Some(current_user.display_name.clone()),
            details = serde_json::json!({"name": name_for_audit})
        );

        state
            .broadcast_sync::<()>(RESOURCE, "deleted", &id_str, None)
            .await;
    }

    Ok(Json(result))
}
