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

// System
pub mod system_state;

// Re-exports
pub use attribute::AttributeRepository;
pub use dining_table::DiningTableRepository;
pub use employee::EmployeeRepository;
pub use order::OrderRepository;
pub use price_rule::PriceRuleRepository;
pub use print_destination::PrintDestinationRepository;
pub use role::RoleRepository;
pub use system_state::SystemStateRepository;
pub use tag::TagRepository;
pub use zone::ZoneRepository;

use surrealdb::Surreal;
use surrealdb::engine::local::Db;
use thiserror::Error;

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
        RepoError::Database(err.to_string())
    }
}

/// Result type for repository operations
pub type RepoResult<T> = Result<T, RepoError>;

/// Common repository trait for basic CRUD
#[allow(async_fn_in_trait)]
pub trait Repository<T, CreateDto, UpdateDto> {
    async fn find_all(&self) -> RepoResult<Vec<T>>;
    async fn find_by_id(&self, id: &str) -> RepoResult<Option<T>>;
    async fn create(&self, data: CreateDto) -> RepoResult<T>;
    async fn update(&self, id: &str, data: UpdateDto) -> RepoResult<T>;
    async fn delete(&self, id: &str) -> RepoResult<bool>;
}

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
