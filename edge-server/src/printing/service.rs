//! Kitchen/Label print service - handles print job generation and reprint

use super::cache::PrintConfigCache;
use super::storage::{PrintStorage, PrintStorageError};
use super::types::{KitchenOrder, KitchenOrderItem, LabelPrintRecord, PrintItemContext};
use shared::order::{CartItemSnapshot, EventPayload, OrderEvent};
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;

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

/// Kitchen/Label print service
///
/// Responsibilities:
/// - Process ItemsAdded events to create KitchenOrder and LabelPrintRecord
/// - Provide reprint functionality
/// - Manage print job lifecycle
#[derive(Clone)]
pub struct KitchenPrintService {
    storage: PrintStorage,
    config_cache: PrintConfigCache,
    /// Pending print jobs (not yet sent to printer)
    /// In a real implementation, this would be a queue to a print spooler
    #[allow(dead_code)]
    inner: Arc<RwLock<KitchenPrintServiceInner>>,
}

#[derive(Debug, Default)]
struct KitchenPrintServiceInner {
    // Future: pending print queue, printer connections, etc.
}

impl KitchenPrintService {
    /// Create a new KitchenPrintService
    pub fn new(storage: PrintStorage, config_cache: PrintConfigCache) -> Self {
        Self {
            storage,
            config_cache,
            inner: Arc::new(RwLock::new(KitchenPrintServiceInner::default())),
        }
    }

    /// Get the config cache (for external updates)
    pub fn config_cache(&self) -> &PrintConfigCache {
        &self.config_cache
    }

    /// Get the storage (for external queries)
    pub fn storage(&self) -> &PrintStorage {
        &self.storage
    }

