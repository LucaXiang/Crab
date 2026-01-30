//! 审计日志后台 Worker
//!
//! 从 mpsc 通道消费 AuditLogRequest，写入 SurrealDB。
//! 通道关闭时自动退出。

use super::service::AuditLogRequest;
use super::storage::AuditStorage;

/// 审计日志后台 Worker
///
/// 从 mpsc 通道消费日志请求，写入 SurrealDB 存储。
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
            match self
                .storage
                .append(
                    req.action,
                    req.resource_type,
                    req.resource_id,
                    req.operator_id,
                    req.operator_name,
                    req.details,
                )
                .await
            {
                Ok(entry) => {
                    tracing::debug!(
                        audit_id = entry.id,
                        action = %entry.action,
                        resource = %entry.resource_type,
                        "Audit entry recorded"
                    );
                }
                Err(e) => {
                    tracing::error!("Failed to write audit entry: {:?}", e);
                }
            }
        }

        tracing::info!("Audit log channel closed, worker stopping");
    }
}
