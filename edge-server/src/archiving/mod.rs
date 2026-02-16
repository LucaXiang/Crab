//! 归档系统 — 订单归档到 SQLite + 哈希链验证
//!
//! - **service**: OrderArchiveService (归档到 SQLite，哈希链完整性)
//! - **worker**: ArchiveWorker (队列处理，并发归档，重试)
//! - **verify**: VerifyScheduler (启动补扫 + 每日定时验证)

pub mod service;
pub mod verify;
pub mod worker;

pub use service::{
    ArchiveError, ArchiveResult, ChainBreak, ChainReset, DailyChainVerification, EventVerification,
    OrderArchiveService, OrderVerification,
};
pub use verify::VerifyScheduler;
pub use worker::ArchiveWorker;
