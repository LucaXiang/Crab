//! 审计日志 SQLite 存储层
//!
//! Append-only 设计，没有任何删除/更新接口。
//! SHA256 哈希链确保防篡改。

use std::sync::Arc;
use sha2::{Digest, Sha256};
use sqlx::SqlitePool;
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

impl From<sqlx::Error> for AuditStorageError {
    fn from(err: sqlx::Error) -> Self {
        AuditStorageError::Database(err.to_string())
    }
}

pub type AuditStorageResult<T> = Result<T, AuditStorageError>;

impl From<AuditStorageError> for shared::error::AppError {
    fn from(err: AuditStorageError) -> Self {
        shared::error::AppError::internal(err.to_string())
    }
}

/// SQLite 查询结果行
#[derive(Debug, Clone, sqlx::FromRow)]
struct AuditRow {
    sequence: i64,
    timestamp: i64,
    action: String,
    resource_type: String,
    resource_id: String,
    operator_id: Option<String>,
    operator_name: Option<String>,
    details: String,
    target: Option<String>,
    prev_hash: String,
    curr_hash: String,
}

impl AuditRow {
    fn into_entry(self) -> AuditEntry {
        let action: AuditAction = serde_json::from_str(&format!("\"{}\"", self.action))
            .unwrap_or(AuditAction::SystemStartup);
        let details: serde_json::Value =
            serde_json::from_str(&self.details).unwrap_or_default();
        AuditEntry {
            id: self.sequence as u64,
            timestamp: self.timestamp,
            action,
            resource_type: self.resource_type,
            resource_id: self.resource_id,
            operator_id: self.operator_id,
            operator_name: self.operator_name,
            details,
            target: self.target,
            prev_hash: self.prev_hash,
            curr_hash: self.curr_hash,
        }
    }
}

/// 审计日志存储 (SQLite)
///
/// Append-only 设计：
/// - 仅提供 `append` 和 `query` 方法
/// - 没有 delete/update 接口
/// - SHA256 哈希链确保完整性
#[derive(Clone)]
pub struct AuditStorage {
    pool: SqlitePool,
    /// 序列化所有 append 操作，防止 read-modify-write 竞争
    append_lock: Arc<tokio::sync::Mutex<()>>,
}

impl AuditStorage {
    pub fn new(pool: SqlitePool) -> Self {
        Self {
            pool,
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
        let last = sqlx::query_as::<_, (i64, String)>(
            "SELECT sequence, curr_hash FROM audit_log ORDER BY sequence DESC LIMIT 1",
        )
        .fetch_optional(&self.pool)
        .await?;

        let (sequence, prev_hash) = match last {
            Some((seq, hash)) => ((seq + 1) as u64, hash),
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

        // 3. 序列化字段用于插入
        let action_str = serde_json::to_value(&action)?
            .as_str()
            .unwrap_or_default()
            .to_string();
        let details_json = serde_json::to_string(&details)?;

        // 4. 写入 SQLite
        let sequence_i64 = sequence as i64;
        sqlx::query!(
            "INSERT INTO audit_log (sequence, timestamp, action, resource_type, resource_id, \
             operator_id, operator_name, details, target, prev_hash, curr_hash) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            sequence_i64,
            timestamp,
            action_str,
            resource_type,
            resource_id,
            operator_id,
            operator_name,
            details_json,
            target,
            prev_hash,
            curr_hash,
        )
        .execute(&self.pool)
        .await?;

        let entry = AuditEntry {
            id: sequence,
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

        Ok(entry)
    }

    /// 查询审计日志
    pub async fn query(&self, q: &AuditQuery) -> AuditStorageResult<(Vec<AuditEntry>, u64)> {
        let mut conditions = Vec::new();
        let mut bind_values: Vec<BindValue> = Vec::new();

        if let Some(from) = q.from {
            conditions.push("timestamp >= ?");
            bind_values.push(BindValue::Int(from));
        }
        if let Some(to) = q.to {
            conditions.push("timestamp <= ?");
            bind_values.push(BindValue::Int(to));
        }
        if let Some(ref action) = q.action {
            conditions.push("action = ?");
            let action_str = serde_json::to_value(action)?
                .as_str()
                .unwrap_or_default()
                .to_string();
            bind_values.push(BindValue::Str(action_str));
        }
        if let Some(ref operator_id) = q.operator_id {
            conditions.push("operator_id = ?");
            bind_values.push(BindValue::Str(operator_id.clone()));
        }
        if let Some(ref resource_type) = q.resource_type {
            conditions.push("resource_type = ?");
            bind_values.push(BindValue::Str(resource_type.clone()));
        }

        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!(" WHERE {}", conditions.join(" AND "))
        };

        // COUNT 查询
        let count_sql = format!("SELECT COUNT(*) FROM audit_log{}", where_clause);
        let mut count_query = sqlx::query_scalar::<_, i64>(&count_sql);
        for v in &bind_values {
            count_query = match v {
                BindValue::Int(i) => count_query.bind(i),
                BindValue::Str(s) => count_query.bind(s),
            };
        }
        let total = count_query.fetch_one(&self.pool).await? as u64;

        // SELECT 查询
        let select_sql = format!(
            "SELECT sequence, timestamp, action, resource_type, resource_id, \
             operator_id, operator_name, details, target, prev_hash, curr_hash \
             FROM audit_log{} ORDER BY sequence DESC LIMIT ? OFFSET ?",
            where_clause
        );
        let mut select_query = sqlx::query_as::<_, AuditRow>(&select_sql);
        for v in &bind_values {
            select_query = match v {
                BindValue::Int(i) => select_query.bind(i),
                BindValue::Str(s) => select_query.bind(s),
            };
        }
        select_query = select_query.bind(q.limit as i64).bind(q.offset as i64);
        let rows = select_query.fetch_all(&self.pool).await?;

        let entries = rows.into_iter().map(AuditRow::into_entry).collect();
        Ok((entries, total))
    }

    /// 查询最后 N 条审计日志（倒序）
    pub async fn query_last(&self, count: usize) -> AuditStorageResult<(Vec<AuditEntry>, u64)> {
        let total = sqlx::query_scalar!("SELECT COUNT(*) FROM audit_log")
            .fetch_one(&self.pool)
            .await? as u64;

        let rows = sqlx::query_as::<_, AuditRow>(
            "SELECT sequence, timestamp, action, resource_type, resource_id, \
             operator_id, operator_name, details, target, prev_hash, curr_hash \
             FROM audit_log ORDER BY sequence DESC LIMIT ?",
        )
        .bind(count as i64)
        .fetch_all(&self.pool)
        .await?;

        let entries = rows.into_iter().map(AuditRow::into_entry).collect();
        Ok((entries, total))
    }
}

/// 动态绑定值（用于构建参数化查询）
enum BindValue {
    Int(i64),
    Str(String),
}

/// 规范化 JSON Value — 将浮点退化的整数还原为 i64
///
/// 历史遗留：旧存储层将所有数字存为 float，读出后 `5` 变成 `5.0`。
/// 此函数确保 `5.0` -> `5`（无小数部分时），使序列化结果在写入和读出时一致。
/// 保留此函数以维持已有哈希链的兼容性。
///
/// 安全范围：f64 尾数 52 bit，仅 |value| <= 2^53 的整数可无损转换。
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
/// - details 经过 normalize_json 规范化，消除数值精度漂移
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
