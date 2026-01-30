//! 审计日志服务
//!
//! `AuditService` 是审计日志的核心服务，提供：
//! - 日志写入（通过 mpsc 通道异步接收）
//! - 日志查询（直接读取 SurrealDB）
//! - 链验证
//! - 系统生命周期管理（LOCK 文件 + 24h 间隔检测）
//! - 启动异常确认（前端 dialog，持久化到文件）

use std::path::PathBuf;
use std::sync::Arc;
use surrealdb::engine::local::Db;
use surrealdb::Surreal;
use tokio::sync::mpsc;

use super::storage::{AuditStorage, AuditStorageError};
use super::types::*;

/// LOCK 文件名
const LOCK_FILE_NAME: &str = "audit.lock";

/// 待确认启动异常文件名（持久化，重启后仍存在）
const PENDING_ACK_FILE: &str = "pending-ack.json";

/// 超过此时间未运行视为长时间停机（毫秒）
const LONG_DOWNTIME_THRESHOLD_MS: i64 = 24 * 60 * 60 * 1000; // 24h

/// 发送到 AuditService 的日志请求
pub struct AuditLogRequest {
    pub action: AuditAction,
    pub resource_type: String,
    pub resource_id: String,
    pub operator_id: Option<String>,
    pub operator_name: Option<String>,
    pub details: serde_json::Value,
}

/// 审计日志服务
///
/// 通过 mpsc 通道接收日志请求，异步写入 SurrealDB。
/// 查询操作直接读取 storage。
///
/// ## LOCK 文件机制
///
/// 启动时创建 `audit.lock` 文件（写入当前时间戳），
/// 正常关闭时删除。下次启动检测：
/// - LOCK 文件存在 → 异常关闭
/// - LOCK 文件不存在但最后审计时间 >24h 前 → 长时间停机
///
/// ## 启动异常确认（持久化）
///
/// 检测到异常关闭或长时间停机时，写入 `pending-ack.json` 文件。
/// 即使用户关机重启，该文件仍存在，前端仍会弹窗要求输入原因。
/// 用户确认后才删除文件。
pub struct AuditService {
    storage: AuditStorage,
    tx: mpsc::Sender<AuditLogRequest>,
    lock_path: PathBuf,
    pending_ack_path: PathBuf,
    pending_startup_issues: tokio::sync::Mutex<Vec<StartupIssue>>,
}

impl std::fmt::Debug for AuditService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AuditService")
            .field("lock_path", &self.lock_path)
            .finish_non_exhaustive()
    }
}

impl AuditService {
    /// 创建审计服务
    ///
    /// `data_dir` — 数据目录（LOCK 文件和 pending-ack 文件存放位置）
    pub fn new(
        db: Surreal<Db>,
        data_dir: &std::path::Path,
        buffer_size: usize,
    ) -> (Arc<Self>, mpsc::Receiver<AuditLogRequest>) {
        let (tx, rx) = mpsc::channel(buffer_size);
        let lock_path = data_dir.join(LOCK_FILE_NAME);
        let pending_ack_path = data_dir.join(PENDING_ACK_FILE);
        let storage = AuditStorage::new(db);
        let service = Arc::new(Self {
            storage,
            tx,
            lock_path,
            pending_ack_path,
            pending_startup_issues: tokio::sync::Mutex::new(Vec::new()),
        });
        (service, rx)
    }

    /// 系统启动时调用 — 检测异常关闭和长时间停机，创建 LOCK 文件
    ///
    /// 检测到的异常会写入审计日志并持久化到 `pending-ack.json`，
    /// 即使用户关机重启，前端仍会弹窗要求输入原因。
    pub async fn on_startup(&self) {
        let now = shared::util::now_millis();

        // 先加载已有的 pending-ack 文件（上次未确认的异常）
        let mut issues = self.load_pending_from_file();

        // 1. 检测异常关闭：LOCK 文件存在
        if self.lock_path.exists() {
            let lock_content = std::fs::read_to_string(&self.lock_path).unwrap_or_default();
            let last_start_ts: i64 = lock_content.trim().parse().unwrap_or(0);

            tracing::warn!(
                "⚠️ Abnormal shutdown detected — LOCK file exists (last start: {})",
                last_start_ts
            );

            let details = serde_json::json!({
                "last_start_timestamp": last_start_ts,
                "detected_at": now,
                "note": "Previous session did not shut down cleanly"
            });

            match self
                .storage
                .append(
                    AuditAction::SystemAbnormalShutdown,
                    "system".to_string(),
                    "server:main".to_string(),
                    None,
                    None,
                    details.clone(),
                )
                .await
            {
                Ok(entry) => {
                    issues.push(StartupIssue {
                        sequence: entry.id,
                        action: AuditAction::SystemAbnormalShutdown,
                        details,
                        timestamp: entry.timestamp,
                    });
                }
                Err(e) => tracing::error!("Failed to log abnormal shutdown: {:?}", e),
            }
        }

        // 2. 检测长时间停机：最后审计记录 >24h 前
        if let Ok((entries, _)) = self.storage.query_last(1).await
            && let Some(last_entry) = entries.first()
        {
            let gap = now - last_entry.timestamp;
            if gap > LONG_DOWNTIME_THRESHOLD_MS {
                let hours = gap / (60 * 60 * 1000);
                tracing::warn!("⚠️ Long downtime detected — system offline for {}h", hours);

                let details = serde_json::json!({
                    "last_activity_timestamp": last_entry.timestamp,
                    "downtime_ms": gap,
                    "downtime_hours": hours,
                });

                match self
                    .storage
                    .append(
                        AuditAction::SystemLongDowntime,
                        "system".to_string(),
                        "server:main".to_string(),
                        None,
                        None,
                        details.clone(),
                    )
                    .await
                {
                    Ok(entry) => {
                        issues.push(StartupIssue {
                            sequence: entry.id,
                            action: AuditAction::SystemLongDowntime,
                            details,
                            timestamp: entry.timestamp,
                        });
                    }
                    Err(e) => tracing::error!("Failed to log long downtime: {:?}", e),
                }
            }
        }

        // 3. 创建 LOCK 文件
        if let Err(e) = std::fs::write(&self.lock_path, now.to_string()) {
            tracing::error!("Failed to create audit LOCK file: {:?}", e);
        }

        // 4. 持久化 pending issues 到文件（重启后仍存在）
        if !issues.is_empty() {
            self.save_pending_to_file(&issues);
        }

        *self.pending_startup_issues.lock().await = issues;
    }

