//! Repository Module
//!
//! Provides CRUD operations for SurrealDB tables using Graph DB patterns.

// Auth
pub mod employee;
pub mod role;

// Product Domain
pub mod tag;
pub mod category;
pub mod product;
pub mod attribute;
pub mod print_destination;

// Location
pub mod zone;
pub mod dining_table;

// Pricing
pub mod price_rule;

// Orders
pub mod order;

// System
pub mod system_state;

// Re-exports
pub use employee::EmployeeRepository;
pub use role::RoleRepository;
pub use tag::TagRepository;
pub use category::CategoryRepository;
pub use product::ProductRepository;
pub use attribute::AttributeRepository;
pub use print_destination::PrintDestinationRepository;
pub use zone::ZoneRepository;
pub use dining_table::DiningTableRepository;
pub use price_rule::PriceRuleRepository;
pub use order::OrderRepository;
pub use system_state::SystemStateRepository;

use surrealdb::engine::local::Db;
use surrealdb::Surreal;
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

/// Helper to create Thing from table:id string
pub fn parse_thing(id: &str) -> Option<surrealdb::sql::Thing> {
    let parts: Vec<&str> = id.split(':').collect();
    if parts.len() == 2 {
        Some(surrealdb::sql::Thing::from((parts[0].to_string(), parts[1].to_string())))
    } else {
        None
    }
}

/// Helper to create Thing from table name and id
pub fn make_thing(table: &str, id: &str) -> surrealdb::sql::Thing {
    surrealdb::sql::Thing::from((table.to_string(), id.to_string()))
}

/// Strip table prefix from id (e.g., "product:xxx" -> "xxx")
pub fn strip_table_prefix<'a>(table: &str, id: &'a str) -> &'a str {
    let prefix = format!("{}:", table);
    id.strip_prefix(&prefix).unwrap_or(id)
}

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
