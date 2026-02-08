//! Archive Worker - Processes pending archive queue
//! Archive Worker - 订单归档处理
//!
//! 监听终端事件通道，处理订单归档。
//! 通过 EventRouter 解耦，不直接依赖 OrdersManager。
//!
//! Note: redb operations are synchronous for stability.

use super::archive::OrderArchiveService;
use super::money::{to_decimal, to_f64};
use super::storage::{OrderStorage, PendingArchive};
use crate::audit::{AuditAction, AuditService};
use crate::db::repository::{payment, shift};
use rust_decimal::prelude::*;
use shared::order::{OrderEvent, OrderEventType, OrderSnapshot};
use sqlx::SqlitePool;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;

/// Arc-wrapped OrderEvent (from EventRouter)
type ArcOrderEvent = Arc<OrderEvent>;

/// Terminal event types (用于 shift cash 判断)
const TERMINAL_EVENT_TYPES: &[OrderEventType] = &[
    OrderEventType::OrderCompleted,
    OrderEventType::OrderVoided,
    OrderEventType::OrderMerged,
];

/// Archive worker configuration
const MAX_RETRY_COUNT: u32 = 3;
const RETRY_BASE_DELAY_SECS: u64 = 5;
const RETRY_MAX_DELAY_SECS: u64 = 60; // 1 minute max
const QUEUE_SCAN_INTERVAL_SECS: u64 = 60;
/// 并发归档数量（单店场景 10 即可，避免 SQLite 写入压力）
const ARCHIVE_CONCURRENCY: usize = 10;

/// Worker for processing archive queue (支持并发归档)
///
/// 通过 EventRouter 解耦，接收 mpsc 通道（已过滤为终端事件）
pub struct ArchiveWorker {
    storage: OrderStorage,
    archive_service: OrderArchiveService,
    audit_service: Arc<AuditService>,
    pool: SqlitePool,
    semaphore: Arc<tokio::sync::Semaphore>,
}

impl ArchiveWorker {
    pub fn new(
        storage: OrderStorage,
        archive_service: OrderArchiveService,
        audit_service: Arc<AuditService>,
        pool: SqlitePool,
    ) -> Self {
        Self {
            storage,
            archive_service,
            audit_service,
            pool,
            semaphore: Arc::new(tokio::sync::Semaphore::new(ARCHIVE_CONCURRENCY)),
        }
    }

