//! 审计日志后台 Worker
//!
//! 从 mpsc 通道消费 AuditLogRequest，写入 SurrealDB。
//! 失败时重试 3 次（指数退避），通道关闭时自动退出。

use super::service::AuditLogRequest;
use super::storage::AuditStorage;

/// 最大重试次数
const MAX_RETRIES: u32 = 3;

/// 审计日志后台 Worker
///
/// 从 mpsc 通道消费日志请求，写入 SurrealDB 存储。
/// 失败时最多重试 3 次，退避间隔 100ms → 200ms → 400ms。
pub struct AuditWorker {
    storage: AuditStorage,
}

impl AuditWorker {
    pub fn new(storage: AuditStorage) -> Self {
        Self { storage }
    }

    /// 运行 worker（阻塞直到通道关闭）
    pub async fn run(self, mut rx: tokio::sync::mpsc::Receiver<AuditLogRequest>) {
        tracing::info!("📋 Audit log worker started");

        while let Some(req) = rx.recv().await {
            let mut last_err = None;

            for attempt in 0..=MAX_RETRIES {
                if attempt > 0 {
                    let backoff = std::time::Duration::from_millis(100 << (attempt - 1));
                    tracing::warn!(
                        attempt,
                        backoff_ms = backoff.as_millis() as u64,
                        "Retrying audit entry write"
                    );
                    tokio::time::sleep(backoff).await;
                }

                match self
                    .storage
                    .append(
                        req.action,
                        req.resource_type.clone(),
                        req.resource_id.clone(),
                        req.operator_id.clone(),
                        req.operator_name.clone(),
                        req.details.clone(),
                    )
                    .await
                {
                    Ok(entry) => {
                        if attempt > 0 {
                            tracing::info!(
                                audit_id = entry.id,
                                attempt,
                                "Audit entry recorded after retry"
                            );
                        } else {
                            tracing::debug!(
                                audit_id = entry.id,
                                action = %entry.action,
                                resource = %entry.resource_type,
                                "Audit entry recorded"
                            );
                        }
                        last_err = None;
                        break;
                    }
                    Err(e) => {
                        last_err = Some(e);
                    }
                }
            }

            if let Some(e) = last_err {
                tracing::error!(
                    "AUDIT ENTRY LOST after {} retries: {:?}",
                    MAX_RETRIES,
                    e
                );
            }
        }

        tracing::info!("Audit log channel closed, worker stopping");
    }
}
