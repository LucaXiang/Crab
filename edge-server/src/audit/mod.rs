//! 审计日志模块 — 税务级防篡改审计追踪
//!
//! # 架构
//!
//! ```text
//! 敏感操作触发
//!   ├─ AuditService::log() → mpsc → AuditWorker → SurrealDB (audit_log 表)
//!   └─ AuditService::log_sync() → SurrealDB (启动/关闭场景)
//!
//! SHA256 哈希链: genesis → entry₁ → entry₂ → ... → entryₙ
//! ```
//!
//! # 防篡改保证
//!
//! - **SHA256 哈希链**: 每条记录包含前一条的哈希
//! - **Append-only**: 无删除/更新接口
//! - **SurrealDB schema**: update/delete 权限为 NONE
//! - **链验证**: 哈希链数据可导出供外部工具验证
//!
//! # 启动异常检测
//!
//! - **LOCK 文件**: 检测异常关闭
//! - **24h 间隔**: 检测长时间停机
//! - **system_issue 表**: 持久化待确认异常（SurrealDB）
//! - **前端 system-issues API**: 渲染对话框要求用户回应

pub mod service;
pub mod storage;
pub mod types;
pub mod worker;

pub use service::{AuditLogRequest, AuditService};
pub use storage::{AuditStorage, AuditStorageError};
pub use types::{
    AuditAction, AuditEntry, AuditListResponse, AuditQuery,
};
pub use worker::AuditWorker;
