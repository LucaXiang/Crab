//! Member Repository

use super::{RepoError, RepoResult};
use shared::models::{Member, MemberCreate, MemberUpdate, MemberWithGroup};
use sqlx::SqlitePool;

const MEMBER_WITH_GROUP_SELECT: &str = "SELECT m.id, m.name, m.phone, m.card_number, m.marketing_group_id, mg.display_name as marketing_group_name, m.birthday, m.points_balance, m.notes, m.is_active, m.created_at, m.updated_at FROM member m JOIN marketing_group mg ON m.marketing_group_id = mg.id";

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
    let id = sqlx::query_scalar!(
        r#"INSERT INTO member (name, phone, card_number, marketing_group_id, birthday, notes, is_active, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 1, ?7, ?7) RETURNING id as "id!""#,
        data.name,
        data.phone,
        data.card_number,
        data.marketing_group_id,
        data.birthday,
        data.notes,
        now
    )
    .fetch_one(pool)
    .await?;
    find_by_id(pool, id)
        .await?
        .ok_or_else(|| RepoError::Database("Failed to create member".into()))
}

pub async fn update(
    pool: &SqlitePool,
    id: i64,
    data: MemberUpdate,
) -> RepoResult<MemberWithGroup> {
    let now = shared::util::now_millis();
    let rows = sqlx::query!(
        "UPDATE member SET name = COALESCE(?1, name), phone = COALESCE(?2, phone), card_number = COALESCE(?3, card_number), marketing_group_id = COALESCE(?4, marketing_group_id), birthday = COALESCE(?5, birthday), notes = COALESCE(?6, notes), is_active = COALESCE(?7, is_active), updated_at = ?8 WHERE id = ?9",
        data.name,
        data.phone,
        data.card_number,
        data.marketing_group_id,
        data.birthday,
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
        "SELECT id, name, phone, card_number, marketing_group_id, birthday, points_balance, notes, is_active, created_at, updated_at FROM member WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}
