//! Kitchen and Label Printing Module
//!
//! This module handles automatic printing on ItemsAdded events:
//! - Kitchen printing: grouped by destination, sent to kitchen printers
//! - Label printing: per-item labels (e.g., bubble tea stickers)

pub mod cache;
pub mod types;

pub use cache::PrintConfigCache;
pub use types::*;
