//! 审计日志 SurrealDB 存储层
//!
//! Append-only 设计，没有任何删除/更新接口。
//! SHA256 哈希链确保防篡改。

use std::sync::Arc;
use sha2::{Digest, Sha256};
use surrealdb::engine::local::Db;
use surrealdb::Surreal;
use thiserror::Error;

use super::types::{AuditAction, AuditEntry, AuditQuery};

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
    #[serde(default)]
    target: Option<String>,
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
            target: r.target,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    target: Option<String>,
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
    #[allow(clippy::too_many_arguments)]
    pub async fn append(
        &self,
        action: AuditAction,
        resource_type: String,
        resource_id: String,
        operator_id: Option<String>,
        operator_name: Option<String>,
        details: serde_json::Value,
        target: Option<String>,
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

        // 2. 计算哈希（所有存储字段参与）
        let timestamp = shared::util::now_millis();
        let curr_hash = compute_audit_hash(
            &prev_hash,
            sequence,
            timestamp,
            &action,
            &resource_type,
            &resource_id,
            operator_id.as_deref(),
            operator_name.as_deref(),
            &details,
            target.as_deref(),
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
            target: target.clone(),
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
            target,
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

/// 规范化 JSON Value — 将 SurrealDB 浮点退化的整数还原为 i64
///
/// SurrealDB 内部将所有数字存为 float，读出后 `5` 变成 `5.0`。
/// 此函数确保 `5.0` → `5`（无小数部分时），使序列化结果在写入和读出时一致。
///
/// 安全范围：f64 尾数 52 bit，仅 |value| ≤ 2^53 的整数可无损转换。
fn normalize_json(value: &serde_json::Value) -> serde_json::Value {
    /// f64 可精确表示的最大整数绝对值 (2^53)
    const MAX_SAFE_INT: f64 = (1_i64 << 53) as f64;

    match value {
        serde_json::Value::Number(n) => {
            if let Some(f) = n.as_f64()
                && f.fract() == 0.0
                && f.abs() <= MAX_SAFE_INT
            {
                return serde_json::Value::Number(serde_json::Number::from(f as i64));
            }
            value.clone()
        }
        serde_json::Value::Object(map) => {
            let normalized: serde_json::Map<String, serde_json::Value> = map
                .iter()
                .map(|(k, v)| (k.clone(), normalize_json(v)))
                .collect();
            serde_json::Value::Object(normalized)
        }
        serde_json::Value::Array(arr) => {
            serde_json::Value::Array(arr.iter().map(normalize_json).collect())
        }
        _ => value.clone(),
    }
}

/// 计算审计条目的 SHA256 哈希
///
/// 所有存储字段参与哈希，任何修改都会导致不匹配。
///
/// 设计要点：
/// - 变长字段间用 `\x00` 分隔，防止 `("ab","cd")` 与 `("abc","d")` 碰撞
/// - 定长字段（u64/i64）用 LE 字节序，无需分隔
/// - Optional 字段用 `\x00`=None / `\x01`+bytes=Some 区分，避免 None 与 Some("") 碰撞
/// - action 使用 serde 序列化（snake_case，跨版本稳定），而非 Debug trait
/// - details 经过 normalize_json 规范化，消除 SurrealDB 数值精度漂移
#[allow(clippy::too_many_arguments)]
fn compute_audit_hash(
    prev_hash: &str,
    id: u64,
    timestamp: i64,
    action: &AuditAction,
    resource_type: &str,
    resource_id: &str,
    operator_id: Option<&str>,
    operator_name: Option<&str>,
    details: &serde_json::Value,
    target: Option<&str>,
) -> String {
    let mut hasher = Sha256::new();

    // 链接前一条哈希
    hasher.update(prev_hash.as_bytes());
    hasher.update(b"\x00");

    // 定长字段
    hasher.update(id.to_le_bytes());
    hasher.update(timestamp.to_le_bytes());

    // action — serde snake_case (稳定格式，与 DB 存储一致)
    let action_str = serde_json::to_string(action).unwrap_or_default();
    hasher.update(action_str.as_bytes());
    hasher.update(b"\x00");

    // 变长字符串字段 — 分隔符隔离
    hasher.update(resource_type.as_bytes());
    hasher.update(b"\x00");
    hasher.update(resource_id.as_bytes());
    hasher.update(b"\x00");

    // Optional 字段 — tag byte 区分 None/Some
    hash_optional(&mut hasher, operator_id);
    hash_optional(&mut hasher, operator_name);

    // details JSON (规范化)
    let normalized = normalize_json(details);
    let details_json = serde_json::to_string(&normalized).unwrap_or_default();
    hasher.update(details_json.as_bytes());
    hasher.update(b"\x00");

    // target
    hash_optional(&mut hasher, target);

    format!("{:x}", hasher.finalize())
}

/// Optional 字段哈希：`\x00` = None, `\x01` + bytes + `\x00` = Some
fn hash_optional(hasher: &mut Sha256, value: Option<&str>) {
    match value {
        Some(v) => {
            hasher.update(b"\x01");
            hasher.update(v.as_bytes());
        }
        None => {
            hasher.update(b"\x00");
        }
    }
    hasher.update(b"\x00");
}
