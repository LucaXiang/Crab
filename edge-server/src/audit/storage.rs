//! 审计日志 SurrealDB 存储层
//!
//! Append-only 设计，没有任何删除/更新接口。
//! SHA256 哈希链确保防篡改。

use std::sync::Arc;
use sha2::{Digest, Sha256};
use surrealdb::engine::local::Db;
use surrealdb::Surreal;
use thiserror::Error;

use super::types::{AuditAction, AuditChainBreak, AuditChainVerification, AuditEntry, AuditQuery, ChainBreakKind};

/// 存储错误
#[derive(Debug, Error)]
pub enum AuditStorageError {
    #[error("Database error: {0}")]
    Database(String),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

impl From<surrealdb::Error> for AuditStorageError {
    fn from(err: surrealdb::Error) -> Self {
        AuditStorageError::Database(err.to_string())
    }
}

pub type AuditStorageResult<T> = Result<T, AuditStorageError>;

impl From<AuditStorageError> for shared::error::AppError {
    fn from(err: AuditStorageError) -> Self {
        shared::error::AppError::internal(err.to_string())
    }
}

/// SurrealDB 反序列化用（包含 SurrealDB record id）
#[derive(Debug, Clone, serde::Deserialize)]
struct AuditRecord {
    #[allow(dead_code)]
    id: surrealdb::RecordId,
    sequence: u64,
    timestamp: i64,
    action: AuditAction,
    resource_type: String,
    resource_id: String,
    operator_id: Option<String>,
    operator_name: Option<String>,
    details: serde_json::Value,
    prev_hash: String,
    curr_hash: String,
}

impl From<AuditRecord> for AuditEntry {
    fn from(r: AuditRecord) -> Self {
        AuditEntry {
            id: r.sequence,
            timestamp: r.timestamp,
            action: r.action,
            resource_type: r.resource_type,
            resource_id: r.resource_id,
            operator_id: r.operator_id,
            operator_name: r.operator_name,
            details: r.details,
            prev_hash: r.prev_hash,
            curr_hash: r.curr_hash,
        }
    }
}

/// 查询最后一条记录的序列号和哈希
#[derive(Debug, serde::Deserialize)]
struct LastEntry {
    sequence: u64,
    curr_hash: String,
}

/// COUNT 结果
#[derive(Debug, serde::Deserialize)]
struct CountResult {
    total: u64,
}

/// 插入用结构（不含 SurrealDB id）
#[derive(Debug, serde::Serialize)]
struct AuditInsert {
    sequence: u64,
    timestamp: i64,
    action: AuditAction,
    resource_type: String,
    resource_id: String,
    operator_id: Option<String>,
    operator_name: Option<String>,
    details: serde_json::Value,
    prev_hash: String,
    curr_hash: String,
}

/// 审计日志存储 (SurrealDB)
///
/// Append-only 设计：
/// - 仅提供 `append` 和 `query` 方法
/// - 没有 delete/update 接口
/// - SHA256 哈希链确保完整性
#[derive(Clone)]
pub struct AuditStorage {
    db: Surreal<Db>,
    /// 序列化所有 append 操作，防止 read-modify-write 竞争
    append_lock: Arc<tokio::sync::Mutex<()>>,
}

impl AuditStorage {
    pub fn new(db: Surreal<Db>) -> Self {
        Self {
            db,
            append_lock: Arc::new(tokio::sync::Mutex::new(())),
        }
    }

