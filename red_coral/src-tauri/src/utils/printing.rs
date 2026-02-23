//! Printer module - delegates to crab-printer
//!
//! This module provides a high-level printing API for the Tauri application,
//! using crab-printer for low-level printer operations.

use crate::commands::printer::LabelPrintRequest;
use tracing::instrument;

#[cfg(target_os = "windows")]
mod platform {
    use crate::commands::printer::LabelPrintRequest;
    use crate::utils::escpos_text::{convert_mixed_utf8_to_gbk, process_logo};
    use crate::utils::label_printer::{LabelTemplate, PrintOptions};
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
        printer.print_sync(&final_bytes).map_err(|e| e.to_string())
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
        tracing::debug!(
            has_logo = receipt
                .store_info
                .as_ref()
                .and_then(|i| i.logo_url.as_ref())
                .is_some(),
            "print_receipt: starting render"
        );

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
        tracing::debug!(
            rendered_bytes = output.len(),
            gbk_bytes = text_bytes.len(),
            total_bytes = data.len() + text_bytes.len(),
            "print_receipt: rendered and encoded"
        );
        data.extend_from_slice(&text_bytes);

        // Print using crab-printer (sync — no async overhead needed)
        let printer = WindowsPrinter::new(&name);
        printer.print_sync(&data).map_err(|e| e.to_string())
    }

    pub fn print_label(request: LabelPrintRequest) -> Result<(), String> {
        let template: Option<LabelTemplate> = request
            .template
            .and_then(|v| serde_json::from_value(v).ok());

        let options = PrintOptions {
            printer_name: request.printer_name,
            doc_name: "label".to_string(),
            label_width_mm: request.label_width_mm.unwrap_or(40.0),
            label_height_mm: request.label_height_mm.unwrap_or(30.0),
            copies: 1,
            fit: crate::utils::label_printer::FitMode::Contain,
            rotate: crate::utils::label_printer::Rotation::R0,
            override_dpi: request.override_dpi,
        };

        info!(?options, "printing label");
        tracing::debug!(
            has_template = template.is_some(),
            "print_label: calling render_and_print_label"
        );
        crate::utils::label_printer::render_and_print_label(
            &request.data,
            template.as_ref(),
            &options,
        )
        .map_err(|e| format!("{}", e))
    }
}

#[cfg(not(target_os = "windows"))]
mod platform {
    use crate::commands::printer::LabelPrintRequest;

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

    pub fn print_raw_bytes(_printer_name: Option<String>, _bytes: Vec<u8>) -> Result<(), String> {
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

    pub fn print_label(_request: LabelPrintRequest) -> Result<(), String> {
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

#[instrument(skip(request))]
pub fn print_label(request: LabelPrintRequest) -> Result<(), String> {
    platform::print_label(request)
}
