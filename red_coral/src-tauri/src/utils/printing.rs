//! Printer adapter module - delegates to crab-printer
//!
//! This module provides a high-level printing API for the Tauri application,
//! using crab-printer for low-level printer operations.

use tracing::{instrument, warn};
#[cfg(target_os = "windows")]
use tracing::{error, info};

pub trait PrinterAdapter {
    fn list_printers(&self) -> Result<Vec<String>, String>;
    fn resolve_printer(&self, printer_name: Option<String>) -> Result<String, String>;
    fn print_raw_bytes(&self, printer_name: Option<String>, bytes: Vec<u8>) -> Result<(), String>;
    fn open_cash_drawer(&self, printer_name: Option<String>) -> Result<(), String>;
    fn print_receipt(
        &self,
        printer_name: Option<String>,
        receipt: crate::api::ReceiptData,
    ) -> Result<(), String>;
}

#[cfg(target_os = "windows")]
mod windows_adapter {
    use super::PrinterAdapter;
    use crate::utils::escpos_text::{convert_mixed_utf8_to_gbk, process_logo};
    use crate::utils::receipt_renderer::ReceiptRenderer;
    use crab_printer::WindowsPrinter;
    use tracing::{error, info, warn};

    pub struct WindowsPrinterAdapter;

    impl PrinterAdapter for WindowsPrinterAdapter {
        fn list_printers(&self) -> Result<Vec<String>, String> {
            WindowsPrinter::list().map_err(|e| e.to_string())
        }

        fn resolve_printer(&self, printer_name: Option<String>) -> Result<String, String> {
            WindowsPrinter::resolve(printer_name.as_deref()).map_err(|e| {
                error!(error = %e, "resolve_printer failed");
                e.to_string()
            })
        }

        fn print_raw_bytes(
            &self,
            printer_name: Option<String>,
            bytes: Vec<u8>,
        ) -> Result<(), String> {
            let name = self.resolve_printer(printer_name)?;
            info!(printer = name, "printing raw bytes");

            // Convert to GBK
            let final_bytes = convert_mixed_utf8_to_gbk(&bytes);

            // Use crab-printer's WindowsPrinter
            let printer = WindowsPrinter::new(&name);

            // Note: crab-printer's print() is async, but we need sync here
            // Use the blocking write_raw approach via a runtime
            tokio::runtime::Handle::current()
                .block_on(async {
                    use crab_printer::Printer;
                    printer.print(&final_bytes).await
                })
                .map_err(|e| e.to_string())
        }

        fn open_cash_drawer(&self, printer_name: Option<String>) -> Result<(), String> {
            let name = self.resolve_printer(printer_name)?;
            info!(printer = name, "open cash drawer");

            let printer = WindowsPrinter::new(&name);
            printer.open_cash_drawer().map_err(|e| e.to_string())
        }

        fn print_receipt(
            &self,
            printer_name: Option<String>,
            receipt: crate::api::ReceiptData,
        ) -> Result<(), String> {
            let name = self.resolve_printer(printer_name)?;
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

    pub fn current_adapter() -> WindowsPrinterAdapter {
        WindowsPrinterAdapter
    }
}

#[cfg(not(target_os = "windows"))]
mod fallback_adapter {
    use super::PrinterAdapter;

    pub struct UnsupportedPrinterAdapter;

    impl PrinterAdapter for UnsupportedPrinterAdapter {
        fn list_printers(&self) -> Result<Vec<String>, String> {
            Ok(Vec::new())
        }

        fn resolve_printer(&self, printer_name: Option<String>) -> Result<String, String> {
            if let Some(name) = printer_name {
                Ok(name)
            } else {
                Err("PRINTING_NOT_SUPPORTED".to_string())
            }
        }

        fn print_raw_bytes(
            &self,
            _printer_name: Option<String>,
            _bytes: Vec<u8>,
        ) -> Result<(), String> {
            Err("PRINTING_NOT_SUPPORTED".to_string())
        }

        fn open_cash_drawer(&self, _printer_name: Option<String>) -> Result<(), String> {
            Err("PRINTING_NOT_SUPPORTED".to_string())
        }

        fn print_receipt(
            &self,
            _printer_name: Option<String>,
            _receipt: crate::api::ReceiptData,
        ) -> Result<(), String> {
            Err("PRINTING_NOT_SUPPORTED".to_string())
        }
    }

    pub fn current_adapter() -> UnsupportedPrinterAdapter {
        UnsupportedPrinterAdapter
    }
}

#[cfg(not(target_os = "windows"))]
use fallback_adapter::current_adapter;
#[cfg(target_os = "windows")]
use windows_adapter::current_adapter;

#[instrument]
pub fn list_printers() -> Result<Vec<String>, String> {
    current_adapter().list_printers()
}

#[instrument(skip(printer_name))]
pub fn resolve_printer(printer_name: Option<String>) -> Result<String, String> {
    current_adapter().resolve_printer(printer_name)
}

#[instrument(skip(bytes))]
pub fn print_raw_bytes(printer_name: Option<String>, bytes: Vec<u8>) -> Result<(), String> {
    current_adapter().print_raw_bytes(printer_name, bytes)
}

#[instrument(skip(printer_name))]
pub fn open_cash_drawer(printer_name: Option<String>) -> Result<(), String> {
    current_adapter().open_cash_drawer(printer_name)
}

#[instrument(skip(receipt))]
pub fn print_receipt(
    printer_name: Option<String>,
    receipt: crate::api::ReceiptData,
) -> Result<(), String> {
    current_adapter().print_receipt(printer_name, receipt)
}
