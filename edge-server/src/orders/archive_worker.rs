//! Archive Worker - Processes pending archive queue
//!
//! Listens for terminal events and processes archive queue with retry logic.
//! Decoupled from OrderManager for better separation of concerns.

use super::archive::OrderArchiveService;
use super::money::{to_decimal, to_f64};
use super::storage::{OrderStorage, PendingArchive};
use crate::db::repository::ShiftRepository;
use rust_decimal::prelude::*;
use shared::order::{OrderEvent, OrderEventType, OrderSnapshot};
use std::sync::Arc;
use std::time::Duration;
use surrealdb::engine::local::Db;
use surrealdb::Surreal;
use tokio::sync::broadcast;

/// Terminal event types that trigger archiving
const TERMINAL_EVENT_TYPES: &[OrderEventType] = &[
    OrderEventType::OrderCompleted,
    OrderEventType::OrderVoided,
    OrderEventType::OrderMoved,
    OrderEventType::OrderMerged,
];

/// Archive worker configuration
const MAX_RETRY_COUNT: u32 = 10;
const RETRY_BASE_DELAY_SECS: u64 = 5;
const RETRY_MAX_DELAY_SECS: u64 = 3600; // 1 hour
const QUEUE_SCAN_INTERVAL_SECS: u64 = 60;
/// 并发归档数量
const ARCHIVE_CONCURRENCY: usize = 50;

/// Worker for processing archive queue (支持并发归档)
pub struct ArchiveWorker {
    storage: OrderStorage,
    archive_service: OrderArchiveService,
    db: Surreal<Db>,
    semaphore: Arc<tokio::sync::Semaphore>,
}

impl ArchiveWorker {
    pub fn new(storage: OrderStorage, archive_service: OrderArchiveService, db: Surreal<Db>) -> Self {
        Self {
            storage,
            archive_service,
            db,
            semaphore: Arc::new(tokio::sync::Semaphore::new(ARCHIVE_CONCURRENCY)),
        }
    }

    /// Run the archive worker (并发处理归档)
    pub async fn run(self, mut event_rx: broadcast::Receiver<OrderEvent>) {
        tracing::info!("ArchiveWorker started with concurrency={}", ARCHIVE_CONCURRENCY);

        let worker = Arc::new(self);

        // Process any pending archives from previous run
        worker.process_pending_queue().await;

        let mut scan_interval =
            tokio::time::interval(Duration::from_secs(QUEUE_SCAN_INTERVAL_SECS));

        loop {
            tokio::select! {
                // Handle new terminal events
                result = event_rx.recv() => {
                    match result {
                        Ok(event) if TERMINAL_EVENT_TYPES.contains(&event.event_type) => {
                            tracing::debug!(order_id = %event.order_id, event_type = ?event.event_type, "Received terminal event");
                            // 并发处理归档
                            let w = worker.clone();
                            let order_id = event.order_id.clone();
                            tokio::spawn(async move {
                                w.process_order_concurrent(&order_id).await;
                            });
                        }
                        Ok(_) => {} // Ignore non-terminal events
                        Err(broadcast::error::RecvError::Lagged(n)) => {
                            tracing::warn!(skipped = n, "Event receiver lagged, processing queue");
                            worker.process_pending_queue().await;
                        }
                        Err(broadcast::error::RecvError::Closed) => {
                            tracing::info!("Event channel closed, shutting down ArchiveWorker");
                            break;
                        }
                    }
                }
                // Periodic queue scan for retries
                _ = scan_interval.tick() => {
                    worker.process_pending_queue().await;
                }
            }
        }
    }

    /// 带并发限制的订单处理
    async fn process_order_concurrent(&self, order_id: &str) {
        let _permit = self.semaphore.acquire().await.unwrap();
        self.process_order(order_id).await;
    }

