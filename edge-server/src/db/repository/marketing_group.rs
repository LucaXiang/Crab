//! Marketing Group Repository

use super::{RepoError, RepoResult};
use shared::models::{
    MarketingGroup, MarketingGroupCreate, MarketingGroupUpdate, MgDiscountRule,
    MgDiscountRuleCreate, MgDiscountRuleUpdate, StampActivity, StampActivityCreate,
    StampActivityDetail, StampActivityUpdate, StampRewardTarget, StampTarget, StampTargetInput,
};
use sqlx::SqlitePool;

// ── MarketingGroup CRUD ──────────────────────────────────────

pub async fn find_all(pool: &SqlitePool) -> RepoResult<Vec<MarketingGroup>> {
    let rows = sqlx::query_as::<_, MarketingGroup>(
        "SELECT id, name, display_name, description, sort_order, points_earn_rate, is_active, created_at, updated_at FROM marketing_group WHERE is_active = 1 ORDER BY sort_order",
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn find_by_id(pool: &SqlitePool, id: i64) -> RepoResult<Option<MarketingGroup>> {
    let row = sqlx::query_as::<_, MarketingGroup>(
        "SELECT id, name, display_name, description, sort_order, points_earn_rate, is_active, created_at, updated_at FROM marketing_group WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

pub async fn create(pool: &SqlitePool, data: MarketingGroupCreate) -> RepoResult<MarketingGroup> {
    let now = shared::util::now_millis();
    let sort_order = data.sort_order.unwrap_or(0);
    let id = sqlx::query_scalar!(
        r#"INSERT INTO marketing_group (name, display_name, description, sort_order, points_earn_rate, is_active, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, 1, ?6, ?6) RETURNING id as "id!""#,
        data.name,
        data.display_name,
        data.description,
        sort_order,
        data.points_earn_rate,
        now
    )
    .fetch_one(pool)
    .await?;
    find_by_id(pool, id)
        .await?
        .ok_or_else(|| RepoError::Database("Failed to create marketing group".into()))
}

pub async fn update(
    pool: &SqlitePool,
    id: i64,
    data: MarketingGroupUpdate,
) -> RepoResult<MarketingGroup> {
    let now = shared::util::now_millis();
    let rows = sqlx::query!(
        "UPDATE marketing_group SET name = COALESCE(?1, name), display_name = COALESCE(?2, display_name), description = COALESCE(?3, description), sort_order = COALESCE(?4, sort_order), points_earn_rate = COALESCE(?5, points_earn_rate), is_active = COALESCE(?6, is_active), updated_at = ?7 WHERE id = ?8",
        data.name,
        data.display_name,
        data.description,
        data.sort_order,
        data.points_earn_rate,
        data.is_active,
        now,
        id
    )
    .execute(pool)
    .await?;
    if rows.rows_affected() == 0 {
        return Err(RepoError::NotFound(format!(
            "Marketing group {id} not found"
        )));
    }
    find_by_id(pool, id)
        .await?
        .ok_or_else(|| RepoError::NotFound(format!("Marketing group {id} not found")))
}

pub async fn delete(pool: &SqlitePool, id: i64) -> RepoResult<bool> {
    let now = shared::util::now_millis();
    let rows = sqlx::query!(
        "UPDATE marketing_group SET is_active = 0, updated_at = ? WHERE id = ? AND is_active = 1",
        now,
        id
    )
    .execute(pool)
    .await?;
    Ok(rows.rows_affected() > 0)
}

// ── MgDiscountRule CRUD ──────────────────────────────────────

pub async fn find_rules_by_group(
    pool: &SqlitePool,
    group_id: i64,
) -> RepoResult<Vec<MgDiscountRule>> {
    let rows = sqlx::query_as::<_, MgDiscountRule>(
        "SELECT id, marketing_group_id, name, display_name, receipt_name, product_scope, target_id, adjustment_type, adjustment_value, is_active, created_at, updated_at FROM mg_discount_rule WHERE marketing_group_id = ? ORDER BY created_at DESC",
    )
    .bind(group_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn find_active_rules_by_group(
    pool: &SqlitePool,
    group_id: i64,
) -> RepoResult<Vec<MgDiscountRule>> {
    let rows = sqlx::query_as::<_, MgDiscountRule>(
        "SELECT id, marketing_group_id, name, display_name, receipt_name, product_scope, target_id, adjustment_type, adjustment_value, is_active, created_at, updated_at FROM mg_discount_rule WHERE marketing_group_id = ? AND is_active = 1 ORDER BY created_at DESC",
    )
    .bind(group_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn create_rule(
    pool: &SqlitePool,
    group_id: i64,
    data: MgDiscountRuleCreate,
) -> RepoResult<MgDiscountRule> {
    let now = shared::util::now_millis();
    let id = sqlx::query_scalar!(
        r#"INSERT INTO mg_discount_rule (marketing_group_id, name, display_name, receipt_name, product_scope, target_id, adjustment_type, adjustment_value, is_active, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 1, ?9, ?9) RETURNING id as "id!""#,
        group_id,
        data.name,
        data.display_name,
        data.receipt_name,
        data.product_scope,
        data.target_id,
        data.adjustment_type,
        data.adjustment_value,
        now
    )
    .fetch_one(pool)
    .await?;

    let row = sqlx::query_as::<_, MgDiscountRule>(
        "SELECT id, marketing_group_id, name, display_name, receipt_name, product_scope, target_id, adjustment_type, adjustment_value, is_active, created_at, updated_at FROM mg_discount_rule WHERE id = ?",
    )
    .bind(id)
    .fetch_one(pool)
    .await?;
    Ok(row)
}

pub async fn update_rule(
    pool: &SqlitePool,
    rule_id: i64,
    data: MgDiscountRuleUpdate,
) -> RepoResult<MgDiscountRule> {
    let now = shared::util::now_millis();
    let rows = sqlx::query!(
        "UPDATE mg_discount_rule SET name = COALESCE(?1, name), display_name = COALESCE(?2, display_name), receipt_name = COALESCE(?3, receipt_name), product_scope = COALESCE(?4, product_scope), target_id = COALESCE(?5, target_id), adjustment_type = COALESCE(?6, adjustment_type), adjustment_value = COALESCE(?7, adjustment_value), is_active = COALESCE(?8, is_active), updated_at = ?9 WHERE id = ?10",
        data.name,
        data.display_name,
        data.receipt_name,
        data.product_scope,
        data.target_id,
        data.adjustment_type,
        data.adjustment_value,
        data.is_active,
        now,
        rule_id
    )
    .execute(pool)
    .await?;
    if rows.rows_affected() == 0 {
        return Err(RepoError::NotFound(format!(
            "MG discount rule {rule_id} not found"
        )));
    }
    let row = sqlx::query_as::<_, MgDiscountRule>(
        "SELECT id, marketing_group_id, name, display_name, receipt_name, product_scope, target_id, adjustment_type, adjustment_value, is_active, created_at, updated_at FROM mg_discount_rule WHERE id = ?",
    )
    .bind(rule_id)
    .fetch_one(pool)
    .await?;
    Ok(row)
}

pub async fn delete_rule(pool: &SqlitePool, rule_id: i64) -> RepoResult<bool> {
    let now = shared::util::now_millis();
    let rows = sqlx::query!(
        "UPDATE mg_discount_rule SET is_active = 0, updated_at = ? WHERE id = ? AND is_active = 1",
        now,
        rule_id
    )
    .execute(pool)
    .await?;
    Ok(rows.rows_affected() > 0)
}

// ── StampActivity CRUD ───────────────────────────────────────

pub async fn find_activities_by_group(
    pool: &SqlitePool,
    group_id: i64,
) -> RepoResult<Vec<StampActivity>> {
    let rows = sqlx::query_as::<_, StampActivity>(
        "SELECT id, marketing_group_id, name, display_name, stamps_required, reward_quantity, reward_strategy, designated_product_id, is_cyclic, is_active, created_at, updated_at FROM stamp_activity WHERE marketing_group_id = ? ORDER BY created_at DESC",
    )
    .bind(group_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn find_active_activities_by_group(
    pool: &SqlitePool,
    group_id: i64,
) -> RepoResult<Vec<StampActivity>> {
    let rows = sqlx::query_as::<_, StampActivity>(
        "SELECT id, marketing_group_id, name, display_name, stamps_required, reward_quantity, reward_strategy, designated_product_id, is_cyclic, is_active, created_at, updated_at FROM stamp_activity WHERE marketing_group_id = ? AND is_active = 1 ORDER BY created_at DESC",
    )
    .bind(group_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn create_activity(
    pool: &SqlitePool,
    group_id: i64,
    data: StampActivityCreate,
) -> RepoResult<StampActivityDetail> {
    let now = shared::util::now_millis();
    let reward_quantity = data.reward_quantity.unwrap_or(1);
    let reward_strategy = data
        .reward_strategy
        .unwrap_or(shared::models::RewardStrategy::Economizador);
    let is_cyclic = data.is_cyclic.unwrap_or(true);

    let mut tx = pool.begin().await?;

    let id = sqlx::query_scalar!(
        r#"INSERT INTO stamp_activity (marketing_group_id, name, display_name, stamps_required, reward_quantity, reward_strategy, designated_product_id, is_cyclic, is_active, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 1, ?9, ?9) RETURNING id as "id!""#,
        group_id,
        data.name,
        data.display_name,
        data.stamps_required,
        reward_quantity,
        reward_strategy,
        data.designated_product_id,
        is_cyclic,
        now
    )
    .fetch_one(&mut *tx)
    .await?;

    replace_stamp_targets(&mut tx, id, &data.stamp_targets).await?;
    replace_reward_targets(&mut tx, id, &data.reward_targets).await?;

    tx.commit().await?;

    load_activity_detail(pool, id).await
}

pub async fn update_activity(
    pool: &SqlitePool,
    activity_id: i64,
    data: StampActivityUpdate,
) -> RepoResult<StampActivityDetail> {
    let now = shared::util::now_millis();

    let mut tx = pool.begin().await?;

    let rows = sqlx::query!(
        "UPDATE stamp_activity SET name = COALESCE(?1, name), display_name = COALESCE(?2, display_name), stamps_required = COALESCE(?3, stamps_required), reward_quantity = COALESCE(?4, reward_quantity), reward_strategy = COALESCE(?5, reward_strategy), designated_product_id = COALESCE(?6, designated_product_id), is_cyclic = COALESCE(?7, is_cyclic), is_active = COALESCE(?8, is_active), updated_at = ?9 WHERE id = ?10",
        data.name,
        data.display_name,
        data.stamps_required,
        data.reward_quantity,
        data.reward_strategy,
        data.designated_product_id,
        data.is_cyclic,
        data.is_active,
        now,
        activity_id
    )
    .execute(&mut *tx)
    .await?;
    if rows.rows_affected() == 0 {
        return Err(RepoError::NotFound(format!(
            "Stamp activity {activity_id} not found"
        )));
    }

    if let Some(ref targets) = data.stamp_targets {
        replace_stamp_targets(&mut tx, activity_id, targets).await?;
    }
    if let Some(ref targets) = data.reward_targets {
        replace_reward_targets(&mut tx, activity_id, targets).await?;
    }

    tx.commit().await?;

    load_activity_detail(pool, activity_id).await
}

pub async fn delete_activity(pool: &SqlitePool, activity_id: i64) -> RepoResult<bool> {
    let now = shared::util::now_millis();
    let rows = sqlx::query!(
        "UPDATE stamp_activity SET is_active = 0, updated_at = ? WHERE id = ? AND is_active = 1",
        now,
        activity_id
    )
    .execute(pool)
    .await?;
    Ok(rows.rows_affected() > 0)
}

// ── Stamp Targets (internal helpers) ─────────────────────────

async fn replace_stamp_targets(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    activity_id: i64,
    targets: &[StampTargetInput],
) -> RepoResult<()> {
    sqlx::query!("DELETE FROM stamp_target WHERE stamp_activity_id = ?", activity_id)
        .execute(&mut **tx)
        .await?;
    for t in targets {
        sqlx::query!(
            "INSERT INTO stamp_target (stamp_activity_id, target_type, target_id) VALUES (?1, ?2, ?3)",
            activity_id,
            t.target_type,
            t.target_id
        )
        .execute(&mut **tx)
        .await?;
    }
    Ok(())
}

async fn replace_reward_targets(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    activity_id: i64,
    targets: &[StampTargetInput],
) -> RepoResult<()> {
    sqlx::query!(
        "DELETE FROM stamp_reward_target WHERE stamp_activity_id = ?",
        activity_id
    )
    .execute(&mut **tx)
    .await?;
    for t in targets {
        sqlx::query!(
            "INSERT INTO stamp_reward_target (stamp_activity_id, target_type, target_id) VALUES (?1, ?2, ?3)",
            activity_id,
            t.target_type,
            t.target_id
        )
        .execute(&mut **tx)
        .await?;
    }
    Ok(())
}

pub async fn find_stamp_targets(
    pool: &SqlitePool,
    activity_id: i64,
) -> RepoResult<Vec<StampTarget>> {
    let rows = sqlx::query_as::<_, StampTarget>(
        "SELECT id, stamp_activity_id, target_type, target_id FROM stamp_target WHERE stamp_activity_id = ?",
    )
    .bind(activity_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn find_reward_targets(
    pool: &SqlitePool,
    activity_id: i64,
) -> RepoResult<Vec<StampRewardTarget>> {
    let rows = sqlx::query_as::<_, StampRewardTarget>(
        "SELECT id, stamp_activity_id, target_type, target_id FROM stamp_reward_target WHERE stamp_activity_id = ?",
    )
    .bind(activity_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

// ── Internal helpers ─────────────────────────────────────────

async fn load_activity_detail(
    pool: &SqlitePool,
    activity_id: i64,
) -> RepoResult<StampActivityDetail> {
    let activity = sqlx::query_as::<_, StampActivity>(
        "SELECT id, marketing_group_id, name, display_name, stamps_required, reward_quantity, reward_strategy, designated_product_id, is_cyclic, is_active, created_at, updated_at FROM stamp_activity WHERE id = ?",
    )
    .bind(activity_id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| RepoError::NotFound(format!("Stamp activity {activity_id} not found")))?;

    let stamp_targets = find_stamp_targets(pool, activity_id).await?;
    let reward_targets = find_reward_targets(pool, activity_id).await?;

    Ok(StampActivityDetail {
        activity,
        stamp_targets,
        reward_targets,
    })
}