    /// 追加一条审计日志
    ///
    /// 1. 查询当前最大序列号和 last_hash
    /// 2. 计算新条目的哈希
    /// 3. 写入条目
    pub async fn append(
        &self,
        action: AuditAction,
        resource_type: String,
        resource_id: String,
        operator_id: Option<String>,
        operator_name: Option<String>,
        details: serde_json::Value,
    ) -> AuditStorageResult<AuditEntry> {
        // 序列化：防止并发 append 导致 sequence 冲突
        let _guard = self.append_lock.lock().await;

        // 1. 读取当前最大序列号和 last_hash
        let mut result = self
            .db
            .query("SELECT sequence, curr_hash FROM audit_log ORDER BY sequence DESC LIMIT 1")
            .await?;
        let last: Vec<LastEntry> = result.take(0)?;

        let (sequence, prev_hash) = match last.first() {
            Some(last) => (last.sequence + 1, last.curr_hash.clone()),
            None => (1, "genesis".to_string()),
        };

        // 2. 计算哈希
        let timestamp = shared::util::now_millis();
        let curr_hash = compute_audit_hash(
            &prev_hash,
            sequence,
            timestamp,
            &action,
            &resource_type,
            &resource_id,
            operator_id.as_deref(),
            &details,
        );

        // 3. 先构造返回值（clone 字段），再构造插入数据（consume 字段）
        let entry = AuditEntry {
            id: sequence,
            timestamp,
            action,
            resource_type: resource_type.clone(),
            resource_id: resource_id.clone(),
            operator_id: operator_id.clone(),
            operator_name: operator_name.clone(),
            details: details.clone(),
            prev_hash: prev_hash.clone(),
            curr_hash: curr_hash.clone(),
        };

        let insert = AuditInsert {
            sequence,
            timestamp,
            action,
            resource_type,
            resource_id,
            operator_id,
            operator_name,
            details,
            prev_hash,
            curr_hash,
        };

        // 4. 写入 SurrealDB（bind 需要 'static，传 owned）
        let mut res = self
            .db
            .query("CREATE audit_log CONTENT $data")
            .bind(("data", insert))
            .await?;
        let _: Vec<AuditRecord> = res.take(0)?;

        Ok(entry)
    }

    /// 查询审计日志
    pub async fn query(&self, q: &AuditQuery) -> AuditStorageResult<(Vec<AuditEntry>, u64)> {
        let mut conditions = Vec::new();

        if q.from.is_some() {
            conditions.push("timestamp >= $from");
        }
        if q.to.is_some() {
            conditions.push("timestamp <= $to");
        }
        if q.action.is_some() {
            conditions.push("action = $action");
        }
        if q.operator_id.is_some() {
            conditions.push("operator_id = $operator_id");
        }
        if q.resource_type.is_some() {
            conditions.push("resource_type = $resource_type");
        }

        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!(" WHERE {}", conditions.join(" AND "))
        };

        let count_sql = format!(
            "SELECT count() as total FROM audit_log{} GROUP ALL",
            where_clause
        );
        let select_sql = format!(
            "SELECT * FROM audit_log{} ORDER BY sequence DESC LIMIT {} START {}",
            where_clause, q.limit, q.offset
        );
        let sql = format!("{}; {}", count_sql, select_sql);

        let mut qb = self.db.query(&sql);

        if let Some(from) = q.from {
            qb = qb.bind(("from", from));
        }
        if let Some(to) = q.to {
            qb = qb.bind(("to", to));
        }
        if let Some(ref action) = q.action {
            let action_str = serde_json::to_value(action)?
                .as_str()
                .unwrap_or_default()
                .to_string();
            qb = qb.bind(("action", action_str));
        }
        if let Some(ref operator_id) = q.operator_id {
            qb = qb.bind(("operator_id", operator_id.clone()));
        }
        if let Some(ref resource_type) = q.resource_type {
            qb = qb.bind(("resource_type", resource_type.clone()));
        }

        let mut result = qb.await?;

        let count_result: Vec<CountResult> = result.take(0)?;
        let total = count_result.first().map(|c| c.total).unwrap_or(0);

        let records: Vec<AuditRecord> = result.take(1)?;
        let entries = records.into_iter().map(AuditEntry::from).collect();

        Ok((entries, total))
    }

