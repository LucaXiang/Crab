//! Marketing Group API Handlers

use axum::{
    Json,
    extract::{Extension, Path, State},
};

use crate::audit::{create_diff, create_snapshot, AuditAction};
use crate::audit_log;
use crate::auth::CurrentUser;
use crate::core::ServerState;
use crate::db::repository::marketing_group;
use crate::utils::{AppError, AppResult};
use crate::utils::validation::{validate_required_text, validate_optional_text, MAX_NAME_LEN, MAX_NOTE_LEN, MAX_RECEIPT_NAME_LEN};
use shared::models::{
    MarketingGroup, MgDiscountRule, MgDiscountRuleCreate, MgDiscountRuleUpdate,
    StampActivityDetail,
};
use shared::models::price_rule::AdjustmentType;

const RESOURCE: &str = "marketing_group";

fn validate_group_create(payload: &shared::models::MarketingGroupCreate) -> AppResult<()> {
    validate_required_text(&payload.name, "name", MAX_NAME_LEN)?;
    validate_required_text(&payload.display_name, "display_name", MAX_NAME_LEN)?;
    validate_optional_text(&payload.description, "description", MAX_NOTE_LEN)?;
    Ok(())
}

fn validate_group_update(payload: &shared::models::MarketingGroupUpdate) -> AppResult<()> {
    if let Some(name) = &payload.name {
        validate_required_text(name, "name", MAX_NAME_LEN)?;
    }
    if let Some(display_name) = &payload.display_name {
        validate_required_text(display_name, "display_name", MAX_NAME_LEN)?;
    }
    validate_optional_text(&payload.description, "description", MAX_NOTE_LEN)?;
    Ok(())
}

fn validate_rule_create(payload: &MgDiscountRuleCreate) -> AppResult<()> {
    validate_required_text(&payload.name, "name", MAX_NAME_LEN)?;
    validate_required_text(&payload.display_name, "display_name", MAX_NAME_LEN)?;
    validate_required_text(&payload.receipt_name, "receipt_name", MAX_RECEIPT_NAME_LEN)?;
    Ok(())
}

fn validate_rule_update(payload: &MgDiscountRuleUpdate) -> AppResult<()> {
    if let Some(name) = &payload.name {
        validate_required_text(name, "name", MAX_NAME_LEN)?;
    }
    if let Some(display_name) = &payload.display_name {
        validate_required_text(display_name, "display_name", MAX_NAME_LEN)?;
    }
    if let Some(receipt_name) = &payload.receipt_name {
        validate_required_text(receipt_name, "receipt_name", MAX_RECEIPT_NAME_LEN)?;
    }
    Ok(())
}

fn validate_activity_create(payload: &shared::models::StampActivityCreate) -> AppResult<()> {
    validate_required_text(&payload.name, "name", MAX_NAME_LEN)?;
    validate_required_text(&payload.display_name, "display_name", MAX_NAME_LEN)?;
    Ok(())
}

fn validate_activity_update(payload: &shared::models::StampActivityUpdate) -> AppResult<()> {
    if let Some(name) = &payload.name {
        validate_required_text(name, "name", MAX_NAME_LEN)?;
    }
    if let Some(display_name) = &payload.display_name {
        validate_required_text(display_name, "display_name", MAX_NAME_LEN)?;
    }
    Ok(())
}

/// Validate points_earn_rate
fn validate_points_earn_rate(rate: Option<f64>) -> AppResult<()> {
    if let Some(r) = rate {
        if !r.is_finite() {
            return Err(AppError::validation("points_earn_rate must be a finite number"));
        }
        if r < 0.0 {
            return Err(AppError::validation(format!(
                "points_earn_rate must be non-negative, got {r}"
            )));
        }
    }
    Ok(())
}