    /// Process all pending archives
    async fn process_pending_queue(&self) {
        // Get pending archives (blocking I/O -> spawn_blocking)
        let storage = self.storage.clone();
        let pending = match tokio::task::spawn_blocking(move || storage.get_pending_archives()).await {
            Ok(Ok(p)) => p,
            Ok(Err(e)) => {
                tracing::error!(error = %e, "Failed to get pending archives");
                return;
            }
            Err(e) => {
                tracing::error!(error = %e, "spawn_blocking panicked");
                return;
            }
        };

        if pending.is_empty() {
            return;
        }

        tracing::info!(count = pending.len(), "Processing pending archive queue");

        for entry in pending {
            if self.should_retry(&entry).await {
                self.process_order(&entry.order_id).await;
            }
        }
    }

    /// Check if entry should be retried based on backoff
    async fn should_retry(&self, entry: &PendingArchive) -> bool {
        if entry.retry_count >= MAX_RETRY_COUNT {
            tracing::error!(
                order_id = %entry.order_id,
                retry_count = entry.retry_count,
                last_error = ?entry.last_error,
                "Max retry count exceeded, removing from queue"
            );
            // Remove from queue - order data remains in redb for manual recovery
            let storage = self.storage.clone();
            let order_id = entry.order_id.clone();
            let _ = tokio::task::spawn_blocking(move || storage.remove_from_pending(&order_id)).await;
            return false;
        }

        // Exponential backoff: delay = base * 2^retry_count, capped at max
        let delay_secs =
            (RETRY_BASE_DELAY_SECS * 2u64.pow(entry.retry_count)).min(RETRY_MAX_DELAY_SECS);
        let retry_after_ms = entry.created_at + (delay_secs as i64 * 1000);
        let now = chrono::Utc::now().timestamp_millis();

        now >= retry_after_ms
    }

    /// Process a single order archive
    ///
    /// Uses spawn_blocking for redb operations to avoid blocking tokio runtime.
    async fn process_order(&self, order_id: &str) {
        let order_id_owned = order_id.to_string();
        let storage = self.storage.clone();

        // 1. Load snapshot and events from redb (blocking I/O -> spawn_blocking)
        let load_result = tokio::task::spawn_blocking(move || {
            let snapshot = storage.get_snapshot(&order_id_owned)?;
            let events = storage.get_events_for_order(&order_id_owned)?;
            Ok::<_, super::storage::StorageError>((snapshot, events, order_id_owned))
        })
        .await;

        let (snapshot, events, order_id_owned) = match load_result {
            Ok(Ok((Some(s), e, oid))) => (s, e, oid),
            Ok(Ok((None, _, oid))) => {
                tracing::warn!(order_id = %oid, "Snapshot not found, removing from queue");
                let storage = self.storage.clone();
                let _ = tokio::task::spawn_blocking(move || storage.remove_from_pending(&oid)).await;
                return;
            }
            Ok(Err(e)) => {
                tracing::error!(order_id = %order_id, error = %e, "Failed to load data from redb");
                return;
            }
            Err(e) => {
                tracing::error!(order_id = %order_id, error = %e, "spawn_blocking panicked");
                return;
            }
        };

        // 2. Archive to SurrealDB (async)
        match self.archive_service.archive_order(&snapshot, events.clone()).await {
            Ok(()) => {
                tracing::info!(order_id = %order_id_owned, "Order archived successfully");

                // 3. Update shift expected_cash for cash payments
                self.update_shift_cash(&snapshot, &events).await;

                // 4. Cleanup redb (blocking I/O -> spawn_blocking)
                let storage = self.storage.clone();
                let oid = order_id_owned.clone();
                if let Err(e) = tokio::task::spawn_blocking(move || storage.complete_archive(&oid)).await {
                    tracing::error!(order_id = %order_id_owned, error = %e, "Failed to complete archive cleanup");
                }
            }
            Err(e) => {
                tracing::error!(order_id = %order_id_owned, error = %e, "Archive failed");
                let storage = self.storage.clone();
                let oid = order_id_owned.clone();
                let err_msg = e.to_string();
                if let Err(e2) = tokio::task::spawn_blocking(move || storage.mark_archive_failed(&oid, &err_msg)).await {
                    tracing::error!(order_id = %order_id_owned, error = %e2, "Failed to mark archive failed");
                }
            }
        }
    }

