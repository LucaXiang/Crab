//! Member API Handlers

use axum::{
    Json,
    extract::{Extension, Path, Query, State},
};

use crate::audit::{create_diff, create_snapshot, AuditAction};
use crate::audit_log;
use crate::auth::CurrentUser;
use crate::core::ServerState;
use crate::db::repository::{member, stamp};
use crate::utils::AppResult;
use shared::models::MemberWithGroup;

const RESOURCE: &str = "member";

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

/// Member detail response (member + stamp progress)
#[derive(serde::Serialize)]
pub struct MemberDetail {
    #[serde(flatten)]
    pub member: MemberWithGroup,
    pub stamp_progress: Vec<shared::models::MemberStampProgressDetail>,
}

/// GET /api/members/:id - 获取单个会员（含集章进度）
pub async fn get_by_id(
    State(state): State<ServerState>,
    Path(id): Path<i64>,
) -> AppResult<Json<MemberDetail>> {
    let member = member::find_by_id(&state.pool, id)
        .await?
        .ok_or_else(|| crate::utils::AppError::not_found(format!("Member {}", id)))?;

    let stamp_progress = stamp::find_progress_details_by_member(&state.pool, id).await?;

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
    let member = member::create(&state.pool, payload).await?;

    let id = member.id.to_string();

    audit_log!(
        state.audit_service,
        AuditAction::MemberCreated,
        "member", &id,
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
    let old_member = member::find_by_id(&state.pool, id)
        .await?
        .ok_or_else(|| crate::utils::AppError::not_found(format!("Member {}", id)))?;

    let member = member::update(&state.pool, id, payload).await?;

    let id_str = id.to_string();

    audit_log!(
        state.audit_service,
        AuditAction::MemberUpdated,
        "member", &id_str,
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
            "member", &id_str,
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
