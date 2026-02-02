//! Printer module - delegates to crab-printer
//!
//! This module provides a high-level printing API for the Tauri application,
//! using crab-printer for low-level printer operations.

use tracing::instrument;

#[cfg(target_os = "windows")]
mod platform {
    use crate::utils::escpos_text::{convert_mixed_utf8_to_gbk, process_logo};
    use crate::utils::receipt_renderer::ReceiptRenderer;
    use crab_printer::WindowsPrinter;
    use tracing::{error, info, warn};

    pub fn list_printers() -> Result<Vec<String>, String> {
        WindowsPrinter::list().map_err(|e| e.to_string())
    }

    pub fn resolve_printer(printer_name: Option<String>) -> Result<String, String> {
        WindowsPrinter::resolve(printer_name.as_deref()).map_err(|e| {
            error!(error = %e, "resolve_printer failed");
            e.to_string()
        })
    }

    pub fn print_raw_bytes(printer_name: Option<String>, bytes: Vec<u8>) -> Result<(), String> {
        let name = resolve_printer(printer_name)?;
        info!(printer = name, "printing raw bytes");

        let final_bytes = convert_mixed_utf8_to_gbk(&bytes);
        let printer = WindowsPrinter::new(&name);

        tokio::runtime::Handle::current()
            .block_on(async {
                use crab_printer::Printer;
                printer.print(&final_bytes).await
            })
            .map_err(|e| e.to_string())
    }

    pub fn open_cash_drawer(printer_name: Option<String>) -> Result<(), String> {
        let name = resolve_printer(printer_name)?;
        info!(printer = name, "open cash drawer");

        let printer = WindowsPrinter::new(&name);
        printer.open_cash_drawer().map_err(|e| e.to_string())
    }

    pub fn print_receipt(
        printer_name: Option<String>,
        receipt: crate::api::ReceiptData,
    ) -> Result<(), String> {
        let name = resolve_printer(printer_name)?;
        info!(printer = name, "printing receipt");

        // Initialize printer (ESC @)
        let mut data: Vec<u8> = vec![0x1B, 0x40];

        // Process logo if available
        if let Some(info) = &receipt.store_info {
            if let Some(logo_path) = &info.logo_url {
                if !logo_path.is_empty() {
                    if let Some(logo_bytes) = process_logo(logo_path) {
                        info!(bytes = logo_bytes.len(), "sending logo data");
                        data.extend_from_slice(&logo_bytes);
                    } else {
                        warn!("process_logo returned None");
                    }
                }
            }
        }

        // Render receipt content
        let output = ReceiptRenderer::new(&receipt, 48).render();
        let text_bytes = convert_mixed_utf8_to_gbk(output.as_bytes());
        data.extend_from_slice(&text_bytes);

        // Print using crab-printer
        let printer = WindowsPrinter::new(&name);
        tokio::runtime::Handle::current()
            .block_on(async {
                use crab_printer::Printer;
                printer.print(&data).await
            })
            .map_err(|e| e.to_string())
    }
}

#[cfg(not(target_os = "windows"))]
mod platform {
    pub fn list_printers() -> Result<Vec<String>, String> {
        Ok(Vec::new())
    }

    pub fn resolve_printer(printer_name: Option<String>) -> Result<String, String> {
        if let Some(name) = printer_name {
            Ok(name)
        } else {
            Err("PRINTING_NOT_SUPPORTED".to_string())
        }
    }

    pub fn print_raw_bytes(
        _printer_name: Option<String>,
        _bytes: Vec<u8>,
    ) -> Result<(), String> {
        Err("PRINTING_NOT_SUPPORTED".to_string())
    }

    pub fn open_cash_drawer(_printer_name: Option<String>) -> Result<(), String> {
        Err("PRINTING_NOT_SUPPORTED".to_string())
    }

    pub fn print_receipt(
        _printer_name: Option<String>,
        _receipt: crate::api::ReceiptData,
    ) -> Result<(), String> {
        Err("PRINTING_NOT_SUPPORTED".to_string())
    }
}

#[instrument]
pub fn list_printers() -> Result<Vec<String>, String> {
    platform::list_printers()
}

#[instrument(skip(printer_name))]
pub fn resolve_printer(printer_name: Option<String>) -> Result<String, String> {
    platform::resolve_printer(printer_name)
}

#[instrument(skip(bytes))]
pub fn print_raw_bytes(printer_name: Option<String>, bytes: Vec<u8>) -> Result<(), String> {
    platform::print_raw_bytes(printer_name, bytes)
}

#[instrument(skip(printer_name))]
pub fn open_cash_drawer(printer_name: Option<String>) -> Result<(), String> {
    platform::open_cash_drawer(printer_name)
}

#[instrument(skip(receipt))]
pub fn print_receipt(
    printer_name: Option<String>,
    receipt: crate::api::ReceiptData,
) -> Result<(), String> {
    platform::print_receipt(printer_name, receipt)
}
