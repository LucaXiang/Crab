//! Archive Worker - Processes pending archive queue
//!
//! Listens for terminal events and processes archive queue with retry logic.
//! Decoupled from OrderManager for better separation of concerns.

use super::archive::OrderArchiveService;
use super::storage::{OrderStorage, PendingArchive};
use shared::order::{OrderEvent, OrderEventType};
use std::time::Duration;
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

/// Worker for processing archive queue
pub struct ArchiveWorker {
    storage: OrderStorage,
    archive_service: OrderArchiveService,
}

impl ArchiveWorker {
    pub fn new(storage: OrderStorage, archive_service: OrderArchiveService) -> Self {
        Self {
            storage,
            archive_service,
        }
    }

    /// Run the archive worker
    pub async fn run(self, mut event_rx: broadcast::Receiver<OrderEvent>) {
        tracing::info!("ArchiveWorker started");

        // Process any pending archives from previous run
        self.process_pending_queue().await;

        let mut scan_interval =
            tokio::time::interval(Duration::from_secs(QUEUE_SCAN_INTERVAL_SECS));

        loop {
            tokio::select! {
                // Handle new terminal events
                result = event_rx.recv() => {
                    match result {
                        Ok(event) if TERMINAL_EVENT_TYPES.contains(&event.event_type) => {
                            tracing::debug!(order_id = %event.order_id, event_type = ?event.event_type, "Received terminal event");
                            self.process_order(&event.order_id).await;
                        }
                        Ok(_) => {} // Ignore non-terminal events
                        Err(broadcast::error::RecvError::Lagged(n)) => {
                            tracing::warn!(skipped = n, "Event receiver lagged, processing queue");
                            self.process_pending_queue().await;
                        }
                        Err(broadcast::error::RecvError::Closed) => {
                            tracing::info!("Event channel closed, shutting down ArchiveWorker");
                            break;
                        }
                    }
                }
                // Periodic queue scan for retries
                _ = scan_interval.tick() => {
                    self.process_pending_queue().await;
                }
            }
        }
    }

    /// Process all pending archives
    async fn process_pending_queue(&self) {
        let pending = match self.storage.get_pending_archives() {
            Ok(p) => p,
            Err(e) => {
                tracing::error!(error = %e, "Failed to get pending archives");
                return;
            }
        };

        if pending.is_empty() {
            return;
        }

        tracing::info!(count = pending.len(), "Processing pending archive queue");

        for entry in pending {
            if self.should_retry(&entry) {
                self.process_order(&entry.order_id).await;
            }
        }
    }

    /// Check if entry should be retried based on backoff
    fn should_retry(&self, entry: &PendingArchive) -> bool {
        if entry.retry_count >= MAX_RETRY_COUNT {
            tracing::error!(
                order_id = %entry.order_id,
                retry_count = entry.retry_count,
                last_error = ?entry.last_error,
                "Max retry count exceeded, removing from queue"
            );
            // Remove from queue - order data remains in redb for manual recovery
            let _ = self.storage.remove_from_pending(&entry.order_id);
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
    async fn process_order(&self, order_id: &str) {
        // 1. Load snapshot from redb
        let snapshot = match self.storage.get_snapshot(order_id) {
            Ok(Some(s)) => s,
            Ok(None) => {
                tracing::warn!(order_id = %order_id, "Snapshot not found, removing from queue");
                let _ = self.storage.remove_from_pending(order_id);
                return;
            }
            Err(e) => {
                tracing::error!(order_id = %order_id, error = %e, "Failed to load snapshot");
                return;
            }
        };

        // 2. Load events from redb
        let events = match self.storage.get_events_for_order(order_id) {
            Ok(e) => e,
            Err(e) => {
                tracing::error!(order_id = %order_id, error = %e, "Failed to load events");
                return;
            }
        };

        // 3. Archive to SurrealDB
        match self.archive_service.archive_order(&snapshot, events).await {
            Ok(()) => {
                tracing::info!(order_id = %order_id, "Order archived successfully");
                // 4. Cleanup redb (removes pending, snapshot, events atomically)
                if let Err(e) = self.storage.complete_archive(order_id) {
                    tracing::error!(order_id = %order_id, error = %e, "Failed to complete archive cleanup");
                }
            }
            Err(e) => {
                tracing::error!(order_id = %order_id, error = %e, "Archive failed");
                if let Err(e2) = self.storage.mark_archive_failed(order_id, &e.to_string()) {
                    tracing::error!(order_id = %order_id, error = %e2, "Failed to mark archive failed");
                }
            }
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
