//! ESC/POS utilities - re-exports from crab-printer
//!
//! This module re-exports the ESC/POS building and encoding utilities
//! from crab-printer with backward-compatible names.

// Re-export GBK encoding utilities
pub use crab_printer::convert_to_gbk as convert_mixed_utf8_to_gbk;
pub use crab_printer::gbk_width as get_gbk_width;
pub use crab_printer::pad_gbk as pad_to_gbk_width;
pub use crab_printer::truncate_gbk as truncate_to_gbk_width;

// Re-export ESC/POS text builder
pub use crab_printer::EscPosTextBuilder;

// Re-export logo processing
pub use crab_printer::process_logo;
