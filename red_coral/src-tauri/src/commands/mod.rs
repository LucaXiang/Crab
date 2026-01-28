//! Tauri Commands for RedCoral POS
//!
//! 提供前端调用的 Tauri 命令接口

pub mod api;
pub mod auth;
pub mod backup;
pub mod data;
pub mod health;
pub mod image;
pub mod location;
pub mod mode;
pub mod order_es;
pub mod orders;
pub mod printer;
pub mod sync;
pub mod system;
pub mod tenant;
pub mod shift;
pub mod statistics;

pub use api::*;
pub use auth::*;
pub use backup::*;
pub use data::*;
pub use health::*;
pub use image::*;
pub use location::*;
pub use mode::*;
pub use order_es::*;
pub use orders::*;
pub use printer::*;
pub use sync::*;
pub use system::*;
pub use tenant::*;
pub use shift::*;
pub use statistics::*;
