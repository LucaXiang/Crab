//! Stamp Progress Repository

use super::{RepoError, RepoResult};
use shared::models::{MemberStampProgress, MemberStampProgressDetail};
use sqlx::SqlitePool;

pub async fn find_progress_by_member(
    pool: &SqlitePool,
    member_id: i64,
) -> RepoResult<Vec<MemberStampProgress>> {
    let rows = sqlx::query_as::<_, MemberStampProgress>(
        "SELECT id, member_id, stamp_activity_id, current_stamps, completed_cycles, last_stamp_at, updated_at FROM member_stamp_progress WHERE member_id = ?",
    )
    .bind(member_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn find_progress(
    pool: &SqlitePool,
    member_id: i64,
    activity_id: i64,
) -> RepoResult<Option<MemberStampProgress>> {
    let row = sqlx::query_as::<_, MemberStampProgress>(
        "SELECT id, member_id, stamp_activity_id, current_stamps, completed_cycles, last_stamp_at, updated_at FROM member_stamp_progress WHERE member_id = ? AND stamp_activity_id = ?",
    )
    .bind(member_id)
    .bind(activity_id)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

pub async fn find_progress_details_by_member(
    pool: &SqlitePool,
    member_id: i64,
) -> RepoResult<Vec<MemberStampProgressDetail>> {
    let rows = sqlx::query_as::<_, MemberStampProgressDetail>(
        "SELECT sa.id as stamp_activity_id, sa.name as stamp_activity_name, sa.stamps_required, COALESCE(msp.current_stamps, 0) as current_stamps, COALESCE(msp.completed_cycles, 0) as completed_cycles, CASE WHEN COALESCE(msp.current_stamps, 0) >= sa.stamps_required THEN 1 ELSE 0 END as is_redeemable, sa.is_cyclic FROM stamp_activity sa LEFT JOIN member_stamp_progress msp ON sa.id = msp.stamp_activity_id AND msp.member_id = ?1 WHERE sa.marketing_group_id IN (SELECT marketing_group_id FROM member WHERE id = ?1) AND sa.is_active = 1",
    )
    .bind(member_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn add_stamps(
    pool: &SqlitePool,
    member_id: i64,
    activity_id: i64,
    count: i32,
    timestamp: i64,
) -> RepoResult<MemberStampProgress> {
    // Ensure progress row exists
    ensure_progress(pool, member_id, activity_id).await?;

    sqlx::query!(
        "UPDATE member_stamp_progress SET current_stamps = current_stamps + ?1, last_stamp_at = ?2, updated_at = ?2 WHERE member_id = ?3 AND stamp_activity_id = ?4",
        count,
        timestamp,
        member_id,
        activity_id
    )
    .execute(pool)
    .await?;

    find_progress(pool, member_id, activity_id)
        .await?
        .ok_or_else(|| RepoError::Database("Failed to update stamp progress".into()))
}

pub async fn redeem(
    pool: &SqlitePool,
    member_id: i64,
    activity_id: i64,
    is_cyclic: bool,
    timestamp: i64,
) -> RepoResult<MemberStampProgress> {
    if is_cyclic {
        // Cyclic: reset current_stamps to 0, increment completed_cycles
        sqlx::query!(
            "UPDATE member_stamp_progress SET current_stamps = 0, completed_cycles = completed_cycles + 1, updated_at = ?1 WHERE member_id = ?2 AND stamp_activity_id = ?3",
            timestamp,
            member_id,
            activity_id
        )
        .execute(pool)
        .await?;
    } else {
        // Non-cyclic: just increment completed_cycles (stamps stay for record)
        sqlx::query!(
            "UPDATE member_stamp_progress SET completed_cycles = completed_cycles + 1, updated_at = ?1 WHERE member_id = ?2 AND stamp_activity_id = ?3",
            timestamp,
            member_id,
            activity_id
        )
        .execute(pool)
        .await?;
    }

    find_progress(pool, member_id, activity_id)
        .await?
        .ok_or_else(|| RepoError::Database("Failed to redeem stamp progress".into()))
}

pub async fn ensure_progress(
    pool: &SqlitePool,
    member_id: i64,
    activity_id: i64,
) -> RepoResult<MemberStampProgress> {
    let now = shared::util::now_millis();
    // INSERT OR IGNORE: only inserts if (member_id, stamp_activity_id) pair doesn't exist
    sqlx::query!(
        "INSERT OR IGNORE INTO member_stamp_progress (member_id, stamp_activity_id, current_stamps, completed_cycles, updated_at) VALUES (?1, ?2, 0, 0, ?3)",
        member_id,
        activity_id,
        now
    )
    .execute(pool)
    .await?;

    find_progress(pool, member_id, activity_id)
        .await?
        .ok_or_else(|| RepoError::Database("Failed to ensure stamp progress".into()))
}
