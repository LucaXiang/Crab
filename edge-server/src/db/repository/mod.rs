//! Repository Module
//!
//! Provides CRUD operations for SurrealDB tables using Graph DB patterns.

// Auth
pub mod employee;
pub mod role;

// Product Domain
pub mod attribute;
pub mod print_destination;
pub mod tag;

// Location
pub mod dining_table;
pub mod zone;

// Pricing
pub mod price_rule;

// Orders
pub mod order;

// Payments
pub mod payment;

// System
pub mod store_info;
pub mod label_template;
pub mod system_state;
pub mod system_issue;

// Image
pub mod image_ref;

// Operations (班次与日结)
pub mod shift;
pub mod daily_report;

// Re-exports
pub use attribute::AttributeRepository;
pub use dining_table::DiningTableRepository;
pub use employee::EmployeeRepository;
pub use order::OrderRepository;
pub use price_rule::PriceRuleRepository;
pub use print_destination::PrintDestinationRepository;
pub use role::RoleRepository;
pub use store_info::StoreInfoRepository;
pub use label_template::LabelTemplateRepository;
pub use system_state::SystemStateRepository;
pub use tag::TagRepository;
pub use zone::ZoneRepository;
pub use image_ref::ImageRefRepository;
pub use shift::ShiftRepository;
pub use daily_report::DailyReportRepository;
pub use payment::PaymentRepository;
pub use system_issue::SystemIssueRepository;

use surrealdb::Surreal;
use surrealdb::engine::local::Db;
use thiserror::Error;
use shared::error::{AppError, ErrorCode};

/// Repository error types
#[derive(Debug, Error)]
pub enum RepoError {
    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Duplicate: {0}")]
    Duplicate(String),

    #[error("Database error: {0}")]
    Database(String),

    #[error("Validation error: {0}")]
    Validation(String),
}

impl From<surrealdb::Error> for RepoError {
    fn from(err: surrealdb::Error) -> Self {
        let msg = err.to_string();
        let lower = msg.to_lowercase();

        // Unique constraint / duplicate record
        if lower.contains("already exists")
            || lower.contains("duplicate")
            || lower.contains("unique")
        {
            return RepoError::Duplicate(msg);
        }

        // Record not found
        if lower.contains("not found")
            || lower.contains("no record")
            || lower.contains("can not find")
        {
            return RepoError::NotFound(msg);
        }

        RepoError::Database(msg)
    }
}

/// Convert a surrealdb::Error to AppError through RepoError's semantic mapping.
///
/// Use this instead of `.map_err(|e| AppError::database(e.to_string()))` to preserve
/// error semantics (NotFound → 404, Duplicate → 409, etc.)
pub fn surreal_err_to_app(err: surrealdb::Error) -> AppError {
    AppError::from(RepoError::from(err))
}

impl From<RepoError> for AppError {
    fn from(err: RepoError) -> Self {
        match err {
            RepoError::NotFound(msg) => AppError::not_found(msg),
            RepoError::Duplicate(msg) => AppError::with_message(ErrorCode::AlreadyExists, msg),
            RepoError::Database(msg) => AppError::database(msg),
            RepoError::Validation(msg) => AppError::validation(msg),
        }
    }
}

/// Result type for repository operations
pub type RepoResult<T> = Result<T, RepoError>;

// =============================================================================
// ID Convention: 全栈统一使用 "table:id" 格式
// =============================================================================
//
// 使用 surrealdb::RecordId 处理所有 ID：
//   - 解析: let id: RecordId = "product:abc".parse()?;
//   - 创建: let id = RecordId::from_table_key("product", "abc");
//   - 获取表名: id.table()
//   - 获取纯ID: id.key().to_string()
//   - CRUD: db.select(id) / db.delete(id) 直接使用 RecordId
//
// 禁止使用旧的 Thing 类型和 make_thing/strip_table_prefix 辅助函数

/// Base repository with database reference
#[derive(Clone)]
pub struct BaseRepository {
    db: Surreal<Db>,
}

impl BaseRepository {
    pub fn new(db: Surreal<Db>) -> Self {
        Self { db }
    }

    pub fn db(&self) -> &Surreal<Db> {
        &self.db
    }
}
