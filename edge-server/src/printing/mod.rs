//! Kitchen and Label Printing Module
//!
//! This module handles automatic printing on ItemsAdded events:
//! - Kitchen printing: grouped by destination, sent to kitchen printers
//! - Label printing: per-item labels (e.g., bubble tea stickers)

pub mod credit_note_renderer;
pub mod executor;
pub mod renderer;
pub mod service;
pub mod storage;
pub mod types;
pub mod worker;

pub use credit_note_renderer::CreditNoteReceiptRenderer;
pub use executor::{LabelContext, PrintExecutor, PrintExecutorError, PrintExecutorResult};
pub use renderer::KitchenTicketRenderer;
pub use service::{KitchenPrintService, PrintServiceError, PrintServiceResult};
pub use storage::{PrintStorage, PrintStorageError, PrintStorageResult};
pub use types::*;
pub use worker::KitchenPrintWorker;
