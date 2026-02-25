//! Print job executor
//!
//! Handles the actual sending of print data to printers.

use std::collections::HashMap;

use chrono_tz::Tz;

use super::renderer::KitchenTicketRenderer;
use super::types::KitchenOrder;
use shared::models::{PrintDestination, Printer};
use thiserror::Error;
use tracing::{error, info, instrument, warn};

#[derive(Debug, Error)]
pub enum PrintExecutorError {
    #[error("No active printers for destination: {0}")]
    NoPrinters(String),

    #[error("Print failed: {0}")]
    PrintFailed(String),

    #[error("Printer offline: {0}")]
    PrinterOffline(String),
}

impl From<PrintExecutorError> for shared::error::AppError {
    fn from(err: PrintExecutorError) -> Self {
        use shared::error::{AppError, ErrorCode};
        match err {
            PrintExecutorError::NoPrinters(dest) => {
                AppError::with_message(ErrorCode::PrintNoPrintersConfigured, dest)
            }
            PrintExecutorError::PrintFailed(msg) => {
                AppError::with_message(ErrorCode::PrintFailed, msg)
            }
            PrintExecutorError::PrinterOffline(msg) => {
                AppError::with_message(ErrorCode::PrintAllPrintersOffline, msg)
            }
        }
    }
}

pub type PrintExecutorResult<T> = Result<T, PrintExecutorError>;

/// Print job executor
///
/// Sends rendered print data to physical printers.
pub struct PrintExecutor {
    renderer: KitchenTicketRenderer,
}

impl PrintExecutor {
    /// Create a new executor with default 80mm paper width and Europe/Madrid timezone
    pub fn new() -> Self {
        Self {
            renderer: KitchenTicketRenderer::default(),
        }
    }

    /// Create an executor with custom paper width and timezone
    pub fn with_config(width: usize, timezone: Tz) -> Self {
        Self {
            renderer: KitchenTicketRenderer::new(width, timezone),
        }
    }

    /// Execute a kitchen order print job
    ///
    /// Groups items by destination and sends to each printer.
    #[instrument(skip(self, order, destinations), fields(order_id = %order.id))]
    pub async fn print_kitchen_order(
        &self,
        order: &KitchenOrder,
        destinations: &HashMap<String, PrintDestination>,
    ) -> PrintExecutorResult<()> {
        // Group items by destination
        let grouped = self.group_by_destination(order);
        tracing::debug!(
            destination_count = grouped.len(),
            "print_kitchen_order: items grouped"
        );

        if grouped.is_empty() {
            info!("No items to print");
            return Ok(());
        }

        // Print to each destination
        for (dest_id, items) in grouped {
            let dest = match destinations.get(&dest_id) {
                Some(d) => d,
                None => {
                    warn!(dest_id = %dest_id, "Destination not found, skipping");
                    continue;
                }
            };

            // Create a sub-order with only items for this destination
            let sub_order = KitchenOrder {
                id: order.id.clone(),
                order_id: order.order_id.clone(),
                table_name: order.table_name.clone(),
                queue_number: order.queue_number,
                is_retail: order.is_retail,
                created_at: order.created_at,
                items,
                print_count: order.print_count,
            };

            // Render the ticket
            let data = self.renderer.render(&sub_order);

            // Send to printer
            if let Err(e) = self.send_to_destination(dest, &data).await {
                error!(dest = %dest.name, error = %e, "Failed to print");
                // Continue with other destinations even if one fails
            } else {
                info!(dest = %dest.name, bytes = data.len(), "Print job sent");
            }
        }

        Ok(())
    }

    /// Group items by their kitchen destination
    fn group_by_destination(
        &self,
        order: &KitchenOrder,
    ) -> HashMap<String, Vec<super::types::KitchenOrderItem>> {
        let mut groups: HashMap<String, Vec<super::types::KitchenOrderItem>> = HashMap::new();

        for item in &order.items {
            for dest_id in &item.context.kitchen_destinations {
                groups
                    .entry(dest_id.clone())
                    .or_default()
                    .push(item.clone());
            }
        }

        groups
    }

