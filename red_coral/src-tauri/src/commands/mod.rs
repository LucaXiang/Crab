//! Tauri Commands for RedCoral POS
//!
//! 提供前端调用的 Tauri 命令接口

pub mod api;
pub mod auth;
pub mod data;
pub mod location;
pub mod mode;
pub mod orders;
pub mod system;
pub mod tenant;

pub use api::*;
pub use auth::*;
pub use data::*;
pub use location::*;
pub use mode::*;
pub use orders::*;
pub use system::*;
pub use tenant::*;
