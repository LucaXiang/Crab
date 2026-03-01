//! redb-based storage for kitchen orders and label records

use super::types::{KitchenOrder, LabelPrintRecord};
use redb::{Database, ReadableDatabase, ReadableTable, TableDefinition, WriteTransaction};
use std::path::Path;
use std::sync::Arc;
use thiserror::Error;

/// Kitchen orders table: key = kitchen_order_id (i64 snowflake), value = JSON
const KITCHEN_ORDERS_TABLE: TableDefinition<i64, &[u8]> = TableDefinition::new("kitchen_orders");

/// Index: (order_id, kitchen_order_id) -> ()
const KITCHEN_ORDERS_BY_ORDER_TABLE: TableDefinition<(i64, i64), ()> =
    TableDefinition::new("kitchen_orders_by_order");

/// Label records table: key = label_record_id (i64 snowflake), value = JSON
const LABEL_RECORDS_TABLE: TableDefinition<i64, &[u8]> = TableDefinition::new("label_records");

/// Index: (order_id, label_record_id) -> ()
const LABEL_RECORDS_BY_ORDER_TABLE: TableDefinition<(i64, i64), ()> =
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
    KitchenOrderNotFound(i64),

    #[error("Label record not found: {0}")]
    LabelRecordNotFound(i64),
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
        let path = path.as_ref();
        let mut db = None;
        for attempt in 0..5 {
            match Database::create(path) {
                Ok(d) => {
                    db = Some(d);
                    break;
                }
                Err(redb::DatabaseError::DatabaseAlreadyOpen) if attempt < 4 => {
                    let wait = std::time::Duration::from_millis(200 * (attempt as u64 + 1));
                    tracing::warn!(
                        attempt = attempt + 1,
                        wait_ms = wait.as_millis() as u64,
                        "print.redb file locked, retrying..."
                    );
                    std::thread::sleep(wait);
                }
                Err(e) => return Err(e.into()),
            }
        }
        let db = db.expect("loop guarantees db is set on break");

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
        let db = Database::builder().create_with_backend(redb::backends::InMemoryBackend::new())?;

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
        table.insert(order.id, value.as_slice())?;

        // Update index
        let mut idx_table = txn.open_table(KITCHEN_ORDERS_BY_ORDER_TABLE)?;
        idx_table.insert((order.order_id, order.id), ())?;

        Ok(())
    }

    /// Get a kitchen order by ID
    pub fn get_kitchen_order(&self, id: i64) -> PrintStorageResult<Option<KitchenOrder>> {
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
        order_id: i64,
    ) -> PrintStorageResult<Vec<KitchenOrder>> {
        let read_txn = self.db.begin_read()?;
        let idx_table = read_txn.open_table(KITCHEN_ORDERS_BY_ORDER_TABLE)?;
        let data_table = read_txn.open_table(KITCHEN_ORDERS_TABLE)?;

        let mut orders = Vec::new();
        let range_start = (order_id, i64::MIN);
        let range_end = (order_id, i64::MAX);

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
        id: i64,
    ) -> PrintStorageResult<()> {
        let mut table = txn.open_table(KITCHEN_ORDERS_TABLE)?;

        // Read first
        let bytes = {
            let value = table
                .get(id)?
                .ok_or(PrintStorageError::KitchenOrderNotFound(id))?;
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
        order_id: i64,
    ) -> PrintStorageResult<()> {
        let mut idx_table = txn.open_table(KITCHEN_ORDERS_BY_ORDER_TABLE)?;
        let mut data_table = txn.open_table(KITCHEN_ORDERS_TABLE)?;

        // Collect IDs to delete
        let range_start = (order_id, i64::MIN);
        let range_end = (order_id, i64::MAX);
        let mut ids_to_delete = Vec::new();

        for result in idx_table.range(range_start..=range_end)? {
            let (key, _) = result?;
            let (_, id) = key.value();
            ids_to_delete.push(id);
        }

        // Delete from both tables
        for id in ids_to_delete {
            data_table.remove(id)?;
            idx_table.remove((order_id, id))?;
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
        table.insert(record.id, value.as_slice())?;

        let mut idx_table = txn.open_table(LABEL_RECORDS_BY_ORDER_TABLE)?;
        idx_table.insert((record.order_id, record.id), ())?;

        Ok(())
    }

    /// Get a label record by ID
    pub fn get_label_record(&self, id: i64) -> PrintStorageResult<Option<LabelPrintRecord>> {
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
        order_id: i64,
    ) -> PrintStorageResult<Vec<LabelPrintRecord>> {
        let read_txn = self.db.begin_read()?;
        let idx_table = read_txn.open_table(LABEL_RECORDS_BY_ORDER_TABLE)?;
        let data_table = read_txn.open_table(LABEL_RECORDS_TABLE)?;

        let mut records = Vec::new();
        let range_start = (order_id, i64::MIN);
        let range_end = (order_id, i64::MAX);

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
        id: i64,
    ) -> PrintStorageResult<()> {
        let mut table = txn.open_table(LABEL_RECORDS_TABLE)?;

        // Read first
        let bytes = {
            let value = table
                .get(id)?
                .ok_or(PrintStorageError::LabelRecordNotFound(id))?;
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
        order_id: i64,
    ) -> PrintStorageResult<()> {
        let mut idx_table = txn.open_table(LABEL_RECORDS_BY_ORDER_TABLE)?;
        let mut data_table = txn.open_table(LABEL_RECORDS_TABLE)?;

        let range_start = (order_id, i64::MIN);
        let range_end = (order_id, i64::MAX);
        let mut ids_to_delete = Vec::new();

        for result in idx_table.range(range_start..=range_end)? {
            let (key, _) = result?;
            let (_, id) = key.value();
            ids_to_delete.push(id);
        }

        for id in ids_to_delete {
            data_table.remove(id)?;
            idx_table.remove((order_id, id))?;
        }

        Ok(())
    }

    // ========== Cleanup ==========

    /// Clean up old records (older than max_age_secs)
    pub fn cleanup_old_records(&self, max_age_secs: i64) -> PrintStorageResult<usize> {
        let now = shared::util::now_millis();
        let cutoff = now - max_age_secs * 1000;

        let txn = self.db.begin_write()?;
        let mut deleted = 0;

        // Kitchen orders
        {
            let mut table = txn.open_table(KITCHEN_ORDERS_TABLE)?;
            let mut idx_table = txn.open_table(KITCHEN_ORDERS_BY_ORDER_TABLE)?;

            let mut to_delete: Vec<(i64, i64)> = Vec::new();
            for result in table.iter()? {
                let (key, guard) = result?;
                let order: KitchenOrder = serde_json::from_slice(guard.value())?;
                if order.created_at < cutoff {
                    to_delete.push((key.value(), order.order_id));
                }
            }

            for (id, order_id) in to_delete {
                table.remove(id)?;
                idx_table.remove((order_id, id))?;
                deleted += 1;
            }
        }

        // Label records
        {
            let mut table = txn.open_table(LABEL_RECORDS_TABLE)?;
            let mut idx_table = txn.open_table(LABEL_RECORDS_BY_ORDER_TABLE)?;

            let mut to_delete: Vec<(i64, i64)> = Vec::new();
            for result in table.iter()? {
                let (key, guard) = result?;
                let record: LabelPrintRecord = serde_json::from_slice(guard.value())?;
                if record.created_at < cutoff {
                    to_delete.push((key.value(), record.order_id));
                }
            }

            for (id, order_id) in to_delete {
                table.remove(id)?;
                idx_table.remove((order_id, id))?;
                deleted += 1;
            }
        }

        txn.commit()?;
        Ok(deleted)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kitchen_order_crud() {
        let storage = PrintStorage::open_in_memory().unwrap();

        let order = KitchenOrder {
            id: 100001,
            order_id: 200001,
            receipt_number: "FAC202401220001".to_string(),
            table_name: Some("Table 1".to_string()),
            zone_name: None,
            queue_number: None,
            is_retail: false,
            created_at: shared::util::now_millis(),
            items: vec![],
            print_count: 0,
        };

        let txn = storage.begin_write().unwrap();
        storage.store_kitchen_order(&txn, &order).unwrap();
        txn.commit().unwrap();

        let retrieved = storage.get_kitchen_order(100001).unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().order_id, 200001);
    }
}
