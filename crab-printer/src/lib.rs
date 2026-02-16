//! # crab-printer
//!
//! ESC/POS thermal printer library - low-level printing capabilities only.
//!
//! ## Scope
//!
//! This crate handles HOW to print:
//! - ESC/POS command building
//! - GBK encoding for Chinese printers
//! - Network printing (TCP port 9100)
//! - Windows driver printing (optional)
//! - Image/logo processing
//!
//! Business logic (WHAT to print) should stay in application code:
//! - Receipt rendering → red_coral
//! - Kitchen ticket rendering → edge-server
//!
//! ## Example
//!
//! ```ignore
//! use crab_printer::{EscPosBuilder, NetworkPrinter, Printer};
//!
//! // Build ESC/POS content
//! let mut builder = EscPosBuilder::new(48);
//! builder.center();
//! builder.double_size();
//! builder.line("厨房单");
//! builder.reset_size();
//! builder.sep_double();
//! builder.left();
//! builder.line("桌号: 100");
//! builder.cut();
//!
//! // Send to network printer
//! let printer = NetworkPrinter::new("192.168.1.100", 9100);
//! printer.print(&builder.build()).await?;
//! ```

mod encoding;
mod error;
mod escpos;
mod printer;

// Re-exports
pub use encoding::{convert_to_gbk, gbk_width, pad_gbk, truncate_gbk};
pub use error::{PrintError, PrintResult};
pub use escpos::{EscPosBuilder, EscPosTextBuilder};
pub use printer::{NetworkPrinter, Printer};

#[cfg(feature = "image")]
pub use escpos::process_logo;

#[cfg(windows)]
pub use printer::WindowsPrinter;