    /// Update shift expected_cash for cash payments in the order
    async fn update_shift_cash(&self, snapshot: &OrderSnapshot, events: &[OrderEvent]) {
        use shared::order::{OrderStatus, VoidType};

        // Skip cash tracking for CANCELLED void orders (no money changed hands)
        // LOSS_SETTLED void orders should still count cash (it was actually received)
        if snapshot.status == OrderStatus::Void {
            if let Some(ref void_type) = snapshot.void_type {
                if *void_type == VoidType::Cancelled {
                    tracing::info!(
                        order_id = %snapshot.order_id,
                        void_type = ?void_type,
                        "Skipping cash tracking for CANCELLED void order"
                    );
                    return;
                }
            }
        }

        // Debug: log all payment methods for troubleshooting
        tracing::info!(
            order_id = %snapshot.order_id,
            payments = ?snapshot.payments.iter().map(|p| (&p.method, p.amount, p.cancelled)).collect::<Vec<_>>(),
            "Processing shift cash update"
        );

        // Calculate total cash payments (non-cancelled) using Decimal for precision
        let cash_total: Decimal = snapshot
            .payments
            .iter()
            .filter(|p| !p.cancelled && p.method == "CASH")
            .map(|p| to_decimal(p.amount))
            .sum();

        if cash_total <= Decimal::ZERO {
            tracing::info!(order_id = %snapshot.order_id, "No cash payments to track");
            return;
        }

        // Get operator_id from terminal event (any terminal event type)
        let operator_id = events
            .iter()
            .rev()
            .find(|e| TERMINAL_EVENT_TYPES.contains(&e.event_type))
            .map(|e| e.operator_id.clone());

        let Some(operator_id) = operator_id else {
            tracing::warn!(
                order_id = %snapshot.order_id,
                event_types = ?events.iter().map(|e| &e.event_type).collect::<Vec<_>>(),
                "No terminal event found for cash tracking"
            );
            return;
        };

        let shift_repo = ShiftRepository::new(self.db.clone());
        let cash_amount = to_f64(cash_total);
        if let Err(e) = shift_repo.add_cash_payment(&operator_id, cash_amount).await {
            // Log but don't fail the archive - shift tracking is secondary
            tracing::warn!(
                order_id = %snapshot.order_id,
                operator_id = %operator_id,
                cash_total = cash_amount,
                error = %e,
                "Failed to update shift expected_cash"
            );
        } else {
            tracing::debug!(
                order_id = %snapshot.order_id,
                operator_id = %operator_id,
                cash_total = cash_amount,
                "Updated shift expected_cash"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_terminal_event_types() {
        assert!(TERMINAL_EVENT_TYPES.contains(&OrderEventType::OrderCompleted));
        assert!(TERMINAL_EVENT_TYPES.contains(&OrderEventType::OrderVoided));
        assert!(TERMINAL_EVENT_TYPES.contains(&OrderEventType::OrderMoved));
        assert!(TERMINAL_EVENT_TYPES.contains(&OrderEventType::OrderMerged));
        assert!(!TERMINAL_EVENT_TYPES.contains(&OrderEventType::ItemsAdded));
    }

    #[test]
    fn test_backoff_calculation() {
        // Test exponential backoff formula
        let base = RETRY_BASE_DELAY_SECS;
        let max = RETRY_MAX_DELAY_SECS;

        assert_eq!((base * 2u64.pow(0)).min(max), 5); // retry 0: 5s
        assert_eq!((base * 2u64.pow(1)).min(max), 10); // retry 1: 10s
        assert_eq!((base * 2u64.pow(2)).min(max), 20); // retry 2: 20s
        assert_eq!((base * 2u64.pow(3)).min(max), 40); // retry 3: 40s
        assert_eq!((base * 2u64.pow(10)).min(max), 3600); // retry 10: capped at 1h
    }
}
