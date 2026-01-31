//! 审计日志服务
//!
//! `AuditService` 是审计日志的核心服务，提供：
//! - 日志写入（通过 mpsc 通道异步接收）
//! - 日志查询（直接读取 SurrealDB）
//! - 链验证
//! - 系统生命周期管理（LOCK 文件 + 24h 间隔检测）
//! - 启动异常检测 → 写入 system_issue 表（前端通过 system-issues API 渲染对话框）

use std::collections::HashMap;
use std::path::PathBuf;
use chrono::Local;
use std::sync::Arc;
use surrealdb::engine::local::Db;
use surrealdb::Surreal;
use tokio::sync::mpsc;

use super::storage::{AuditStorage, AuditStorageError};
use super::types::*;
use crate::db::repository::system_issue::{CreateSystemIssue, SystemIssueRepository};

/// LOCK 文件名
const LOCK_FILE_NAME: &str = "audit.lock";

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
/// ## 启动异常检测
///
/// 检测到异常关闭或长时间停机时，写入 `system_issue` 表。
/// 前端通过 `/api/system-issues/pending` 拉取待处理问题，
/// 渲染阻塞式对话框要求用户回应。
pub struct AuditService {
    storage: AuditStorage,
    db: Surreal<Db>,
    tx: mpsc::Sender<AuditLogRequest>,
    lock_path: PathBuf,
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
    /// `data_dir` — 数据目录（LOCK 文件存放位置）
    pub fn new(
        db: Surreal<Db>,
        data_dir: &std::path::Path,
        buffer_size: usize,
    ) -> (Arc<Self>, mpsc::Receiver<AuditLogRequest>) {
        let (tx, rx) = mpsc::channel(buffer_size);
        let lock_path = data_dir.join(LOCK_FILE_NAME);
        let storage = AuditStorage::new(db.clone());
        let service = Arc::new(Self {
            storage,
            db,
            tx,
            lock_path,
        });
        (service, rx)
    }

    /// 系统启动时调用 — 检测异常关闭和长时间停机，创建 LOCK 文件
    ///
    /// 检测到的异常会：
    /// 1. 写入审计日志（不可篡改的记录）
    /// 2. 写入 system_issue 表（前端对话框渲染）
    pub async fn on_startup(&self) {
        let now = shared::util::now_millis();
        let issue_repo = SystemIssueRepository::new(self.db.clone());

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
                "note": "abnormal_shutdown_detected"
            });

            // 审计日志始终记录（每次异常关闭都是独立事件）
            if let Err(e) = self
                .storage
                .append(
                    AuditAction::SystemAbnormalShutdown,
                    "system".to_string(),
                    "server:main".to_string(),
                    None,
                    None,
                    details,
                )
                .await
            {
                tracing::error!("Failed to log abnormal shutdown: {:?}", e);
            }

            // 去重：如果已有同类型未解决的 issue，不重复创建
            match issue_repo.find_pending_by_kind("abnormal_shutdown").await {
                Ok(existing) if existing.is_empty() => {
                    let mut params = HashMap::new();
                    let formatted_ts = chrono::DateTime::from_timestamp_millis(last_start_ts)
                        .map(|dt| dt.with_timezone(&Local).format("%Y-%m-%d %H:%M").to_string())
                        .unwrap_or_else(|| last_start_ts.to_string());
                    params.insert("last_start_timestamp".to_string(), formatted_ts);
                    if let Err(e) = issue_repo
                        .create(CreateSystemIssue {
                            source: "local".to_string(),
                            kind: "abnormal_shutdown".to_string(),
                            blocking: true,
                            target: None,
                            params,
                            title: None,
                            description: None,
                            options: vec![
                                "power_outage".to_string(),
                                "app_crash".to_string(),
                                "device_failure".to_string(),
                                "maintenance_restart".to_string(),
                                "other".to_string(),
                            ],
                        })
                        .await
                    {
                        tracing::error!("Failed to create system_issue for abnormal shutdown: {:?}", e);
                    }
                }
                Ok(_) => {
                    tracing::debug!("Pending abnormal_shutdown issue already exists, skipping");
                }
                Err(e) => {
                    tracing::error!("Failed to query pending issues: {:?}", e);
                }
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

                if let Err(e) = self
                    .storage
                    .append(
                        AuditAction::SystemLongDowntime,
                        "system".to_string(),
                        "server:main".to_string(),
                        None,
                        None,
                        details,
                    )
                    .await
                {
                    tracing::error!("Failed to log long downtime: {:?}", e);
                }

                // 去重：如果已有同类型未解决的 issue，不重复创建
                match issue_repo.find_pending_by_kind("long_downtime").await {
                    Ok(existing) if existing.is_empty() => {
                        let mut params = HashMap::new();
                        params.insert("downtime_hours".to_string(), hours.to_string());
                        if let Err(e) = issue_repo
                            .create(CreateSystemIssue {
                                source: "local".to_string(),
                                kind: "long_downtime".to_string(),
                                blocking: true,
                                target: None,
                                params,
                                title: None,
                                description: None,
                                options: vec![
                                    "power_outage".to_string(),
                                    "device_failure".to_string(),
                                    "maintenance_restart".to_string(),
                                    "other".to_string(),
                                ],
                            })
                            .await
                        {
                            tracing::error!("Failed to create system_issue for long downtime: {:?}", e);
                        }
                    }
                    Ok(_) => {
                        tracing::debug!("Pending long_downtime issue already exists, skipping");
                    }
                    Err(e) => {
                        tracing::error!("Failed to query pending issues: {:?}", e);
                    }
                }
            }
        }

        // 3. 创建 LOCK 文件
        if let Err(e) = std::fs::write(&self.lock_path, now.to_string()) {
            tracing::error!("Failed to create audit LOCK file: {:?}", e);
        }
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

    /// 获取存储引用
    pub fn storage(&self) -> &AuditStorage {
        &self.storage
    }
}