    /// Process an ItemsAdded event
    ///
    /// Creates KitchenOrder and LabelPrintRecord entries if printing is enabled.
    /// Returns the created KitchenOrder ID if any items were processed.
    pub async fn process_items_added(
        &self,
        event: &OrderEvent,
        table_name: Option<String>,
    ) -> PrintServiceResult<Option<String>> {
        // Quick check: is any printing enabled?
        let kitchen_enabled = self.config_cache.is_kitchen_print_enabled().await;
        let label_enabled = self.config_cache.is_label_print_enabled().await;

        if !kitchen_enabled && !label_enabled {
            // Printing not configured, skip entirely (zero overhead)
            return Ok(None);
        }

        // Extract items from event
        let items = match &event.payload {
            EventPayload::ItemsAdded { items } => items,
            _ => return Ok(None), // Not an ItemsAdded event
        };

        if items.is_empty() {
            return Ok(None);
        }

        // Build print contexts for each item
        let mut kitchen_items = Vec::new();
        let mut label_records = Vec::new();

        for item in items {
            let context = self.build_print_context(item).await;

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
                    label_context.quantity = 1; // Each label is for one item

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

        // If no items to print, return early
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

        // Store kitchen order (even if empty items, for tracking)
        self.storage.store_kitchen_order(&txn, &kitchen_order)?;

        // Store label records
        for record in &label_records {
            self.storage.store_label_record(&txn, record)?;
        }

        txn.commit().map_err(PrintStorageError::from)?;

        // TODO: Actually send to printers here
        // For now, we just store the records. Printing will be handled separately.

        Ok(Some(kitchen_order.id))
    }

    /// Build a PrintItemContext from a CartItemSnapshot
    async fn build_print_context(&self, item: &CartItemSnapshot) -> PrintItemContext {
        // Get product config from cache
        let product_config = self.config_cache.get_product(&item.id).await;

        // Get category info from cache
        let (category_id, category_name) = if let Some(ref pc) = product_config {
            let cat_config = self.config_cache.get_category(&pc.category_id).await;
            (
                pc.category_id.clone(),
                cat_config.map(|c| c.category_name).unwrap_or_default(),
            )
        } else {
            (String::new(), String::new())
        };

        // Get destinations using the fallback chain
        let kitchen_destinations = self.config_cache.get_kitchen_destinations(&item.id).await;
        let label_destinations = self.config_cache.get_label_destinations(&item.id).await;

        // Build options list from selected_options
        let options: Vec<String> = item
            .selected_options
            .as_ref()
            .map(|opts| opts.iter().map(|opt| opt.option_name.clone()).collect())
            .unwrap_or_default();

        // Get spec name if present
        let spec_name = item
            .selected_specification
            .as_ref()
            .map(|s| s.name.clone());

        PrintItemContext {
            category_id,
            category_name,
            product_id: item.id.clone(),
            external_id: product_config
                .as_ref()
                .and_then(|p| p.root_spec_external_id),
            kitchen_name: product_config
                .as_ref()
                .map(|p| p.kitchen_name.clone())
                .unwrap_or_else(|| item.name.clone()),
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
    pub async fn reprint_kitchen_order(&self, id: &str) -> PrintServiceResult<()> {
        // Load kitchen order
        let order = self
            .storage
            .get_kitchen_order(id)?
            .ok_or_else(|| PrintServiceError::KitchenOrderNotFound(id.to_string()))?;

        // TODO: Actually send to printers

        // Increment print count
        let txn = self.storage.begin_write()?;
        self.storage.increment_kitchen_order_print_count(&txn, id)?;
        txn.commit().map_err(PrintStorageError::from)?;

        tracing::info!(kitchen_order_id = %id, print_count = order.print_count + 1, "Kitchen order reprinted");

        Ok(())
    }

    /// Reprint a label record
    pub async fn reprint_label_record(&self, id: &str) -> PrintServiceResult<()> {
        // Load label record
        let record = self
            .storage
            .get_label_record(id)?
            .ok_or_else(|| PrintServiceError::LabelRecordNotFound(id.to_string()))?;

        // TODO: Actually send to printer

        // Increment print count
        let txn = self.storage.begin_write()?;
        self.storage.increment_label_record_print_count(&txn, id)?;
        txn.commit().map_err(PrintStorageError::from)?;

        tracing::info!(label_record_id = %id, print_count = record.print_count + 1, "Label record reprinted");

        Ok(())
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

    /// Delete kitchen orders and label records for an order
    /// Called when an order is voided or for manual cleanup
    pub fn delete_records_for_order(&self, order_id: &str) -> PrintServiceResult<()> {
        let txn = self.storage.begin_write()?;
        self.storage
            .delete_kitchen_orders_for_order(&txn, order_id)?;
        self.storage
            .delete_label_records_for_order(&txn, order_id)?;
        txn.commit().map_err(PrintStorageError::from)?;
        Ok(())
    }
}

impl std::fmt::Debug for KitchenPrintService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KitchenPrintService")
            .field("storage", &"<PrintStorage>")
            .field("config_cache", &"<PrintConfigCache>")
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::order::{EventPayload, OrderEvent, OrderEventType};

    fn create_test_service() -> KitchenPrintService {
        let storage = PrintStorage::open_in_memory().unwrap();
        let cache = PrintConfigCache::new();
        KitchenPrintService::new(storage, cache)
    }

    fn create_test_item() -> CartItemSnapshot {
        CartItemSnapshot {
            id: "prod-1".to_string(),
            instance_id: "inst-1".to_string(),
            name: "Test Product".to_string(),
            price: 10.0,
            original_price: None,
            quantity: 2,
            unpaid_quantity: 2,
            selected_options: None,
            selected_specification: None,
            manual_discount_percent: None,
            surcharge: None,
            rule_discount_amount: None,
            rule_surcharge_amount: None,
            applied_rules: None,
            line_total: None,
            note: None,
            authorizer_id: None,
            authorizer_name: None,
        }
    }

    fn create_test_event(event_id: &str, order_id: &str, items: Vec<CartItemSnapshot>) -> OrderEvent {
        OrderEvent {
            event_id: event_id.to_string(),
            order_id: order_id.to_string(),
            event_type: OrderEventType::ItemsAdded,
            sequence: 1,
            timestamp: chrono::Utc::now().timestamp(),
            client_timestamp: None,
            operator_id: "op-1".to_string(),
            operator_name: "Test".to_string(),
            command_id: "cmd-1".to_string(),
            payload: EventPayload::ItemsAdded { items },
        }
    }

    #[tokio::test]
    async fn test_process_items_added_no_printing_configured() {
        let service = create_test_service();
        let event = create_test_event("evt-1", "order-1", vec![create_test_item()]);

        // Without any printing configured, should return None
        let result = service.process_items_added(&event, None).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_process_items_added_with_kitchen_enabled() {
        let service = create_test_service();

        // Configure kitchen printing
        service
            .config_cache
            .set_defaults(Some("default-kitchen".to_string()), None)
            .await;

        let event = create_test_event("evt-1", "order-1", vec![create_test_item()]);

        let result = service
            .process_items_added(&event, Some("Table 1".to_string()))
            .await;
        assert!(result.is_ok());
        let kitchen_order_id = result.unwrap();
        assert!(kitchen_order_id.is_some());

        // Verify kitchen order was stored
        let ko = service.get_kitchen_order("evt-1").unwrap();
        assert!(ko.is_some());
        let ko = ko.unwrap();
        assert_eq!(ko.order_id, "order-1");
        assert_eq!(ko.table_name, Some("Table 1".to_string()));
        assert_eq!(ko.items.len(), 1);
    }
}