    /// Send data to a print destination
    async fn send_to_destination(
        &self,
        dest: &PrintDestination,
        data: &[u8],
    ) -> PrintExecutorResult<()> {
        // Find active printers, sorted by priority
        let mut printers: Vec<_> = dest.printers.iter().filter(|p| p.is_active).collect();

        printers.sort_by_key(|p| p.priority);

        if printers.is_empty() {
            return Err(PrintExecutorError::NoPrinters(dest.name.clone()));
        }

        // Try each printer until one succeeds
        let mut last_error = None;
        for printer in printers {
            match self.send_to_printer(printer, data).await {
                Ok(()) => return Ok(()),
                Err(e) => {
                    warn!(
                        printer = ?printer.driver_name.as_ref().or(printer.ip.as_ref()),
                        error = %e,
                        "Printer failed, trying next"
                    );
                    last_error = Some(e);
                }
            }
        }

        Err(last_error.unwrap_or_else(|| PrintExecutorError::NoPrinters(dest.name.clone())))
    }

    /// Send data to a specific printer
    async fn send_to_printer(&self, printer: &Printer, data: &[u8]) -> PrintExecutorResult<()> {
        tracing::debug!(
            connection = %printer.connection,
            driver = ?printer.driver_name,
            ip = ?printer.ip,
            bytes = data.len(),
            "send_to_printer: attempting"
        );
        match printer.connection.as_str() {
            "driver" => self.send_to_driver_printer(printer, data).await,
            "network" => self.send_to_network_printer(printer, data).await,
            other => {
                warn!(connection = other, "Unknown printer connection type");
                Err(PrintExecutorError::PrintFailed(format!(
                    "Unknown printer connection: {}",
                    other
                )))
            }
        }
    }

    /// Send to Windows driver printer
    #[cfg(windows)]
    async fn send_to_driver_printer(
        &self,
        printer: &Printer,
        data: &[u8],
    ) -> PrintExecutorResult<()> {
        use crab_printer::{Printer, WindowsPrinter};

        let driver_name = printer.driver_name.as_ref().ok_or_else(|| {
            PrintExecutorError::PrintFailed("No driver name specified".to_string())
        })?;

        let win_printer = WindowsPrinter::new(driver_name);
        win_printer
            .print(data)
            .await
            .map_err(|e| PrintExecutorError::PrintFailed(e.to_string()))
    }

    /// Fallback for non-Windows: driver printing not supported
    #[cfg(not(windows))]
    async fn send_to_driver_printer(
        &self,
        _printer: &Printer,
        _data: &[u8],
    ) -> PrintExecutorResult<()> {
        Err(PrintExecutorError::PrintFailed(
            "Driver printing not supported on this platform".to_string(),
        ))
    }

    /// Send to network printer (TCP 9100)
    async fn send_to_network_printer(
        &self,
        printer: &Printer,
        data: &[u8],
    ) -> PrintExecutorResult<()> {
        use crab_printer::{NetworkPrinter, Printer};

        let ip = printer
            .ip
            .as_ref()
            .ok_or_else(|| PrintExecutorError::PrintFailed("No IP specified".to_string()))?;

        let port = printer.port.unwrap_or(9100) as u16;

        let net_printer = NetworkPrinter::new(ip, port)
            .map_err(|e| PrintExecutorError::PrintFailed(e.to_string()))?;

        net_printer
            .print(data)
            .await
            .map_err(|e| PrintExecutorError::PrintFailed(e.to_string()))
    }

