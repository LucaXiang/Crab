//! Kitchen/Label print service - handles print job generation and reprint

use super::storage::{PrintStorage, PrintStorageError};
use super::types::{KitchenOrder, KitchenOrderItem, LabelPrintRecord, PrintItemContext};
use crate::services::CatalogService;
use shared::order::{CartItemSnapshot, EventPayload, OrderEvent};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PrintServiceError {
    #[error("Storage error: {0}")]
    Storage(#[from] PrintStorageError),

    #[error("Kitchen order not found: {0}")]
    KitchenOrderNotFound(String),

    #[error("Label record not found: {0}")]
    LabelRecordNotFound(String),

    #[error("Printing disabled")]
    PrintingDisabled,
}

pub type PrintServiceResult<T> = Result<T, PrintServiceError>;

impl From<PrintServiceError> for shared::error::AppError {
    fn from(err: PrintServiceError) -> Self {
        use shared::error::{AppError, ErrorCode};
        match err {
            PrintServiceError::Storage(e) => AppError::database(e.to_string()),
            PrintServiceError::KitchenOrderNotFound(id) => {
                AppError::not_found(format!("Kitchen order {}", id))
            }
            PrintServiceError::LabelRecordNotFound(id) => {
                AppError::not_found(format!("Label record {}", id))
            }
            PrintServiceError::PrintingDisabled => AppError::with_message(
                ErrorCode::PrinterNotAvailable,
                "Printing disabled".to_string(),
            ),
        }
    }
}

/// Kitchen/Label print service
///
/// Responsibilities:
/// - Process ItemsAdded events to create KitchenOrder and LabelPrintRecord
/// - Provide reprint functionality
/// - Manage print job lifecycle
#[derive(Clone)]
pub struct KitchenPrintService {
    storage: PrintStorage,
}

impl KitchenPrintService {
    /// Create a new KitchenPrintService
    pub fn new(storage: PrintStorage) -> Self {
        Self { storage }
    }

    /// Process an ItemsAdded event
    ///
    /// Creates KitchenOrder and LabelPrintRecord entries if printing is enabled.
    /// Returns the created KitchenOrder ID if any items were processed.
    pub fn process_items_added(
        &self,
        event: &OrderEvent,
        table_name: Option<String>,
        catalog: &CatalogService,
    ) -> PrintServiceResult<Option<String>> {
        // Quick check: is any printing enabled?
        let kitchen_enabled = catalog.is_kitchen_print_enabled();
        let label_enabled = catalog.is_label_print_enabled();

        if !kitchen_enabled && !label_enabled {
            return Ok(None);
        }

        // Extract items from event
        let items = match &event.payload {
            EventPayload::ItemsAdded { items } => items,
            _ => return Ok(None),
        };

        if items.is_empty() {
            return Ok(None);
        }

        // Build print contexts for each item
        let mut kitchen_items = Vec::new();
        let mut label_records = Vec::new();

        for item in items {
            let context = self.build_print_context(item, catalog);

            // Check if this item should be printed to kitchen
            if kitchen_enabled && !context.kitchen_destinations.is_empty() {
                kitchen_items.push(KitchenOrderItem {
                    context: context.clone(),
                });
            }

            // Check if this item should have labels printed
            if label_enabled && !context.label_destinations.is_empty() {
                // Create one LabelPrintRecord per quantity unit
                for i in 1..=item.quantity {
                    let mut label_context = context.clone();
                    label_context.index = Some(format!("{}/{}", i, item.quantity));
                    label_context.quantity = 1;

                    label_records.push(LabelPrintRecord {
                        id: uuid::Uuid::new_v4().to_string(),
                        order_id: event.order_id.clone(),
                        kitchen_order_id: event.event_id.clone(),
                        table_name: table_name.clone(),
                        created_at: event.timestamp,
                        context: label_context,
                        print_count: 0,
                    });
                }
            }
        }

        if kitchen_items.is_empty() && label_records.is_empty() {
            return Ok(None);
        }

        // Create KitchenOrder
        let kitchen_order = KitchenOrder {
            id: event.event_id.clone(),
            order_id: event.order_id.clone(),
            table_name,
            created_at: event.timestamp,
            items: kitchen_items,
            print_count: 0,
        };

        // Store in database
        let txn = self.storage.begin_write()?;
        self.storage.store_kitchen_order(&txn, &kitchen_order)?;
        for record in &label_records {
            self.storage.store_label_record(&txn, record)?;
        }
        txn.commit().map_err(PrintStorageError::from)?;

        Ok(Some(kitchen_order.id))
    }

    /// Build a PrintItemContext from a CartItemSnapshot
    fn build_print_context(
        &self,
        item: &CartItemSnapshot,
        catalog: &CatalogService,
    ) -> PrintItemContext {
        // Get product from catalog
        let product = catalog.get_product(item.id);

        // Get category info
        let (category_id, category_name) = if let Some(ref p) = product {
            let cat_name = catalog
                .get_category(p.category_id)
                .map(|c| c.name.clone())
                .unwrap_or_default();
            (p.category_id, cat_name)
        } else {
            (0, String::new())
        };

        // Get print config from catalog (with fallback chain)
        let kitchen_config = catalog.get_kitchen_print_config(item.id);
        let label_config = catalog.get_label_print_config(item.id);

        let kitchen_destinations = kitchen_config
            .as_ref()
            .filter(|c| c.enabled)
            .map(|c| c.destinations.clone())
            .unwrap_or_default();

        let label_destinations = label_config
            .as_ref()
            .filter(|c| c.enabled)
            .map(|c| c.destinations.clone())
            .unwrap_or_default();

        let kitchen_name = kitchen_config
            .as_ref()
            .and_then(|c| c.kitchen_name.clone())
            .or_else(|| product.as_ref().map(|p| p.name.clone()))
            .unwrap_or_else(|| item.name.clone());

        // Get product external_id (now at product level)
        let external_id = product.as_ref().and_then(|p| p.external_id);

        // Build options list (with quantity if > 1)
        let options: Vec<String> = item
            .selected_options
            .as_ref()
            .map(|opts| {
                opts.iter()
                    .map(|opt| {
                        if opt.quantity > 1 {
                            format!("{}×{}", opt.option_name, opt.quantity)
                        } else {
                            opt.option_name.clone()
                        }
                    })
                    .collect()
            })
            .unwrap_or_default();

        let spec_name = item.selected_specification.as_ref().map(|s| s.name.clone());

        PrintItemContext {
            category_id,
            category_name,
            product_id: item.id,
            external_id,
            kitchen_name,
            product_name: item.name.clone(),
            spec_name,
            quantity: item.quantity,
            index: None,
            options,
            note: item.note.clone(),
            kitchen_destinations,
            label_destinations,
        }
    }

    /// Reprint a kitchen order
    pub fn reprint_kitchen_order(&self, id: &str) -> PrintServiceResult<KitchenOrder> {
        let order = self
            .storage
            .get_kitchen_order(id)?
            .ok_or_else(|| PrintServiceError::KitchenOrderNotFound(id.to_string()))?;

        let txn = self.storage.begin_write()?;
        self.storage.increment_kitchen_order_print_count(&txn, id)?;
        txn.commit().map_err(PrintStorageError::from)?;

        tracing::info!(kitchen_order_id = %id, print_count = order.print_count + 1, "Kitchen order reprinted");

        Ok(order)
    }

    /// Reprint a label record
    pub fn reprint_label_record(&self, id: &str) -> PrintServiceResult<LabelPrintRecord> {
        let record = self
            .storage
            .get_label_record(id)?
            .ok_or_else(|| PrintServiceError::LabelRecordNotFound(id.to_string()))?;

        let txn = self.storage.begin_write()?;
        self.storage.increment_label_record_print_count(&txn, id)?;
        txn.commit().map_err(PrintStorageError::from)?;

        tracing::info!(label_record_id = %id, print_count = record.print_count + 1, "Label record reprinted");

        Ok(record)
    }

    /// Get kitchen orders for an order
    pub fn get_kitchen_orders_for_order(
        &self,
        order_id: &str,
    ) -> PrintServiceResult<Vec<KitchenOrder>> {
        Ok(self.storage.get_kitchen_orders_for_order(order_id)?)
    }

    /// Get all kitchen orders (paginated)
    pub fn get_all_kitchen_orders(
        &self,
        offset: usize,
        limit: usize,
    ) -> PrintServiceResult<Vec<KitchenOrder>> {
        Ok(self.storage.get_all_kitchen_orders(offset, limit)?)
    }

    /// Get a kitchen order by ID
    pub fn get_kitchen_order(&self, id: &str) -> PrintServiceResult<Option<KitchenOrder>> {
        Ok(self.storage.get_kitchen_order(id)?)
    }

    /// Get label records for an order
    pub fn get_label_records_for_order(
        &self,
        order_id: &str,
    ) -> PrintServiceResult<Vec<LabelPrintRecord>> {
        Ok(self.storage.get_label_records_for_order(order_id)?)
    }

    /// Get a label record by ID
    pub fn get_label_record(&self, id: &str) -> PrintServiceResult<Option<LabelPrintRecord>> {
        Ok(self.storage.get_label_record(id)?)
    }

    /// Cleanup old records (older than max_age_secs)
    pub fn cleanup_old_records(&self, max_age_secs: i64) -> PrintServiceResult<usize> {
        Ok(self.storage.cleanup_old_records(max_age_secs)?)
    }
}

impl std::fmt::Debug for KitchenPrintService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KitchenPrintService")
            .field("storage", &"<PrintStorage>")
            .finish()
    }
}
