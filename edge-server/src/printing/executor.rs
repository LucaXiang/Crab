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
