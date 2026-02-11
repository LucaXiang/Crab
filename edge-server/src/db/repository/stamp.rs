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
        "SELECT sa.id as stamp_activity_id, sa.name as stamp_activity_name, sa.stamps_required, COALESCE(msp.current_stamps, 0) as current_stamps, COALESCE(msp.completed_cycles, 0) as completed_cycles, CASE WHEN COALESCE(msp.current_stamps, 0) >= sa.stamps_required THEN 1 ELSE 0 END as is_redeemable, sa.is_cyclic, sa.reward_strategy, sa.reward_quantity, sa.designated_product_id FROM stamp_activity sa LEFT JOIN member_stamp_progress msp ON sa.id = msp.stamp_activity_id AND msp.member_id = ?1 WHERE sa.marketing_group_id IN (SELECT marketing_group_id FROM member WHERE id = ?1) AND sa.is_active = 1",
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
    stamps_required: i32,
    is_cyclic: bool,
    timestamp: i64,
) -> RepoResult<MemberStampProgress> {
    if is_cyclic {
        // Cyclic: subtract stamps_required (keep excess for next cycle), increment completed_cycles
        sqlx::query!(
            "UPDATE member_stamp_progress SET current_stamps = MAX(0, current_stamps - ?1), completed_cycles = completed_cycles + 1, updated_at = ?2 WHERE member_id = ?3 AND stamp_activity_id = ?4",
            stamps_required,
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

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    /// Create an in-memory SQLite pool with the required schema for stamp tests.
    async fn test_pool() -> SqlitePool {
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();

        sqlx::query(
            "CREATE TABLE marketing_group (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL,
                display_name TEXT NOT NULL,
                is_active INTEGER NOT NULL DEFAULT 1,
                created_at INTEGER NOT NULL DEFAULT 0,
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        )
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
            "CREATE TABLE member (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL,
                marketing_group_id INTEGER NOT NULL,
                is_active INTEGER NOT NULL DEFAULT 1,
                created_at INTEGER NOT NULL DEFAULT 0,
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        )
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
            "CREATE TABLE stamp_activity (
                id INTEGER PRIMARY KEY,
                marketing_group_id INTEGER NOT NULL,
                name TEXT NOT NULL,
                display_name TEXT NOT NULL,
                stamps_required INTEGER NOT NULL,
                reward_quantity INTEGER NOT NULL DEFAULT 1,
                reward_strategy TEXT NOT NULL DEFAULT 'ECONOMIZADOR',
                designated_product_id INTEGER,
                is_cyclic INTEGER NOT NULL DEFAULT 1,
                is_active INTEGER NOT NULL DEFAULT 1,
                created_at INTEGER NOT NULL DEFAULT 0,
                updated_at INTEGER NOT NULL DEFAULT 0
            )",
        )
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
            "CREATE TABLE member_stamp_progress (
                id INTEGER PRIMARY KEY,
                member_id INTEGER NOT NULL,
                stamp_activity_id INTEGER NOT NULL,
                current_stamps INTEGER NOT NULL DEFAULT 0,
                completed_cycles INTEGER NOT NULL DEFAULT 0,
                last_stamp_at INTEGER,
                updated_at INTEGER NOT NULL DEFAULT 0,
                UNIQUE(member_id, stamp_activity_id)
            )",
        )
        .execute(&pool)
        .await
        .unwrap();

        // Seed: marketing_group + member + stamp_activity
        sqlx::query("INSERT INTO marketing_group (id, name, display_name) VALUES (1, 'VIP', 'VIP')")
            .execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO member (id, name, marketing_group_id) VALUES (1, 'Alice', 1)")
            .execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO stamp_activity (id, marketing_group_id, name, display_name, stamps_required, is_cyclic) VALUES (1, 1, 'coffee', 'Coffee Card', 10, 1)")
            .execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO stamp_activity (id, marketing_group_id, name, display_name, stamps_required, is_cyclic) VALUES (2, 1, 'tea', 'Tea Card', 5, 0)")
            .execute(&pool).await.unwrap();

        pool
    }

    #[tokio::test]
    async fn test_add_stamps_basic() {
        let pool = test_pool().await;
        let p = add_stamps(&pool, 1, 1, 5, 1000).await.unwrap();
        assert_eq!(p.current_stamps, 5);
        assert_eq!(p.completed_cycles, 0);
    }

    #[tokio::test]
    async fn test_add_stamps_accumulates() {
        let pool = test_pool().await;
        add_stamps(&pool, 1, 1, 5, 1000).await.unwrap();
        let p = add_stamps(&pool, 1, 1, 7, 2000).await.unwrap();
        assert_eq!(p.current_stamps, 12);
    }

    #[tokio::test]
    async fn test_redeem_cyclic_preserves_overflow() {
        let pool = test_pool().await;
        // 27 stamps, require 10, cyclic → 27-10 = 17
        add_stamps(&pool, 1, 1, 27, 1000).await.unwrap();
        let p = redeem(&pool, 1, 1, 10, true, 2000).await.unwrap();
        assert_eq!(p.current_stamps, 17);
        assert_eq!(p.completed_cycles, 1);
    }

    #[tokio::test]
    async fn test_redeem_cyclic_exact() {
        let pool = test_pool().await;
        // 10 stamps, require 10 → 0
        add_stamps(&pool, 1, 1, 10, 1000).await.unwrap();
        let p = redeem(&pool, 1, 1, 10, true, 2000).await.unwrap();
        assert_eq!(p.current_stamps, 0);
        assert_eq!(p.completed_cycles, 1);
    }

    #[tokio::test]
    async fn test_redeem_cyclic_multiple_cycles() {
        let pool = test_pool().await;
        // 35 stamps → redeem 3 times → 35-10-10-10 = 5, 3 cycles
        add_stamps(&pool, 1, 1, 35, 1000).await.unwrap();
        redeem(&pool, 1, 1, 10, true, 2000).await.unwrap();
        redeem(&pool, 1, 1, 10, true, 3000).await.unwrap();
        let p = redeem(&pool, 1, 1, 10, true, 4000).await.unwrap();
        assert_eq!(p.current_stamps, 5);
        assert_eq!(p.completed_cycles, 3);
    }

    #[tokio::test]
    async fn test_redeem_noncyclic_keeps_stamps() {
        let pool = test_pool().await;
        // Non-cyclic (activity 2, requires 5): stamps stay after redeem
        add_stamps(&pool, 1, 2, 8, 1000).await.unwrap();
        let p = redeem(&pool, 1, 2, 5, false, 2000).await.unwrap();
        assert_eq!(p.current_stamps, 8); // stamps untouched
        assert_eq!(p.completed_cycles, 1);
    }

    #[tokio::test]
    async fn test_redeem_cyclic_floor_at_zero() {
        let pool = test_pool().await;
        // Edge: stamps_required > current (shouldn't happen, but MAX(0,...) protects)
        add_stamps(&pool, 1, 1, 3, 1000).await.unwrap();
        let p = redeem(&pool, 1, 1, 10, true, 2000).await.unwrap();
        assert_eq!(p.current_stamps, 0); // MAX(0, 3-10) = 0
        assert_eq!(p.completed_cycles, 1);
    }

    #[tokio::test]
    async fn test_add_then_redeem_full_cycle() {
        let pool = test_pool().await;
        // Simulate real flow: earn 27 stamps from order, then redeem (requires 10)
        let p = add_stamps(&pool, 1, 1, 27, 1000).await.unwrap();
        assert_eq!(p.current_stamps, 27);

        let p = redeem(&pool, 1, 1, 10, true, 2000).await.unwrap();
        assert_eq!(p.current_stamps, 17);
        assert_eq!(p.completed_cycles, 1);
    }

    #[tokio::test]
    async fn test_double_add_stamps_not_idempotent() {
        // If track_stamps_on_completion is called twice (crash recovery), stamps double!
        // This test documents the current behavior — it's a known issue.
        let pool = test_pool().await;
        add_stamps(&pool, 1, 1, 10, 1000).await.unwrap();
        let p = add_stamps(&pool, 1, 1, 10, 1000).await.unwrap(); // duplicate call
        assert_eq!(p.current_stamps, 20); // BUG: should be 10 if idempotent
    }

    #[tokio::test]
    async fn test_double_redeem_decrements_twice() {
        // If redeem is called twice (crash recovery), stamps double-decremented!
        let pool = test_pool().await;
        add_stamps(&pool, 1, 1, 20, 1000).await.unwrap();
        redeem(&pool, 1, 1, 10, true, 2000).await.unwrap();
        let p = redeem(&pool, 1, 1, 10, true, 2000).await.unwrap(); // duplicate
        assert_eq!(p.current_stamps, 0); // 20-10-10 = 0
        assert_eq!(p.completed_cycles, 2); // BUG: should be 1 if idempotent
    }

    #[tokio::test]
    async fn test_ensure_progress_idempotent() {
        let pool = test_pool().await;
        let p1 = ensure_progress(&pool, 1, 1).await.unwrap();
        assert_eq!(p1.current_stamps, 0);
        // Call again → no error, same result
        let p2 = ensure_progress(&pool, 1, 1).await.unwrap();
        assert_eq!(p2.current_stamps, 0);
    }

    #[tokio::test]
    async fn test_separate_activities_independent() {
        let pool = test_pool().await;
        add_stamps(&pool, 1, 1, 15, 1000).await.unwrap();
        add_stamps(&pool, 1, 2, 3, 1000).await.unwrap();

        let p1 = find_progress(&pool, 1, 1).await.unwrap().unwrap();
        let p2 = find_progress(&pool, 1, 2).await.unwrap().unwrap();
        assert_eq!(p1.current_stamps, 15);
        assert_eq!(p2.current_stamps, 3);

        // Redeem activity 1 doesn't affect activity 2
        redeem(&pool, 1, 1, 10, true, 2000).await.unwrap();
        let p1 = find_progress(&pool, 1, 1).await.unwrap().unwrap();
        let p2 = find_progress(&pool, 1, 2).await.unwrap().unwrap();
        assert_eq!(p1.current_stamps, 5);
        assert_eq!(p2.current_stamps, 3);
    }
}
