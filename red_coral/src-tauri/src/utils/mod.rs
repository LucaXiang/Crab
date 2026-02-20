//! Utility module for helper functions and infrastructure code.

pub mod escpos_text;
#[cfg(target_os = "windows")]
pub mod label_printer;
pub mod printing;
pub mod receipt_renderer;
