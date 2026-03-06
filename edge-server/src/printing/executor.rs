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

    /// Create an executor with custom paper width, timezone, and locale
    pub fn with_config(width: usize, timezone: Tz, locale: String) -> Self {
        Self {
            renderer: KitchenTicketRenderer::new(width, timezone, locale),
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
                id: order.id,
                order_id: order.order_id,
                receipt_number: order.receipt_number.clone(),
                table_name: order.table_name.clone(),
                zone_name: order.zone_name.clone(),
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
    #[instrument(skip(self, records, destinations, template, db_fields, label_ctx), fields(record_count = records.len()))]
    pub async fn print_label_records(
        &self,
        records: &[super::types::LabelPrintRecord],
        destinations: &HashMap<String, PrintDestination>,
        template: &crab_printer::label::LabelTemplate,
        db_fields: &[shared::models::LabelField],
        label_ctx: &LabelContext,
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

                let mut data = build_label_data(record, label_ctx);
                inject_static_images(&mut data, db_fields, label_ctx.images_dir.as_deref());

                // Paper size must include padding for correct positioning
                let options = crab_printer::label::PrintOptions {
                    printer_name: Some(driver_name),
                    doc_name: "label".to_string(),
                    label_width_mm: template.width_mm + template.padding_mm_x,
                    label_height_mm: template.height_mm + template.padding_mm_y,
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
}

/// Extra context for label data rendering (store info, image paths)
pub struct LabelContext {
    pub store_name: String,
    pub store_address: String,
    pub store_phone: String,
    pub store_nif: String,
    /// Base64 data URI of store logo (pre-loaded)
    pub store_logo: Option<String>,
    /// Images directory for loading static image fields
    pub images_dir: Option<std::path::PathBuf>,
}

impl LabelContext {
    /// Build from StoreInfo + images directory
    pub fn from_store_info(
        info: Option<&shared::models::StoreInfo>,
        images_dir: Option<&std::path::Path>,
    ) -> Self {
        let (name, address, phone, nif, logo_url) = match info {
            Some(i) => (
                i.name.clone(),
                i.address.clone(),
                i.phone.clone().unwrap_or_default(),
                i.nif.clone(),
                i.logo_url.clone(),
            ),
            None => Default::default(),
        };

        // Pre-load store logo as base64 data URI
        let store_logo = logo_url.filter(|u| !u.is_empty()).and_then(|url| {
            let path = images_dir?.join(&url);
            let bytes = std::fs::read(&path).ok()?;
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("png");
            let mime = match ext {
                "jpg" | "jpeg" => "image/jpeg",
                "webp" => "image/webp",
                _ => "image/png",
            };
            use base64::Engine;
            let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
            Some(format!("data:{};base64,{}", mime, b64))
        });

        Self {
            store_name: name,
            store_address: address,
            store_phone: phone,
            store_nif: nif,
            store_logo,
            images_dir: images_dir.map(|p| p.to_path_buf()),
        }
    }
}

/// Build label data JSON from LabelPrintRecord.
///
/// Keys here are the single source of truth — they must match
/// `SUPPORTED_LABEL_FIELDS.key` in the frontend and `data_source` values
/// in label templates. To add a new field:
/// 1. Add the key here
/// 2. Add to `SUPPORTED_LABEL_FIELDS` in `labelTemplate.ts`
/// 3. Populate from `PrintItemContext` (extend if needed)
#[cfg(windows)]
fn build_label_data(
    record: &super::types::LabelPrintRecord,
    label_ctx: &LabelContext,
) -> serde_json::Value {
    let ctx = &record.context;
    let now = chrono::Local::now();
    let mut data = serde_json::json!({
        // Product
        "product_id": ctx.product_id,
        "product_name": ctx.product_name,
        "kitchen_name": ctx.kitchen_name,
        "category_id": ctx.category_id,
        "category_name": ctx.category_name,
        "external_id": ctx.external_id.unwrap_or(0),
        // Specification
        "spec_name": ctx.spec_name.as_deref().unwrap_or(""),
        // Item
        "price": ctx.price,
        "quantity": ctx.quantity,
        "subtotal": ctx.price * ctx.quantity as f64,
        "index": ctx.index.as_deref().unwrap_or(""),
        "options": ctx.label_options.join(", "),
        "kitchen_options": ctx.options.join(", "),
        "note": ctx.note.as_deref().unwrap_or(""),
        // Order
        "order_id": record.order_id,
        "receipt_number": record.receipt_number,
        "table_name": record.table_name.as_deref().unwrap_or(""),
        "zone_name": record.zone_name.as_deref().unwrap_or(""),
        "queue_number": record.queue_number.map(|n| format!("#{:03}", n)).unwrap_or_default(),
        "is_retail": record.is_retail,
        // Store
        "store_name": label_ctx.store_name,
        "store_address": label_ctx.store_address,
        "store_phone": label_ctx.store_phone,
        "store_nif": label_ctx.store_nif,
        // Print
        "print_count": record.print_count,
        "weekday": now.format("%A").to_string(),
        "time": now.format("%H:%M").to_string(),
        "date": now.format("%Y-%m-%d").to_string(),
        "datetime": now.format("%Y-%m-%d %H:%M").to_string(),
    });
    // Store logo (image, only if loaded)
    if let Some(ref logo) = label_ctx.store_logo {
        data["store_logo"] = serde_json::Value::String(logo.clone());
    }
    data
}

/// Inject static image fields into label data.
///
/// For image fields with `source_type == "image"` and a non-empty `template` (image hash),
/// load the image from disk and inject as Base64 data URI into the data JSON.
/// This enables user-uploaded static images (e.g. custom logos) to print correctly.
#[cfg(windows)]
fn inject_static_images(
    data: &mut serde_json::Value,
    db_fields: &[shared::models::LabelField],
    images_dir: Option<&std::path::Path>,
) {
    use base64::Engine;

    let Some(images_dir) = images_dir else { return };

    for field in db_fields {
        // Only process static image fields (source_type=image with a hash in template)
        if field.source_type.as_deref() != Some("image") {
            continue;
        }
        let Some(hash) = field.template.as_deref().filter(|h| !h.is_empty()) else {
            continue;
        };

        // The data_key is what the printer will look for in the JSON
        let data_key = field.resolve_image_data_key();

        // Skip if already populated (e.g. store_logo)
        if data.get(&data_key).is_some() {
            continue;
        }

        // Load image file by hash from images directory
        let path = images_dir.join(hash);
        match std::fs::read(&path) {
            Ok(bytes) => {
                // Detect MIME by magic bytes (hash filenames may lack extension)
                let mime = if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
                    "image/jpeg"
                } else if bytes.len() > 12 && bytes.starts_with(b"RIFF") && &bytes[8..12] == b"WEBP"
                {
                    "image/webp"
                } else {
                    "image/png"
                };
                let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
                data[&data_key] =
                    serde_json::Value::String(format!("data:{};base64,{}", mime, b64));
                tracing::debug!(field_id = %field.field_id, data_key, "Injected static image");
            }
            Err(e) => {
                warn!(field_id = %field.field_id, hash, error = %e, "Failed to load static image");
            }
        }
    }
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
                template: f.resolve_text_template(),
            },
        )),
        LabelFieldType::Image | LabelFieldType::Barcode | LabelFieldType::Qrcode => Some(
            crab_printer::label::TemplateField::Image(crab_printer::label::ImageField {
                x: f.x,
                y: f.y,
                width: f.width,
                height: f.height,
                maintain_aspect_ratio: f.maintain_aspect_ratio.unwrap_or(true),
                data_key: f.resolve_image_data_key(),
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
            id: shared::util::snowflake_id(),
            order_id: 1001,
            receipt_number: "FAC202401220001".to_string(),
            table_name: Some("100桌".to_string()),
            zone_name: Some("大厅".to_string()),
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
                    price: 38.0,
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