    /// Print label records (Windows: GDI+ rendering via crab-printer)
    #[cfg(windows)]
    #[instrument(skip(self, records, destinations, template), fields(record_count = records.len()))]
    pub async fn print_label_records(
        &self,
        records: &[super::types::LabelPrintRecord],
        destinations: &HashMap<String, PrintDestination>,
        template: &crab_printer::label::LabelTemplate,
    ) -> PrintExecutorResult<()> {
        for record in records {
            tracing::debug!(
                record_id = %record.id,
                product = %record.context.product_name,
                label_dests = record.context.label_destinations.len(),
                "print_label_records: processing record"
            );
            for dest_id in &record.context.label_destinations {
                let dest = match destinations.get(dest_id) {
                    Some(d) => d,
                    None => {
                        warn!(dest_id = %dest_id, "Label destination not found, skipping");
                        continue;
                    }
                };

                // 找活跃的驱动打印机（优先级排序）
                let printer = dest
                    .printers
                    .iter()
                    .filter(|p| p.is_active && p.connection == "driver")
                    .min_by_key(|p| p.priority);

                let Some(printer) = printer else {
                    warn!(dest = %dest.name, "No active driver printer for labels");
                    continue;
                };

                let driver_name = match &printer.driver_name {
                    Some(name) => name.clone(),
                    None => {
                        warn!("Label printer has no driver_name");
                        continue;
                    }
                };

                let data = build_label_data(record);

                let options = crab_printer::label::PrintOptions {
                    printer_name: Some(driver_name),
                    doc_name: "label".to_string(),
                    label_width_mm: template.width_mm,
                    label_height_mm: template.height_mm,
                    copies: 1,
                    fit: crab_printer::label::FitMode::Contain,
                    rotate: crab_printer::label::Rotation::R0,
                    override_dpi: None,
                };

                let template_clone = template.clone();
                let result = tokio::task::spawn_blocking(move || {
                    crab_printer::label::render_and_print_label(
                        &data,
                        Some(&template_clone),
                        &options,
                    )
                })
                .await;

                match result {
                    Ok(Ok(())) => {
                        info!(
                            record_id = %record.id,
                            product = %record.context.product_name,
                            index = ?record.context.index,
                            "Label printed"
                        );
                        break; // 成功打印，不再尝试其他目的地
                    }
                    Ok(Err(e)) => {
                        error!(
                            record_id = %record.id,
                            dest = %dest.name,
                            error = %e,
                            "Label print failed"
                        );
                    }
                    Err(e) => {
                        error!(record_id = %record.id, error = %e, "Label print task panicked");
                    }
                }
            }
        }
        Ok(())
    }

    /// Print label records (non-Windows: not supported, GDI+ required)
    #[cfg(not(windows))]
    pub async fn print_label_records(
        &self,
        _records: &[super::types::LabelPrintRecord],
        _destinations: &HashMap<String, PrintDestination>,
    ) -> PrintExecutorResult<()> {
        warn!("Label printing requires Windows (GDI+)");
        Ok(())
    }
}

/// Build label data JSON from LabelPrintRecord
#[cfg(windows)]
fn build_label_data(record: &super::types::LabelPrintRecord) -> serde_json::Value {
    let ctx = &record.context;
    let time = chrono::Local::now().format("%H:%M").to_string();
    serde_json::json!({
        "product_name": ctx.product_name,
        "kitchen_name": ctx.kitchen_name,
        "item_name": ctx.kitchen_name,
        "category_name": ctx.category_name,
        "spec_name": ctx.spec_name.as_deref().unwrap_or(""),
        "specs": ctx.spec_name.as_deref().unwrap_or(""),
        "options": ctx.options.join(", "),
        "quantity": ctx.quantity,
        "index": ctx.index.as_deref().unwrap_or(""),
        "note": ctx.note.as_deref().unwrap_or(""),
        "external_id": ctx.external_id.unwrap_or(0),
        "table_name": record.table_name.as_deref().unwrap_or(""),
        "queue_number": record.queue_number.map(|n| format!("#{:03}", n)).unwrap_or_default(),
        "time": time,
    })
}