/// Validate MG discount rule adjustment_value
fn validate_mg_adjustment(adjustment_type: &AdjustmentType, value: f64) -> AppResult<()> {
    if !value.is_finite() {
        return Err(AppError::validation("adjustment_value must be a finite number"));
    }
    if value < 0.0 {
        return Err(AppError::validation("adjustment_value must be non-negative"));
    }
    match adjustment_type {
        AdjustmentType::Percentage => {
            if value > 100.0 {
                return Err(AppError::validation(
                    "Percentage adjustment_value must be between 0 and 100",
                ));
            }
        }
        AdjustmentType::FixedAmount => {
            if value > 1_000_000.0 {
                return Err(AppError::validation(
                    "FixedAmount adjustment_value must not exceed 1,000,000",
                ));
            }
        }
    }
    Ok(())
}

/// Marketing group detail response
#[derive(serde::Serialize)]
pub struct MarketingGroupDetail {
    #[serde(flatten)]
    pub group: MarketingGroup,
    pub discount_rules: Vec<MgDiscountRule>,
    pub stamp_activities: Vec<StampActivityDetail>,
}

/// GET /api/marketing-groups - 获取所有营销组
pub async fn list(State(state): State<ServerState>) -> AppResult<Json<Vec<MarketingGroup>>> {
    let groups = marketing_group::find_all(&state.pool).await?;
    Ok(Json(groups))
}

/// GET /api/marketing-groups/:id - 获取营销组详情（含折扣规则、集章活动）
pub async fn get_by_id(
    State(state): State<ServerState>,
    Path(id): Path<i64>,
) -> AppResult<Json<MarketingGroupDetail>> {
    let group = marketing_group::find_by_id(&state.pool, id)
        .await?
        .ok_or_else(|| crate::utils::AppError::not_found(format!("Marketing group {}", id)))?;

    let discount_rules = marketing_group::find_rules_by_group(&state.pool, id).await?;
    let activities = marketing_group::find_activities_by_group(&state.pool, id).await?;

    // 为每个 activity 加载 targets
    let mut stamp_activities = Vec::with_capacity(activities.len());
    for activity in activities {
        let stamp_targets =
            marketing_group::find_stamp_targets(&state.pool, activity.id).await?;
        let reward_targets =
            marketing_group::find_reward_targets(&state.pool, activity.id).await?;
        stamp_activities.push(StampActivityDetail {
            activity,
            stamp_targets,
            reward_targets,
        });
    }

    Ok(Json(MarketingGroupDetail {
        group,
        discount_rules,
        stamp_activities,
    }))
}

/// POST /api/marketing-groups - 创建营销组
pub async fn create(
    State(state): State<ServerState>,
    Extension(current_user): Extension<CurrentUser>,
    Json(payload): Json<shared::models::MarketingGroupCreate>,
) -> AppResult<Json<MarketingGroup>> {
    validate_group_create(&payload)?;
    validate_points_earn_rate(payload.points_earn_rate)?;

    let group = marketing_group::create(&state.pool, payload).await?;

    let id = group.id.to_string();

    audit_log!(
        state.audit_service,
        AuditAction::MarketingGroupCreated,
        "marketing_group", &id,
        operator_id = Some(current_user.id),
        operator_name = Some(current_user.display_name.clone()),
        details = create_snapshot(&group, "marketing_group")
    );

    state
        .broadcast_sync(RESOURCE, "created", &id, Some(&group))
        .await;

    Ok(Json(group))
}

/// PUT /api/marketing-groups/:id - 更新营销组
pub async fn update(
    State(state): State<ServerState>,
    Extension(current_user): Extension<CurrentUser>,
    Path(id): Path<i64>,
    Json(payload): Json<shared::models::MarketingGroupUpdate>,
) -> AppResult<Json<MarketingGroup>> {
    validate_group_update(&payload)?;
    validate_points_earn_rate(payload.points_earn_rate)?;

    let old_group = marketing_group::find_by_id(&state.pool, id)
        .await?
        .ok_or_else(|| crate::utils::AppError::not_found(format!("Marketing group {}", id)))?;

    let group = marketing_group::update(&state.pool, id, payload).await?;

    let id_str = id.to_string();

    audit_log!(
        state.audit_service,
        AuditAction::MarketingGroupUpdated,
        "marketing_group", &id_str,
        operator_id = Some(current_user.id),
        operator_name = Some(current_user.display_name.clone()),
        details = create_diff(&old_group, &group, "marketing_group")
    );

    state
        .broadcast_sync(RESOURCE, "updated", &id_str, Some(&group))
        .await;

    Ok(Json(group))
}

