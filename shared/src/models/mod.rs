//! 数据模型 (API DTOs)
//!
//! 这些类型用于 REST API 请求/响应，可被客户端直接使用。
//! 所有 ID 字段使用 String 类型，与数据库实现解耦。

pub mod attribute;
pub mod category;
pub mod dining_table;
pub mod employee;
pub mod order;
pub mod price_rule;
pub mod print_destination;
pub mod product;
pub mod sync;
pub mod system_state;
pub mod tag;
pub mod zone;

// Re-exports
pub use attribute::*;
pub use category::*;
pub use dining_table::*;
pub use employee::*;
pub use order::*;
pub use price_rule::*;
pub use print_destination::*;
pub use product::*;
pub use sync::*;
pub use system_state::*;
pub use tag::*;
pub use zone::*;
