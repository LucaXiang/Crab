//! System Issue Repository
//!
//! 系统问题表 CRUD — 本地异常检测和远程推送的问题。
//! 前端按 kind 渲染 i18n 对话框，远程广播则使用下发的文本。

use std::collections::HashMap;

use super::{BaseRepository, RepoResult};
use crate::db::models::serde_helpers;
use serde::{Deserialize, Serialize};
use surrealdb::engine::local::Db;
use surrealdb::Surreal;

/// SurrealDB system_issue 记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemIssueRow {
    #[serde(default, with = "serde_helpers::option_record_id")]
    pub id: Option<surrealdb::RecordId>,
    pub source: String,
    pub kind: String,
    pub blocking: bool,
    pub target: Option<String>,
    pub params: HashMap<String, String>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub options: Vec<String>,
    pub status: String,
    pub response: Option<String>,
    pub resolved_by: Option<String>,
    pub resolved_at: Option<i64>,
    pub created_at: i64,
}

/// 创建 system_issue 的请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSystemIssue {
    pub source: String,
    pub kind: String,
    pub blocking: bool,
    #[serde(default)]
    pub target: Option<String>,
    #[serde(default)]
    pub params: HashMap<String, String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub options: Vec<String>,
}

/// 解决 system_issue 的请求
#[derive(Debug, Clone, Deserialize)]
pub struct ResolveSystemIssue {
    pub id: String,
    pub response: String,
}

#[derive(Clone)]
pub struct SystemIssueRepository {
    base: BaseRepository,
}

impl SystemIssueRepository {
    pub fn new(db: Surreal<Db>) -> Self {
        Self {
            base: BaseRepository::new(db),
        }
    }

    /// 创建系统问题记录
    pub async fn create(&self, data: CreateSystemIssue) -> RepoResult<SystemIssueRow> {
        let now = shared::util::now_millis();
        let mut result = self
            .base
            .db()
            .query(
                r#"
                CREATE system_issue SET
                    source      = $source,
                    kind        = $kind,
                    blocking    = $blocking,
                    target      = $target,
                    params      = $params,
                    title       = $title,
                    description = $description,
                    options     = $options,
                    status      = "pending",
                    response    = NONE,
                    resolved_by = NONE,
                    resolved_at = NONE,
                    created_at  = $created_at
                "#,
            )
            .bind(("source", data.source))
            .bind(("kind", data.kind))
            .bind(("blocking", data.blocking))
            .bind(("target", data.target))
            .bind(("params", data.params))
            .bind(("title", data.title))
            .bind(("description", data.description))
            .bind(("options", data.options))
            .bind(("created_at", now))
            .await
            .map_err(super::RepoError::from)?;

        result
            .take::<Option<SystemIssueRow>>(0)
            .map_err(super::RepoError::from)?
            .ok_or_else(|| super::RepoError::Database("Failed to create system_issue".to_string()))
    }

    /// 查询所有 pending 状态的问题
    pub async fn find_pending(&self) -> RepoResult<Vec<SystemIssueRow>> {
        let rows: Vec<SystemIssueRow> = self
            .base
            .db()
            .query(r#"SELECT * FROM system_issue WHERE status = "pending" ORDER BY created_at ASC"#)
            .await
            .map_err(super::RepoError::from)?
            .take(0)
            .map_err(super::RepoError::from)?;
        Ok(rows)
    }

    /// 按 kind 查询 pending 状态的问题（用于去重）
    pub async fn find_pending_by_kind(&self, kind: &str) -> RepoResult<Vec<SystemIssueRow>> {
        let rows: Vec<SystemIssueRow> = self
            .base
            .db()
            .query(
                r#"SELECT * FROM system_issue WHERE status = "pending" AND kind = $kind"#,
            )
            .bind(("kind", kind.to_string()))
            .await
            .map_err(super::RepoError::from)?
            .take(0)
            .map_err(super::RepoError::from)?;
        Ok(rows)
    }

    /// 解决一个问题
    pub async fn resolve(
        &self,
        id: &str,
        response: &str,
        resolved_by: Option<&str>,
    ) -> RepoResult<SystemIssueRow> {
        let now = shared::util::now_millis();
        let thing: surrealdb::RecordId = id
            .parse()
            .map_err(|_| super::RepoError::Validation(format!("Invalid ID: {}", id)))?;

        let mut result = self
            .base
            .db()
            .query(
                r#"
                UPDATE $thing SET
                    status      = "resolved",
                    response    = $response,
                    resolved_by = $resolved_by,
                    resolved_at = $resolved_at
                RETURN AFTER
                "#,
            )
            .bind(("thing", thing))
            .bind(("response", response.to_string()))
            .bind(("resolved_by", resolved_by.map(|s| s.to_string())))
            .bind(("resolved_at", now))
            .await
            .map_err(super::RepoError::from)?;

        result
            .take::<Option<SystemIssueRow>>(0)
            .map_err(super::RepoError::from)?
            .ok_or_else(|| super::RepoError::NotFound(format!("system_issue {} not found", id)))
    }
}
