//! Member Repository

use super::{RepoError, RepoResult};
use shared::models::{Member, MemberCreate, MemberUpdate, MemberWithGroup};
use sqlx::SqlitePool;

const MEMBER_WITH_GROUP_SELECT: &str = "SELECT m.id, m.name, m.phone, m.card_number, m.marketing_group_id, mg.display_name as marketing_group_name, m.birthday, m.email, m.points_balance, m.total_spent, m.notes, m.is_active, m.created_at, m.updated_at FROM member m JOIN marketing_group mg ON m.marketing_group_id = mg.id";

pub async fn find_all(pool: &SqlitePool) -> RepoResult<Vec<MemberWithGroup>> {
    let sql = format!(
        "{} WHERE m.is_active = 1 ORDER BY m.created_at DESC",
        MEMBER_WITH_GROUP_SELECT
    );
    let rows = sqlx::query_as::<_, MemberWithGroup>(&sql)
        .fetch_all(pool)
        .await?;
    Ok(rows)
}

pub async fn find_by_id(pool: &SqlitePool, id: i64) -> RepoResult<Option<MemberWithGroup>> {
    let sql = format!("{} WHERE m.id = ?", MEMBER_WITH_GROUP_SELECT);
    let row = sqlx::query_as::<_, MemberWithGroup>(&sql)
        .bind(id)
        .fetch_optional(pool)
        .await?;
    Ok(row)
}

pub async fn search(pool: &SqlitePool, query: &str) -> RepoResult<Vec<MemberWithGroup>> {
    let pattern = format!("%{query}%");
    let sql = format!(
        "{} WHERE m.is_active = 1 AND (m.phone LIKE ?1 OR m.card_number LIKE ?1 OR m.name LIKE ?1) ORDER BY m.created_at DESC",
        MEMBER_WITH_GROUP_SELECT
    );
    let rows = sqlx::query_as::<_, MemberWithGroup>(&sql)
        .bind(&pattern)
        .fetch_all(pool)
        .await?;
    Ok(rows)
}

pub async fn create(pool: &SqlitePool, data: MemberCreate) -> RepoResult<MemberWithGroup> {
    let now = shared::util::now_millis();
    let id = shared::util::snowflake_id();
    sqlx::query!(
        "INSERT INTO member (id, name, phone, card_number, marketing_group_id, birthday, email, notes, is_active, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 1, ?9, ?9)",
        id,
        data.name,
        data.phone,
        data.card_number,
        data.marketing_group_id,
        data.birthday,
        data.email,
        data.notes,
        now
    )
    .execute(pool)
    .await?;
    find_by_id(pool, id)
        .await?
        .ok_or_else(|| RepoError::Database("Failed to create member".into()))
}

pub async fn update(pool: &SqlitePool, id: i64, data: MemberUpdate) -> RepoResult<MemberWithGroup> {
    let now = shared::util::now_millis();
    let rows = sqlx::query!(
        "UPDATE member SET name = COALESCE(?1, name), phone = COALESCE(?2, phone), card_number = COALESCE(?3, card_number), marketing_group_id = COALESCE(?4, marketing_group_id), birthday = COALESCE(?5, birthday), email = COALESCE(?6, email), notes = COALESCE(?7, notes), is_active = COALESCE(?8, is_active), updated_at = ?9 WHERE id = ?10",
        data.name,
        data.phone,
        data.card_number,
        data.marketing_group_id,
        data.birthday,
        data.email,
        data.notes,
        data.is_active,
        now,
        id
    )
    .execute(pool)
    .await?;
    if rows.rows_affected() == 0 {
        return Err(RepoError::NotFound(format!("Member {id} not found")));
    }
    find_by_id(pool, id)
        .await?
        .ok_or_else(|| RepoError::NotFound(format!("Member {id} not found")))
}

pub async fn delete(pool: &SqlitePool, id: i64) -> RepoResult<bool> {
    let now = shared::util::now_millis();
    let rows = sqlx::query!(
        "UPDATE member SET is_active = 0, updated_at = ? WHERE id = ? AND is_active = 1",
        now,
        id
    )
    .execute(pool)
    .await?;
    Ok(rows.rows_affected() > 0)
}

pub async fn find_member_by_id(pool: &SqlitePool, id: i64) -> RepoResult<Option<Member>> {
    let row = sqlx::query_as::<_, Member>(
        "SELECT id, name, phone, card_number, marketing_group_id, birthday, email, points_balance, total_spent, notes, is_active, created_at, updated_at FROM member WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

/// Atomically update member stats after order completion (total_spent + points_balance)
pub async fn update_member_stats(
    pool: &SqlitePool,
    member_id: i64,
    spent_amount: f64,
    points_earned: i64,
) -> RepoResult<()> {
    let now = shared::util::now_millis();
    sqlx::query!(
        "UPDATE member SET total_spent = total_spent + ?1, points_balance = points_balance + ?2, updated_at = ?3 WHERE id = ?4 AND is_active = 1",
        spent_amount,
        points_earned,
        now,
        member_id
    )
    .execute(pool)
    .await?;
    Ok(())
}
