//! 审计日志后台 Worker
//!
//! 从 mpsc 通道消费 AuditLogRequest，写入 SQLite。
//! 失败时重试 3 次（指数退避），重试耗尽后写入 dead letter 文件。
//! 通道关闭时自动退出。

use std::path::PathBuf;

use super::service::AuditLogRequest;
use super::storage::AuditStorage;

/// 最大重试次数
const MAX_RETRIES: u32 = 3;

/// 审计日志后台 Worker
///
/// 从 mpsc 通道消费日志请求，写入 SQLite 存储。
/// 失败时最多重试 3 次，退避间隔 100ms → 200ms → 400ms。
/// 重试耗尽后写入 dead letter 文件，防止条目丢失。
pub struct AuditWorker {
    storage: AuditStorage,
    dead_letter_path: PathBuf,
}

impl AuditWorker {
    pub fn new(storage: AuditStorage, dead_letter_path: PathBuf) -> Self {
        Self {
            storage,
            dead_letter_path,
        }
    }

    /// 运行 worker（阻塞直到通道关闭）
    pub async fn run(self, mut rx: tokio::sync::mpsc::Receiver<AuditLogRequest>) {
        tracing::info!("Audit log worker started");

        while let Some(req) = rx.recv().await {
            self.process_entry(req).await;
        }

        tracing::info!("Audit log channel closed, worker stopping");
    }

    /// 处理单条审计条目（含重试和 dead letter）
    async fn process_entry(&self, req: AuditLogRequest) {
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
                    req.operator_id,
                    req.operator_name.clone(),
                    req.details.clone(),
                    req.target.clone(),
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
                    return;
                }
                Err(e) => {
                    last_err = Some(e);
                }
            }
        }

        // 重试耗尽 — 写入 dead letter 文件
        if let Some(e) = last_err {
            tracing::error!(
                "Audit entry failed after {} retries: {:?}, writing to dead letter",
                MAX_RETRIES,
                e
            );
            self.write_dead_letter(&req);
        }
    }

    /// 将失败的条目追加到 dead letter JSONL 文件
    fn write_dead_letter(&self, req: &AuditLogRequest) {
        use std::io::Write;

        match serde_json::to_string(req) {
            Ok(json) => {
                let result = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&self.dead_letter_path)
                    .and_then(|mut f| writeln!(f, "{}", json));

                match result {
                    Ok(()) => {
                        tracing::error!(
                            path = %self.dead_letter_path.display(),
                            "Audit entry written to dead letter file"
                        );
                    }
                    Err(e) => {
                        tracing::error!("AUDIT ENTRY LOST — dead letter write failed: {:?}", e);
                    }
                }
            }
            Err(e) => {
                tracing::error!("AUDIT ENTRY LOST — serialization failed: {:?}", e);
            }
        }
    }
}
