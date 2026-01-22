//! redb-based storage for kitchen orders and label records

use super::types::{KitchenOrder, LabelPrintRecord};
use redb::{
    Database, ReadableDatabase, ReadableTable, ReadableTableMetadata, TableDefinition,
    WriteTransaction,
};
use std::path::Path;
use std::sync::Arc;
use thiserror::Error;

/// Kitchen orders table: key = kitchen_order_id, value = JSON
const KITCHEN_ORDERS_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("kitchen_orders");

/// Index: (order_id, kitchen_order_id) -> ()
const KITCHEN_ORDERS_BY_ORDER_TABLE: TableDefinition<(&str, &str), ()> =
    TableDefinition::new("kitchen_orders_by_order");

/// Label records table: key = label_record_id, value = JSON
const LABEL_RECORDS_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("label_records");

/// Index: (order_id, label_record_id) -> ()
const LABEL_RECORDS_BY_ORDER_TABLE: TableDefinition<(&str, &str), ()> =
    TableDefinition::new("label_records_by_order");

#[derive(Debug, Error)]
pub enum PrintStorageError {
    #[error("Database error: {0}")]
    Database(#[from] redb::DatabaseError),

    #[error("Transaction error: {0}")]
    Transaction(#[from] redb::TransactionError),

    #[error("Table error: {0}")]
    Table(#[from] redb::TableError),

    #[error("Storage error: {0}")]
    Storage(#[from] redb::StorageError),

    #[error("Commit error: {0}")]
    Commit(#[from] redb::CommitError),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Kitchen order not found: {0}")]
    KitchenOrderNotFound(String),

    #[error("Label record not found: {0}")]
    LabelRecordNotFound(String),
}

pub type PrintStorageResult<T> = Result<T, PrintStorageError>;

/// Kitchen/Label printing storage
#[derive(Clone)]
pub struct PrintStorage {
    db: Arc<Database>,
}

impl PrintStorage {
    /// Open or create database
    pub fn open(path: impl AsRef<Path>) -> PrintStorageResult<Self> {
        let db = Database::create(path)?;

        // Initialize tables
        let write_txn = db.begin_write()?;
        {
            let _ = write_txn.open_table(KITCHEN_ORDERS_TABLE)?;
            let _ = write_txn.open_table(KITCHEN_ORDERS_BY_ORDER_TABLE)?;
            let _ = write_txn.open_table(LABEL_RECORDS_TABLE)?;
            let _ = write_txn.open_table(LABEL_RECORDS_BY_ORDER_TABLE)?;
        }
        write_txn.commit()?;

        Ok(Self { db: Arc::new(db) })
    }

    /// Open in-memory database (for testing)
    #[cfg(test)]
    pub fn open_in_memory() -> PrintStorageResult<Self> {
        let db =
            Database::builder().create_with_backend(redb::backends::InMemoryBackend::new())?;

        let write_txn = db.begin_write()?;
        {
            let _ = write_txn.open_table(KITCHEN_ORDERS_TABLE)?;
            let _ = write_txn.open_table(KITCHEN_ORDERS_BY_ORDER_TABLE)?;
            let _ = write_txn.open_table(LABEL_RECORDS_TABLE)?;
            let _ = write_txn.open_table(LABEL_RECORDS_BY_ORDER_TABLE)?;
        }
        write_txn.commit()?;

        Ok(Self { db: Arc::new(db) })
    }

    pub fn begin_write(&self) -> PrintStorageResult<WriteTransaction> {
        Ok(self.db.begin_write()?)
    }

    // ========== Kitchen Orders ==========

    /// Store a kitchen order
    pub fn store_kitchen_order(
        &self,
        txn: &WriteTransaction,
        order: &KitchenOrder,
    ) -> PrintStorageResult<()> {
        let mut table = txn.open_table(KITCHEN_ORDERS_TABLE)?;
        let value = serde_json::to_vec(order)?;
        table.insert(order.id.as_str(), value.as_slice())?;

        // Update index
        let mut idx_table = txn.open_table(KITCHEN_ORDERS_BY_ORDER_TABLE)?;
        idx_table.insert((order.order_id.as_str(), order.id.as_str()), ())?;

        Ok(())
    }

    /// Get a kitchen order by ID
    pub fn get_kitchen_order(&self, id: &str) -> PrintStorageResult<Option<KitchenOrder>> {
        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_table(KITCHEN_ORDERS_TABLE)?;

        match table.get(id)? {
            Some(guard) => {
                let order: KitchenOrder = serde_json::from_slice(guard.value())?;
                Ok(Some(order))
            }
            None => Ok(None),
        }
    }

    /// Get kitchen orders for an order
    pub fn get_kitchen_orders_for_order(
        &self,
        order_id: &str,
    ) -> PrintStorageResult<Vec<KitchenOrder>> {
        let read_txn = self.db.begin_read()?;
        let idx_table = read_txn.open_table(KITCHEN_ORDERS_BY_ORDER_TABLE)?;
        let data_table = read_txn.open_table(KITCHEN_ORDERS_TABLE)?;

        let mut orders = Vec::new();
        let range_start: (&str, &str) = (order_id, "");
        let range_end: (&str, &str) = (order_id, "\u{ffff}");

        for result in idx_table.range(range_start..=range_end)? {
            let (key, _) = result?;
            let (_, kitchen_order_id) = key.value();
            if let Some(guard) = data_table.get(kitchen_order_id)? {
                let order: KitchenOrder = serde_json::from_slice(guard.value())?;
                orders.push(order);
            }
        }

        orders.sort_by_key(|o| o.created_at);
        Ok(orders)
    }

    /// Get all kitchen orders (paginated)
    pub fn get_all_kitchen_orders(
        &self,
        offset: usize,
        limit: usize,
    ) -> PrintStorageResult<Vec<KitchenOrder>> {
        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_table(KITCHEN_ORDERS_TABLE)?;

        let mut orders: Vec<KitchenOrder> = Vec::new();
        for result in table.iter()? {
            let (_, guard) = result?;
            let order: KitchenOrder = serde_json::from_slice(guard.value())?;
            orders.push(order);
        }

        // Sort by created_at descending
        orders.sort_by_key(|o| std::cmp::Reverse(o.created_at));

        Ok(orders.into_iter().skip(offset).take(limit).collect())
    }

    /// Update kitchen order print count
    pub fn increment_kitchen_order_print_count(
        &self,
        txn: &WriteTransaction,
        id: &str,
    ) -> PrintStorageResult<()> {
        let mut table = txn.open_table(KITCHEN_ORDERS_TABLE)?;

        // Read first
        let bytes = {
            let value = table
                .get(id)?
                .ok_or_else(|| PrintStorageError::KitchenOrderNotFound(id.to_string()))?;
            value.value().to_vec()
        };

        let mut order: KitchenOrder = serde_json::from_slice(&bytes)?;
        order.print_count += 1;

        let new_value = serde_json::to_vec(&order)?;
        table.insert(id, new_value.as_slice())?;

        Ok(())
    }

    /// Delete kitchen orders for an order
    pub fn delete_kitchen_orders_for_order(
        &self,
        txn: &WriteTransaction,
        order_id: &str,
    ) -> PrintStorageResult<()> {
        let mut idx_table = txn.open_table(KITCHEN_ORDERS_BY_ORDER_TABLE)?;
        let mut data_table = txn.open_table(KITCHEN_ORDERS_TABLE)?;

        // Collect IDs to delete
        let range_start: (&str, &str) = (order_id, "");
        let range_end: (&str, &str) = (order_id, "\u{ffff}");
        let mut ids_to_delete = Vec::new();

        for result in idx_table.range(range_start..=range_end)? {
            let (key, _) = result?;
            let (_, id) = key.value();
            ids_to_delete.push(id.to_string());
        }

        // Delete from both tables
        for id in &ids_to_delete {
            data_table.remove(id.as_str())?;
            idx_table.remove((order_id, id.as_str()))?;
        }

        Ok(())
    }

    // ========== Label Records ==========

    /// Store a label record
    pub fn store_label_record(
        &self,
        txn: &WriteTransaction,
        record: &LabelPrintRecord,
    ) -> PrintStorageResult<()> {
        let mut table = txn.open_table(LABEL_RECORDS_TABLE)?;
        let value = serde_json::to_vec(record)?;
        table.insert(record.id.as_str(), value.as_slice())?;

        let mut idx_table = txn.open_table(LABEL_RECORDS_BY_ORDER_TABLE)?;
        idx_table.insert((record.order_id.as_str(), record.id.as_str()), ())?;

        Ok(())
    }

    /// Get a label record by ID
    pub fn get_label_record(&self, id: &str) -> PrintStorageResult<Option<LabelPrintRecord>> {
        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_table(LABEL_RECORDS_TABLE)?;

        match table.get(id)? {
            Some(guard) => {
                let record: LabelPrintRecord = serde_json::from_slice(guard.value())?;
                Ok(Some(record))
            }
            None => Ok(None),
        }
    }

    /// Get label records for an order
    pub fn get_label_records_for_order(
        &self,
        order_id: &str,
    ) -> PrintStorageResult<Vec<LabelPrintRecord>> {
        let read_txn = self.db.begin_read()?;
        let idx_table = read_txn.open_table(LABEL_RECORDS_BY_ORDER_TABLE)?;
        let data_table = read_txn.open_table(LABEL_RECORDS_TABLE)?;

        let mut records = Vec::new();
        let range_start: (&str, &str) = (order_id, "");
        let range_end: (&str, &str) = (order_id, "\u{ffff}");

        for result in idx_table.range(range_start..=range_end)? {
            let (key, _) = result?;
            let (_, record_id) = key.value();
            if let Some(guard) = data_table.get(record_id)? {
                let record: LabelPrintRecord = serde_json::from_slice(guard.value())?;
                records.push(record);
            }
        }

        records.sort_by_key(|r| r.created_at);
        Ok(records)
    }

    /// Increment label record print count
    pub fn increment_label_record_print_count(
        &self,
        txn: &WriteTransaction,
        id: &str,
    ) -> PrintStorageResult<()> {
        let mut table = txn.open_table(LABEL_RECORDS_TABLE)?;

        // Read first
        let bytes = {
            let value = table
                .get(id)?
                .ok_or_else(|| PrintStorageError::LabelRecordNotFound(id.to_string()))?;
            value.value().to_vec()
        };

        let mut record: LabelPrintRecord = serde_json::from_slice(&bytes)?;
        record.print_count += 1;

        let new_value = serde_json::to_vec(&record)?;
        table.insert(id, new_value.as_slice())?;

        Ok(())
    }

    /// Delete label records for an order
    pub fn delete_label_records_for_order(
        &self,
        txn: &WriteTransaction,
        order_id: &str,
    ) -> PrintStorageResult<()> {
        let mut idx_table = txn.open_table(LABEL_RECORDS_BY_ORDER_TABLE)?;
        let mut data_table = txn.open_table(LABEL_RECORDS_TABLE)?;

        let range_start: (&str, &str) = (order_id, "");
        let range_end: (&str, &str) = (order_id, "\u{ffff}");
        let mut ids_to_delete = Vec::new();

        for result in idx_table.range(range_start..=range_end)? {
            let (key, _) = result?;
            let (_, id) = key.value();
            ids_to_delete.push(id.to_string());
        }

        for id in &ids_to_delete {
            data_table.remove(id.as_str())?;
            idx_table.remove((order_id, id.as_str()))?;
        }

        Ok(())
    }

    // ========== Cleanup ==========

    /// Clean up old records (older than max_age_secs)
    pub fn cleanup_old_records(&self, max_age_secs: i64) -> PrintStorageResult<usize> {
        let now = chrono::Utc::now().timestamp();
        let cutoff = now - max_age_secs;

        let txn = self.db.begin_write()?;
        let mut deleted = 0;

        // Kitchen orders
        {
            let mut table = txn.open_table(KITCHEN_ORDERS_TABLE)?;
            let mut idx_table = txn.open_table(KITCHEN_ORDERS_BY_ORDER_TABLE)?;

            let mut to_delete = Vec::new();
            for result in table.iter()? {
                let (key, guard) = result?;
                let order: KitchenOrder = serde_json::from_slice(guard.value())?;
                if order.created_at < cutoff {
                    to_delete.push((key.value().to_string(), order.order_id.clone()));
                }
            }

            for (id, order_id) in &to_delete {
                table.remove(id.as_str())?;
                idx_table.remove((order_id.as_str(), id.as_str()))?;
                deleted += 1;
            }
        }

        // Label records
        {
            let mut table = txn.open_table(LABEL_RECORDS_TABLE)?;
            let mut idx_table = txn.open_table(LABEL_RECORDS_BY_ORDER_TABLE)?;

            let mut to_delete = Vec::new();
            for result in table.iter()? {
                let (key, guard) = result?;
                let record: LabelPrintRecord = serde_json::from_slice(guard.value())?;
                if record.created_at < cutoff {
                    to_delete.push((key.value().to_string(), record.order_id.clone()));
                }
            }

            for (id, order_id) in &to_delete {
                table.remove(id.as_str())?;
                idx_table.remove((order_id.as_str(), id.as_str()))?;
                deleted += 1;
            }
        }

        txn.commit()?;
        Ok(deleted)
    }

    /// Get storage statistics
    pub fn get_stats(&self) -> PrintStorageResult<PrintStorageStats> {
        let read_txn = self.db.begin_read()?;
        let ko_table = read_txn.open_table(KITCHEN_ORDERS_TABLE)?;
        let lr_table = read_txn.open_table(LABEL_RECORDS_TABLE)?;

        Ok(PrintStorageStats {
            kitchen_order_count: ko_table.len()?,
            label_record_count: lr_table.len()?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct PrintStorageStats {
    pub kitchen_order_count: u64,
    pub label_record_count: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kitchen_order_crud() {
        let storage = PrintStorage::open_in_memory().unwrap();

        let order = KitchenOrder {
            id: "ko-1".to_string(),
            order_id: "order-1".to_string(),
            table_name: Some("Table 1".to_string()),
            created_at: chrono::Utc::now().timestamp(),
            items: vec![],
            print_count: 0,
        };

        let txn = storage.begin_write().unwrap();
        storage.store_kitchen_order(&txn, &order).unwrap();
        txn.commit().unwrap();

        let retrieved = storage.get_kitchen_order("ko-1").unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().order_id, "order-1");
    }
}
