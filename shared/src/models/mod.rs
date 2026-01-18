//! 数据模型 (API DTOs)
//!
//! 这些类型用于 REST API 请求/响应，可被客户端直接使用。
//! 所有 ID 字段使用 String 类型，与数据库实现解耦。

pub mod tag;
pub mod category;
pub mod product;
pub mod attribute;
pub mod kitchen_printer;
pub mod zone;
pub mod dining_table;
pub mod price_rule;
pub mod employee;
pub mod order;
pub mod system_state;

// Re-exports
pub use tag::*;
pub use category::*;
pub use product::*;
pub use attribute::*;
pub use kitchen_printer::*;
pub use zone::*;
pub use dining_table::*;
pub use price_rule::*;
pub use employee::*;
pub use order::*;
pub use system_state::*;