    /// Run the archive worker (并发处理归档)
    ///
    /// 接收来自 EventRouter 的 mpsc 通道（已过滤为终端事件）
    pub async fn run(self, mut event_rx: mpsc::Receiver<ArcOrderEvent>) {
        tracing::info!("ArchiveWorker started with concurrency={}", ARCHIVE_CONCURRENCY);

        let worker = Arc::new(self);

        // Recover dead letter entries (previously failed archives) back to pending queue
        match worker.storage.recover_dead_letters() {
            Ok(0) => {}
            Ok(n) => tracing::info!(count = n, "Recovered dead letter entries to pending queue"),
            Err(e) => tracing::error!(error = %e, "Failed to recover dead letter entries"),
        }

        // Process any pending archives from previous run
        worker.process_pending_queue().await;

        let mut scan_interval =
            tokio::time::interval(Duration::from_secs(QUEUE_SCAN_INTERVAL_SECS));

        loop {
            tokio::select! {
                // Handle new terminal events (EventRouter 已过滤)
                event_opt = event_rx.recv() => {
                    match event_opt {
                        Some(event) => {
                            tracing::debug!(order_id = %event.order_id, event_type = ?event.event_type, "Received terminal event");
                            // 并发处理归档
                            let w = worker.clone();
                            let order_id = event.order_id.clone();
                            tokio::spawn(async move {
                                w.process_order_concurrent(&order_id).await;
                            });
                        }
                        None => {
                            tracing::info!("Archive channel closed, shutting down ArchiveWorker");
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
        let _permit = self.semaphore.acquire().await.expect("semaphore is never closed");
        self.process_order(order_id).await;
    }

    /// Process all pending archives
    async fn process_pending_queue(&self) {
        // Get pending archives (synchronous redb operation)
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
                "Max retry count exceeded, moving to dead letter queue"
            );
            // Move to dead letter queue for manual recovery
            let error = entry.last_error.as_deref().unwrap_or("Unknown error");
            let _ = self.storage.move_to_dead_letter(&entry.order_id, error);
            return false;
        }

        // Exponential backoff: delay = base * 2^retry_count, capped at max
        let delay_secs =
            (RETRY_BASE_DELAY_SECS * 2u64.pow(entry.retry_count)).min(RETRY_MAX_DELAY_SECS);
        let retry_after_ms = entry.created_at + (delay_secs as i64 * 1000);
        let now = shared::util::now_millis();

        now >= retry_after_ms
    }

    /// Process a single order archive
    ///
    /// redb operations are synchronous for stability.
    async fn process_order(&self, order_id: &str) {
        // 1. Load snapshot and events from redb (synchronous)
        let (snapshot, events) = match self.load_order_data(order_id) {
            Some(data) => data,
            None => return,
        };

        // 2. Archive to SQLite (async)
        match self.archive_service.archive_order(&snapshot, events.clone()).await {
            Ok(()) => {
                tracing::info!(order_id = %order_id, "Order archived successfully");

                // 3. Update shift expected_cash for cash payments
                self.update_shift_cash(&snapshot, &events).await;

                // 4. Write payment records to independent payment table
                self.write_payment_records(&snapshot, &events).await;

                // 5. Write audit log for terminal event
                self.write_order_audit(&snapshot, &events).await;

                // 6. Cleanup redb (synchronous)
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

    /// Load order data from redb (synchronous helper)
    fn load_order_data(&self, order_id: &str) -> Option<(OrderSnapshot, Vec<OrderEvent>)> {
        let snapshot = match self.storage.get_snapshot(order_id) {
            Ok(Some(s)) => s,
            Ok(None) => {
                tracing::warn!(order_id = %order_id, "Snapshot not found, removing from queue");
                let _ = self.storage.remove_from_pending(order_id);
                return None;
            }
            Err(e) => {
                tracing::error!(order_id = %order_id, error = %e, "Failed to load snapshot from redb");
                return None;
            }
        };

        let events = match self.storage.get_events_for_order(order_id) {
            Ok(e) => e,
            Err(e) => {
                tracing::error!(order_id = %order_id, error = %e, "Failed to load events from redb");
                return None;
            }
        };

        Some((snapshot, events))
    }

    /// Write payment records to independent payment table (for statistics/reconciliation)
    async fn write_payment_records(&self, snapshot: &OrderSnapshot, events: &[OrderEvent]) {
        if snapshot.payments.is_empty() {
            return;
        }

        // Extract operator from terminal event
        let (op_id, op_name) = events
            .iter()
            .rev()
            .find(|e| {
                matches!(
                    e.event_type,
                    OrderEventType::OrderCompleted | OrderEventType::OrderVoided
                )
            })
            .map(|e| (Some(e.operator_id.to_string()), Some(e.operator_name.as_str())))
            .unwrap_or((None, None));

        match payment::create_from_snapshot(&self.pool, snapshot, op_id.as_deref(), op_name).await {
            Ok(count) => {
                tracing::info!(
                    order_id = %snapshot.order_id,
                    payment_count = count,
                    "Payment records written to payment table"
                );
            }
            Err(e) => {
                // Non-fatal: payment table is a projection, not critical path
                tracing::warn!(
                    order_id = %snapshot.order_id,
                    error = %e,
                    "Failed to write payment records"
                );
            }
        }
    }

    /// Write audit log entry for the terminal event in the order
    async fn write_order_audit(&self, snapshot: &OrderSnapshot, events: &[OrderEvent]) {
        use shared::order::EventPayload;

        // Find the terminal event (last event that triggered archival)
        let terminal = events
            .iter()
            .rev()
            .find(|e| TERMINAL_EVENT_TYPES.contains(&e.event_type));

        let Some(event) = terminal else { return };

        let action = match event.event_type {
            OrderEventType::OrderCompleted => AuditAction::OrderCompleted,
            OrderEventType::OrderVoided => AuditAction::OrderVoided,
            OrderEventType::OrderMerged => AuditAction::OrderMerged,
            _ => return,
        };

        // Common fields
        let mut details = serde_json::json!({
            "receipt_number": snapshot.receipt_number,
            "status": serde_json::to_value(snapshot.status).unwrap_or_default(),
            "total": snapshot.total,
            "item_count": snapshot.items.len(),
        });

        // Event-specific details + target
        let mut target: Option<String> = None;

        match &event.payload {
            EventPayload::OrderCompleted { payment_summary, .. } => {
                let summary: Vec<serde_json::Value> = payment_summary
                    .iter()
                    .map(|p| serde_json::json!({ "method": p.method, "amount": p.amount }))
                    .collect();
                details["payment_summary"] = serde_json::Value::Array(summary);
                details["paid_amount"] = serde_json::json!(snapshot.paid_amount);
                if let Some(ref table_name) = snapshot.table_name {
                    details["table_name"] = serde_json::json!(table_name);
                }
                if let Some(ref zone_name) = snapshot.zone_name {
                    details["zone_name"] = serde_json::json!(zone_name);
                }
            }
            EventPayload::OrderVoided {
                void_type,
                loss_reason,
                loss_amount,
                note,
                authorizer_id,
                authorizer_name,
            } => {
                details["void_type"] = serde_json::to_value(void_type).unwrap_or_default();
                if let Some(reason) = loss_reason {
                    details["loss_reason"] = serde_json::to_value(reason).unwrap_or_default();
                }
                if let Some(amount) = loss_amount {
                    details["loss_amount"] = serde_json::json!(amount);
                }
                if let Some(n) = note {
                    details["void_note"] = serde_json::json!(n);
                }
                if let Some(id) = authorizer_id {
                    details["authorizer_id"] = serde_json::json!(id);
                }
                if let Some(name) = authorizer_name {
                    details["authorizer_name"] = serde_json::json!(name);
                }
            }
            EventPayload::OrderMerged {
                source_table_id,
                source_table_name,
                items,
                payments,
                paid_amount,
                authorizer_id,
                authorizer_name,
                ..
            } => {
                details["source_table"] = serde_json::json!(source_table_name);
                details["merged_item_count"] = serde_json::json!(items.len());
                details["merged_payment_count"] = serde_json::json!(payments.len());
                details["merged_paid_amount"] = serde_json::json!(paid_amount);
                if let Some(id) = authorizer_id {
                    details["authorizer_id"] = serde_json::json!(id);
                }
                if let Some(name) = authorizer_name {
                    details["authorizer_name"] = serde_json::json!(name);
                }
                // target points to the source table (where items came from)
                target = Some(source_table_id.to_string());
            }
            _ => {}
        }

        let resource_id = format!("order:{}", snapshot.order_id);
        self.audit_service
            .log_with_target(
                action,
                "order",
                &resource_id,
                Some(event.operator_id.to_string()),
                Some(event.operator_name.clone()),
                details,
                target,
            )
            .await;
    }

    /// Update shift expected_cash for cash payments in the order
    async fn update_shift_cash(&self, snapshot: &OrderSnapshot, events: &[OrderEvent]) {
        use shared::order::{OrderStatus, VoidType};

        // Skip cash tracking for CANCELLED void orders (no money changed hands)
        // LOSS_SETTLED void orders should still count cash (it was actually received)
        if snapshot.status == OrderStatus::Void
            && let Some(ref void_type) = snapshot.void_type
            && *void_type == VoidType::Cancelled
        {
            tracing::info!(
                order_id = %snapshot.order_id,
                void_type = ?void_type,
                "Skipping cash tracking for CANCELLED void order"
            );
            return;
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
            .map(|e| e.operator_id);

        let Some(operator_id) = operator_id else {
            tracing::warn!(
                order_id = %snapshot.order_id,
                event_types = ?events.iter().map(|e| &e.event_type).collect::<Vec<_>>(),
                "No terminal event found for cash tracking"
            );
            return;
        };

        let cash_amount = to_f64(cash_total);
        if let Err(e) = shift::add_cash_payment(&self.pool, operator_id, cash_amount).await {
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
        assert!(TERMINAL_EVENT_TYPES.contains(&OrderEventType::OrderMerged));
        assert!(!TERMINAL_EVENT_TYPES.contains(&OrderEventType::ItemsAdded));
    }

    #[test]
    fn test_backoff_calculation() {
        // Test exponential backoff formula (max 3 retries, max 60s delay)
        let base = RETRY_BASE_DELAY_SECS;
        let max = RETRY_MAX_DELAY_SECS;

        assert_eq!((base * 2u64.pow(0)).min(max), 5); // retry 0: 5s
        assert_eq!((base * 2u64.pow(1)).min(max), 10); // retry 1: 10s
        assert_eq!((base * 2u64.pow(2)).min(max), 20); // retry 2: 20s
        assert_eq!((base * 2u64.pow(3)).min(max), 40); // retry 3: 40s (but max is 3, so won't happen)
        assert_eq!((base * 2u64.pow(4)).min(max), 60); // capped at 60s
    }
}