/// DELETE /api/marketing-groups/:id - 删除营销组（软删除）
pub async fn delete(
    State(state): State<ServerState>,
    Extension(current_user): Extension<CurrentUser>,
    Path(id): Path<i64>,
) -> AppResult<Json<bool>> {
    let name_for_audit = marketing_group::find_by_id(&state.pool, id)
        .await
        .ok()
        .flatten()
        .map(|g| g.name.clone())
        .unwrap_or_default();

    let result = marketing_group::delete(&state.pool, id).await?;

    let id_str = id.to_string();

    if result {
        audit_log!(
            state.audit_service,
            AuditAction::MarketingGroupDeleted,
            "marketing_group", &id_str,
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

// ── Discount Rule Handlers ──────────────────────────────────

/// POST /api/marketing-groups/:id/discount-rules - 创建折扣规则
pub async fn create_rule(
    State(state): State<ServerState>,
    Extension(current_user): Extension<CurrentUser>,
    Path(group_id): Path<i64>,
    Json(payload): Json<MgDiscountRuleCreate>,
) -> AppResult<Json<MgDiscountRule>> {
    validate_rule_create(&payload)?;
    validate_mg_adjustment(&payload.adjustment_type, payload.adjustment_value)?;

    let group = marketing_group::find_by_id(&state.pool, group_id)
        .await?
        .ok_or_else(|| crate::utils::AppError::not_found(format!("Marketing group {}", group_id)))?;

    let rule = marketing_group::create_rule(&state.pool, group_id, payload).await?;

    let id_str = rule.id.to_string();

    audit_log!(
        state.audit_service,
        AuditAction::MarketingGroupUpdated,
        "mg_discount_rule", &id_str,
        operator_id = Some(current_user.id),
        operator_name = Some(current_user.display_name.clone()),
        details = create_snapshot(&rule, "mg_discount_rule")
    );

    state
        .broadcast_sync(RESOURCE, "updated", &group_id.to_string(), Some(&group))
        .await;

    Ok(Json(rule))
}

/// PUT /api/marketing-groups/:id/discount-rules/:rule_id - 更新折扣规则
pub async fn update_rule(
    State(state): State<ServerState>,
    Extension(current_user): Extension<CurrentUser>,
    Path((group_id, rule_id)): Path<(i64, i64)>,
    Json(payload): Json<MgDiscountRuleUpdate>,
) -> AppResult<Json<MgDiscountRule>> {
    validate_rule_update(&payload)?;

    if let (Some(at), Some(av)) = (&payload.adjustment_type, payload.adjustment_value) {
        validate_mg_adjustment(at, av)?;
    } else if let Some(av) = payload.adjustment_value {
        // Basic validation when type is not changing
        if !av.is_finite() {
            return Err(AppError::validation("adjustment_value must be a finite number"));
        }
        if av < 0.0 {
            return Err(AppError::validation("adjustment_value must be non-negative"));
        }
    }

    let rule = marketing_group::update_rule(&state.pool, rule_id, payload).await?;

    let id_str = rule_id.to_string();

    audit_log!(
        state.audit_service,
        AuditAction::MarketingGroupUpdated,
        "mg_discount_rule", &id_str,
        operator_id = Some(current_user.id),
        operator_name = Some(current_user.display_name.clone()),
        details = create_snapshot(&rule, "mg_discount_rule")
    );

    // Broadcast parent group for store incremental sync
    if let Ok(Some(group)) = marketing_group::find_by_id(&state.pool, group_id).await {
        state
            .broadcast_sync(RESOURCE, "updated", &group_id.to_string(), Some(&group))
            .await;
    }

    Ok(Json(rule))
}

/// DELETE /api/marketing-groups/:id/discount-rules/:rule_id - 删除折扣规则（软删除）
pub async fn delete_rule(
    State(state): State<ServerState>,
    Extension(current_user): Extension<CurrentUser>,
    Path((group_id, rule_id)): Path<(i64, i64)>,
) -> AppResult<Json<bool>> {
    let result = marketing_group::delete_rule(&state.pool, rule_id).await?;

    if result {
        let id_str = rule_id.to_string();

        audit_log!(
            state.audit_service,
            AuditAction::MarketingGroupUpdated,
            "mg_discount_rule", &id_str,
            operator_id = Some(current_user.id),
            operator_name = Some(current_user.display_name.clone()),
            details = serde_json::json!({"action": "rule_deleted", "rule_id": rule_id})
        );

        if let Ok(Some(group)) = marketing_group::find_by_id(&state.pool, group_id).await {
            state
                .broadcast_sync(RESOURCE, "updated", &group_id.to_string(), Some(&group))
                .await;
        }
    }

    Ok(Json(result))
}

// ── Stamp Activity Handlers ─────────────────────────────────

/// POST /api/marketing-groups/:id/stamp-activities - 创建集章活动
pub async fn create_activity(
    State(state): State<ServerState>,
    Extension(current_user): Extension<CurrentUser>,
    Path(group_id): Path<i64>,
    Json(payload): Json<shared::models::StampActivityCreate>,
) -> AppResult<Json<StampActivityDetail>> {
    validate_activity_create(&payload)?;

    let group = marketing_group::find_by_id(&state.pool, group_id)
        .await?
        .ok_or_else(|| crate::utils::AppError::not_found(format!("Marketing group {}", group_id)))?;

    let detail = marketing_group::create_activity(&state.pool, group_id, payload).await?;

    let id_str = detail.activity.id.to_string();

    audit_log!(
        state.audit_service,
        AuditAction::MarketingGroupUpdated,
        "stamp_activity", &id_str,
        operator_id = Some(current_user.id),
        operator_name = Some(current_user.display_name.clone()),
        details = create_snapshot(&detail, "stamp_activity")
    );

    state
        .broadcast_sync(RESOURCE, "updated", &group_id.to_string(), Some(&group))
        .await;

    Ok(Json(detail))
}

/// PUT /api/marketing-groups/:id/stamp-activities/:activity_id - 更新集章活动
pub async fn update_activity(
    State(state): State<ServerState>,
    Extension(current_user): Extension<CurrentUser>,
    Path((group_id, activity_id)): Path<(i64, i64)>,
    Json(payload): Json<shared::models::StampActivityUpdate>,
) -> AppResult<Json<StampActivityDetail>> {
    validate_activity_update(&payload)?;

    let detail = marketing_group::update_activity(&state.pool, activity_id, payload).await?;

    let id_str = activity_id.to_string();

    audit_log!(
        state.audit_service,
        AuditAction::MarketingGroupUpdated,
        "stamp_activity", &id_str,
        operator_id = Some(current_user.id),
        operator_name = Some(current_user.display_name.clone()),
        details = create_snapshot(&detail, "stamp_activity")
    );

    // Broadcast parent group for store incremental sync
    if let Ok(Some(group)) = marketing_group::find_by_id(&state.pool, group_id).await {
        state
            .broadcast_sync(RESOURCE, "updated", &group_id.to_string(), Some(&group))
            .await;
    }

    Ok(Json(detail))
}

/// DELETE /api/marketing-groups/:id/stamp-activities/:activity_id - 删除集章活动（软删除）
pub async fn delete_activity(
    State(state): State<ServerState>,
    Extension(current_user): Extension<CurrentUser>,
    Path((group_id, activity_id)): Path<(i64, i64)>,
) -> AppResult<Json<bool>> {
    let result = marketing_group::delete_activity(&state.pool, activity_id).await?;

    if result {
        let id_str = activity_id.to_string();

        audit_log!(
            state.audit_service,
            AuditAction::MarketingGroupUpdated,
            "stamp_activity", &id_str,
            operator_id = Some(current_user.id),
            operator_name = Some(current_user.display_name.clone()),
            details = serde_json::json!({"action": "activity_deleted", "activity_id": activity_id})
        );

        if let Ok(Some(group)) = marketing_group::find_by_id(&state.pool, group_id).await {
            state
                .broadcast_sync(RESOURCE, "updated", &group_id.to_string(), Some(&group))
                .await;
        }
    }

    Ok(Json(result))
}
