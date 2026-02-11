//! Data models
//!
//! Shared between edge-server and frontend (via API).
//! DB row types use `#[cfg_attr(feature = "db", derive(sqlx::FromRow))]`.
//! All IDs are `i64` (SQLite INTEGER PRIMARY KEY).

pub mod attribute;
pub mod category;
pub mod daily_report;
pub mod dining_table;
pub mod employee;
pub mod label_template;
pub mod price_rule;
pub mod print_destination;
pub mod product;
pub mod role;
pub mod shift;
pub mod store_info;
pub mod sync;
pub mod system_issue;
pub mod system_state;
pub mod tag;
pub mod zone;
pub mod image_ref;
pub mod marketing_group;
pub mod member;
pub mod stamp;

// Re-exports
pub use attribute::*;
pub use category::*;
pub use daily_report::*;
pub use dining_table::*;
pub use employee::*;
pub use image_ref::*;
pub use label_template::*;
pub use price_rule::*;
pub use print_destination::*;
pub use product::*;
pub use role::*;
pub use shift::*;
pub use store_info::*;
pub use sync::*;
pub use system_issue::*;
pub use system_state::*;
pub use tag::*;
pub use zone::*;
pub use marketing_group::*;
pub use member::*;
pub use stamp::*;