    /// 验证审计链完整性
    pub async fn verify_chain(
        &self,
        from: Option<i64>,
        to: Option<i64>,
    ) -> AuditStorageResult<AuditChainVerification> {
        let records: Vec<AuditRecord> = self
            .db
            .query("SELECT * FROM audit_log ORDER BY sequence")
            .await?
            .take(0)?;

        let mut breaks = Vec::new();
        let mut expected_prev_hash = "genesis".to_string();
        let mut expected_sequence: Option<u64> = None;
        let mut checked_count = 0u64;

        for record in records {
            let entry = AuditEntry::from(record);

            // 时间范围过滤
            if let Some(from_ts) = from
                && entry.timestamp < from_ts
            {
                expected_prev_hash = entry.curr_hash.clone();
                expected_sequence = Some(entry.id + 1);
                continue;
            }
            if let Some(to_ts) = to
                && entry.timestamp > to_ts
            {
                break;
            }

            // 验证序列号连续性（检测删除）
            if let Some(expected_seq) = expected_sequence
                && entry.id != expected_seq
            {
                breaks.push(AuditChainBreak {
                    entry_id: entry.id,
                    kind: ChainBreakKind::SequenceGap,
                    expected: expected_seq.to_string(),
                    actual: entry.id.to_string(),
                });
            }

            // 验证 prev_hash
            if entry.prev_hash != expected_prev_hash {
                breaks.push(AuditChainBreak {
                    entry_id: entry.id,
                    kind: ChainBreakKind::HashMismatch,
                    expected: expected_prev_hash.clone(),
                    actual: entry.prev_hash.clone(),
                });
            }

            // 验证 curr_hash 是否与重新计算的值一致
            let recomputed = compute_audit_hash(
                &entry.prev_hash,
                entry.id,
                entry.timestamp,
                &entry.action,
                &entry.resource_type,
                &entry.resource_id,
                entry.operator_id.as_deref(),
                &entry.details,
            );
            if entry.curr_hash != recomputed {
                breaks.push(AuditChainBreak {
                    entry_id: entry.id,
                    kind: ChainBreakKind::HashRecompute,
                    expected: recomputed,
                    actual: entry.curr_hash.clone(),
                });
            }

            expected_prev_hash = entry.curr_hash.clone();
            expected_sequence = Some(entry.id + 1);
            checked_count += 1;
        }

        Ok(AuditChainVerification {
            total_entries: checked_count,
            chain_intact: breaks.is_empty(),
            breaks,
        })
    }

    /// 查询最后 N 条审计日志（倒序）
    pub async fn query_last(&self, count: usize) -> AuditStorageResult<(Vec<AuditEntry>, u64)> {
        let sql = format!(
            "SELECT count() as total FROM audit_log GROUP ALL; \
             SELECT * FROM audit_log ORDER BY sequence DESC LIMIT {}",
            count
        );
        let mut result = self.db.query(&sql).await?;

        let count_result: Vec<CountResult> = result.take(0)?;
        let total = count_result.first().map(|c| c.total).unwrap_or(0);

        let records: Vec<AuditRecord> = result.take(1)?;
        let entries = records.into_iter().map(AuditEntry::from).collect();

        Ok((entries, total))
    }
}

/// 计算审计条目的 SHA256 哈希
///
/// 包含所有关键字段，任何修改都会导致哈希不匹配。
#[allow(clippy::too_many_arguments)]
fn compute_audit_hash(
    prev_hash: &str,
    id: u64,
    timestamp: i64,
    action: &AuditAction,
    resource_type: &str,
    resource_id: &str,
    operator_id: Option<&str>,
    details: &serde_json::Value,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(prev_hash.as_bytes());
    hasher.update(id.to_le_bytes());
    hasher.update(timestamp.to_le_bytes());
    hasher.update(format!("{:?}", action).as_bytes());
    hasher.update(resource_type.as_bytes());
    hasher.update(resource_id.as_bytes());
    hasher.update(operator_id.unwrap_or("system").as_bytes());
    let details_json = serde_json::to_string(details).unwrap_or_default();
    hasher.update(details_json.as_bytes());
    format!("{:x}", hasher.finalize())
}
