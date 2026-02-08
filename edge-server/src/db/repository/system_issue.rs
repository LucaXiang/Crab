//! System Issue Repository

use super::{RepoError, RepoResult};
use shared::models::{SystemIssue, SystemIssueCreate};
use sqlx::SqlitePool;

pub async fn create(pool: &SqlitePool, data: SystemIssueCreate) -> RepoResult<SystemIssue> {
    let now = shared::util::now_millis();
    let params_json =
        serde_json::to_string(&data.params).unwrap_or_else(|_| "{}".to_string());
    let options_json =
        serde_json::to_string(&data.options).unwrap_or_else(|_| "[]".to_string());

    let id = sqlx::query_scalar!(
        r#"INSERT INTO system_issue (source, kind, blocking, target, params, title, description, options, status, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'pending', ?9) RETURNING id as "id!""#,
        data.source,
        data.kind,
        data.blocking,
        data.target,
        params_json,
        data.title,
        data.description,
        options_json,
        now
    )
    .fetch_one(pool)
    .await?;

    find_by_id(pool, id)
        .await?
        .ok_or_else(|| RepoError::Database("Failed to create system_issue".into()))
}

pub async fn find_by_id(pool: &SqlitePool, id: i64) -> RepoResult<Option<SystemIssue>> {
    let issue = sqlx::query_as::<_, SystemIssue>(
        "SELECT id, source, kind, blocking, target, params, title, description, options, status, response, resolved_by, resolved_at, created_at FROM system_issue WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    Ok(issue)
}

pub async fn find_pending(pool: &SqlitePool) -> RepoResult<Vec<SystemIssue>> {
    let issues = sqlx::query_as::<_, SystemIssue>(
        "SELECT id, source, kind, blocking, target, params, title, description, options, status, response, resolved_by, resolved_at, created_at FROM system_issue WHERE status = 'pending' ORDER BY created_at ASC",
    )
    .fetch_all(pool)
    .await?;
    Ok(issues)
}

pub async fn find_pending_by_kind(pool: &SqlitePool, kind: &str) -> RepoResult<Vec<SystemIssue>> {
    let issues = sqlx::query_as::<_, SystemIssue>(
        "SELECT id, source, kind, blocking, target, params, title, description, options, status, response, resolved_by, resolved_at, created_at FROM system_issue WHERE status = 'pending' AND kind = ?",
    )
    .bind(kind)
    .fetch_all(pool)
    .await?;
    Ok(issues)
}

pub async fn resolve(
    pool: &SqlitePool,
    id: i64,
    response: &str,
    resolved_by: Option<&str>,
) -> RepoResult<SystemIssue> {
    let now = shared::util::now_millis();
    let rows = sqlx::query!(
        "UPDATE system_issue SET status = 'resolved', response = ?1, resolved_by = ?2, resolved_at = ?3 WHERE id = ?4",
        response,
        resolved_by,
        now,
        id
    )
    .execute(pool)
    .await?;

    if rows.rows_affected() == 0 {
        return Err(RepoError::NotFound(format!(
            "system_issue {id} not found"
        )));
    }
    find_by_id(pool, id)
        .await?
        .ok_or_else(|| RepoError::NotFound(format!("system_issue {id} not found")))
}