    /// 系统正常关闭时调用 — 删除 LOCK 文件
    pub fn on_shutdown(&self) {
        if let Err(e) = std::fs::remove_file(&self.lock_path)
            && e.kind() != std::io::ErrorKind::NotFound
        {
            tracing::error!("Failed to remove audit LOCK file: {:?}", e);
        }
    }

    /// 异步记录审计日志（非阻塞）
    ///
    /// 通过 mpsc 通道发送到后台 worker。
    /// 如果通道满，阻塞等待（审计日志不允许丢失）。
    pub async fn log(
        &self,
        action: AuditAction,
        resource_type: impl Into<String>,
        resource_id: impl Into<String>,
        operator_id: Option<String>,
        operator_name: Option<String>,
        details: serde_json::Value,
    ) {
        let req = AuditLogRequest {
            action,
            resource_type: resource_type.into(),
            resource_id: resource_id.into(),
            operator_id,
            operator_name,
            details,
        };

        // 阻塞发送 — 审计日志不允许丢失
        if self.tx.send(req).await.is_err() {
            tracing::error!("Audit log channel closed — audit entry lost!");
        }
    }

    /// 直接写入审计日志（用于启动/关闭等场景）
    pub async fn log_sync(
        &self,
        action: AuditAction,
        resource_type: impl Into<String>,
        resource_id: impl Into<String>,
        details: serde_json::Value,
    ) -> Result<AuditEntry, AuditStorageError> {
        self.storage
            .append(
                action,
                resource_type.into(),
                resource_id.into(),
                None,
                None,
                details,
            )
            .await
    }

    /// 查询审计日志
    pub async fn query(
        &self,
        q: &AuditQuery,
    ) -> Result<(Vec<AuditEntry>, u64), AuditStorageError> {
        self.storage.query(q).await
    }

    /// 验证审计链完整性
    pub async fn verify_chain(
        &self,
        from: Option<i64>,
        to: Option<i64>,
    ) -> Result<AuditChainVerification, AuditStorageError> {
        self.storage.verify_chain(from, to).await
    }

    /// 获取待确认的启动异常
    pub async fn get_pending_startup(&self) -> Vec<StartupIssue> {
        self.pending_startup_issues.lock().await.clone()
    }

    /// 确认启动异常（前端用户提交原因后调用）
    ///
    /// 创建一条 AcknowledgeStartupIssue 审计记录，
    /// 并删除 pending-ack.json 文件。
    pub async fn acknowledge_startup(
        &self,
        reason: String,
        operator_id: Option<String>,
        operator_name: Option<String>,
    ) -> Result<(), AuditStorageError> {
        let mut pending = self.pending_startup_issues.lock().await;

        if pending.is_empty() {
            return Ok(());
        }

        let sequences: Vec<u64> = pending.iter().map(|i| i.sequence).collect();

        self.storage
            .append(
                AuditAction::AcknowledgeStartupIssue,
                "system".to_string(),
                "server:main".to_string(),
                operator_id,
                operator_name,
                serde_json::json!({
                    "reason": reason,
                    "acknowledged_sequences": sequences,
                }),
            )
            .await?;

        pending.clear();

        // 删除持久化文件
        let _ = std::fs::remove_file(&self.pending_ack_path);

        Ok(())
    }

    /// 获取存储引用
    pub fn storage(&self) -> &AuditStorage {
        &self.storage
    }

    // ═══ 内部方法 ═══

    /// 从 pending-ack.json 加载已有的未确认异常
    fn load_pending_from_file(&self) -> Vec<StartupIssue> {
        if !self.pending_ack_path.exists() {
            return Vec::new();
        }
        match std::fs::read_to_string(&self.pending_ack_path) {
            Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
            Err(_) => Vec::new(),
        }
    }

    /// 持久化 pending issues 到 pending-ack.json
    fn save_pending_to_file(&self, issues: &[StartupIssue]) {
        match serde_json::to_string_pretty(issues) {
            Ok(json) => {
                if let Err(e) = std::fs::write(&self.pending_ack_path, json) {
                    tracing::error!("Failed to write pending-ack.json: {:?}", e);
                }
            }
            Err(e) => tracing::error!("Failed to serialize pending issues: {:?}", e),
        }
    }
}