/// Convert DB LabelTemplate to crab-printer LabelTemplate
#[cfg(windows)]
pub fn convert_label_template(
    db_template: &shared::models::LabelTemplate,
) -> crab_printer::label::LabelTemplate {
    let fields = db_template
        .fields
        .iter()
        .filter(|f| f.visible)
        .filter_map(convert_label_field)
        .collect();

    crab_printer::label::LabelTemplate {
        width_mm: db_template.width_mm.unwrap_or(db_template.width),
        height_mm: db_template.height_mm.unwrap_or(db_template.height),
        padding_mm_x: db_template.padding_mm_x.unwrap_or(0.0),
        padding_mm_y: db_template.padding_mm_y.unwrap_or(0.0),
        fields,
    }
}

#[cfg(windows)]
fn convert_label_field(
    f: &shared::models::LabelField,
) -> Option<crab_printer::label::TemplateField> {
    use shared::models::LabelFieldType;
    match f.field_type {
        LabelFieldType::Text
        | LabelFieldType::Datetime
        | LabelFieldType::Price
        | LabelFieldType::Counter => Some(crab_printer::label::TemplateField::Text(
            crab_printer::label::TextField {
                x: f.x,
                y: f.y,
                width: f.width,
                height: f.height,
                font_size: f.font_size as f32,
                font_family: f.font_family.clone(),
                style: if f.font_weight.as_deref() == Some("bold") {
                    crab_printer::label::TextStyle::Bold
                } else {
                    crab_printer::label::TextStyle::Regular
                },
                align: match f.alignment {
                    Some(shared::models::LabelFieldAlignment::Center) => {
                        crab_printer::label::TextAlign::Center
                    }
                    Some(shared::models::LabelFieldAlignment::Right) => {
                        crab_printer::label::TextAlign::Right
                    }
                    _ => crab_printer::label::TextAlign::Left,
                },
                template: f
                    .template
                    .clone()
                    .or_else(|| f.data_key.as_ref().map(|k| format!("{{{}}}", k)))
                    .unwrap_or_default(),
            },
        )),
        LabelFieldType::Image | LabelFieldType::Barcode | LabelFieldType::Qrcode => Some(
            crab_printer::label::TemplateField::Image(crab_printer::label::ImageField {
                x: f.x,
                y: f.y,
                width: f.width,
                height: f.height,
                maintain_aspect_ratio: f.maintain_aspect_ratio.unwrap_or(true),
                data_key: f.data_key.clone().unwrap_or_else(|| f.name.clone()),
            }),
        ),
        LabelFieldType::Separator => Some(crab_printer::label::TemplateField::Separator(
            crab_printer::label::SeparatorField {
                y: f.y,
                x_start: Some(f.x),
                x_end: Some(f.x + f.width),
            },
        )),
    }
}

impl Default for PrintExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::printing::types::{KitchenOrderItem, PrintItemContext};

    fn create_test_order() -> KitchenOrder {
        KitchenOrder {
            id: "evt-1".to_string(),
            order_id: "order-1".to_string(),
            table_name: Some("100桌".to_string()),
            queue_number: None,
            is_retail: false,
            created_at: 1705912335000, // 2024-01-22 14:32:15 UTC (millis)
            items: vec![KitchenOrderItem {
                context: PrintItemContext {
                    category_id: 1,
                    category_name: "热菜".to_string(),
                    product_id: 1,
                    external_id: Some(1),
                    kitchen_name: "宫保鸡丁".to_string(),
                    product_name: "宫保鸡丁".to_string(),
                    spec_name: None,
                    quantity: 2,
                    index: None,
                    options: vec![],
                    label_options: vec![],
                    note: None,
                    kitchen_destinations: vec!["dest-1".to_string()],
                    label_destinations: vec![],
                },
            }],
            print_count: 0,
        }
    }

    #[test]
    fn test_group_by_destination() {
        let executor = PrintExecutor::new();
        let order = create_test_order();

        let grouped = executor.group_by_destination(&order);

        assert_eq!(grouped.len(), 1);
        assert!(grouped.contains_key("dest-1"));
        assert_eq!(grouped.get("dest-1").unwrap().len(), 1);
    }
}
