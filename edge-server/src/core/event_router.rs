//! Event Router - 事件路由与分发
//!
//! 解耦 OrdersManager 和各个 Worker，提供独立的通道。
//!
//! ```text
//! OrdersManager (broadcast)
//!        │
//!        └── EventRouter
//!               ├── mpsc ──► ArchiveWorker (terminal events only) [CRITICAL]
//!               ├── mpsc ──► KitchenPrintWorker (ItemsAdded only) [best-effort]
//!               └── mpsc ──► OrderSyncForwarder (all events) [best-effort]
//! ```
//!
//! ## 优先级策略
//!
//! - **Archive**: 关键业务，阻塞发送保证不丢失
//! - **Sync/Print**: Best-effort，满则丢弃（不阻塞关键路径）

use shared::order::{OrderEvent, OrderEventType};
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc};

/// 终端事件类型（触发归档）
const TERMINAL_EVENTS: &[OrderEventType] = &[
    OrderEventType::OrderCompleted,
    OrderEventType::OrderVoided,
    OrderEventType::OrderMerged,
];

/// 事件通道集合
pub struct EventChannels {
    /// 归档事件（仅终端事件）- Arc 包装减少克隆开销
    pub archive_rx: mpsc::Receiver<Arc<OrderEvent>>,
    /// 打印事件（仅 ItemsAdded）
    pub print_rx: mpsc::Receiver<Arc<OrderEvent>>,
    /// 同步事件（所有事件）
    pub sync_rx: mpsc::Receiver<Arc<OrderEvent>>,
}

/// 事件路由器
///
/// 订阅 OrdersManager 的 broadcast，按类型分发到独立的 mpsc 通道。
/// 使用 Arc<OrderEvent> 减少克隆开销。
pub struct EventRouter {
    archive_tx: mpsc::Sender<Arc<OrderEvent>>,
    print_tx: mpsc::Sender<Arc<OrderEvent>>,
    sync_tx: mpsc::Sender<Arc<OrderEvent>>,
}

impl EventRouter {
    /// 创建路由器和通道
    ///
    /// # 参数
    /// - `archive_buffer`: 归档通道 buffer（关键业务，建议较大）
    /// - `other_buffer`: 其他通道 buffer（best-effort）
    pub fn new(archive_buffer: usize, other_buffer: usize) -> (Self, EventChannels) {
        let (archive_tx, archive_rx) = mpsc::channel(archive_buffer);
        let (print_tx, print_rx) = mpsc::channel(other_buffer);
        let (sync_tx, sync_rx) = mpsc::channel(other_buffer);

        let router = Self {
            archive_tx,
            print_tx,
            sync_tx,
        };

        let channels = EventChannels {
            archive_rx,
            print_rx,
            sync_rx,
        };

        (router, channels)
    }

    /// 运行路由器（阻塞直到源通道关闭）
    pub async fn run(self, mut source: broadcast::Receiver<OrderEvent>) {
        tracing::info!("Event router started");

        loop {
            match source.recv().await {
                Ok(event) => {
                    self.dispatch(event).await;
                }
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    // ⚠️ Lag 是严重问题 - 可能丢失归档事件
                    tracing::error!(
                        skipped = n,
                        "Event router lagged! Events skipped - archive data may be lost"
                    );
                }
                Err(broadcast::error::RecvError::Closed) => {
                    tracing::info!("Source channel closed, event router stopping");
                    break;
                }
            }
        }
    }

    /// 分发事件到对应通道
    ///
    /// 优先级策略：
    /// 1. Archive: 阻塞发送，保证不丢失（关键业务）
    /// 2. Sync/Print: try_send，满则丢弃（不阻塞关键路径）
    async fn dispatch(&self, event: OrderEvent) {
        let event = Arc::new(event);

        // 1. 归档通道优先：阻塞发送保证不丢失（关键业务）
        if TERMINAL_EVENTS.contains(&event.event_type)
            && self.archive_tx.send(Arc::clone(&event)).await.is_err()
        {
            tracing::error!("Archive channel closed - critical data may be lost!");
        }

        // 2. 同步通道：best-effort，满则丢弃
        match self.sync_tx.try_send(Arc::clone(&event)) {
            Ok(()) => {}
            Err(mpsc::error::TrySendError::Full(_)) => {
                tracing::warn!(
                    order_id = %event.order_id,
                    event_type = ?event.event_type,
                    "Sync channel full, event dropped"
                );
            }
            Err(mpsc::error::TrySendError::Closed(_)) => {
                tracing::debug!("Sync channel closed");
            }
        }

        // 3. 打印通道：best-effort，满则丢弃
        if event.event_type == OrderEventType::ItemsAdded {
            match self.print_tx.try_send(Arc::clone(&event)) {
                Ok(()) => {}
                Err(mpsc::error::TrySendError::Full(_)) => {
                    tracing::warn!(
                        order_id = %event.order_id,
                        "Print channel full, print job dropped"
                    );
                }
                Err(mpsc::error::TrySendError::Closed(_)) => {
                    tracing::debug!("Print channel closed");
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::order::EventPayload;
    use shared::order::types::ServiceType;

    fn make_test_event(event_type: OrderEventType, sequence: u64) -> OrderEvent {
        let payload = match event_type {
            OrderEventType::ItemsAdded => EventPayload::ItemsAdded { items: vec![] },
            OrderEventType::OrderCompleted => EventPayload::OrderCompleted {
                receipt_number: "TEST-001".to_string(),
                service_type: Some(ServiceType::DineIn),
                final_total: 100.0,
                payment_summary: vec![],
            },
            _ => EventPayload::OrderInfoUpdated {
                guest_count: None,
                table_name: None,
                is_pre_payment: None,
            },
        };

        OrderEvent {
            event_id: uuid::Uuid::new_v4().to_string(),
            sequence,
            order_id: "test".to_string(),
            timestamp: shared::util::now_millis(),
            client_timestamp: None,
            operator_id: 1,
            operator_name: "Test Operator".to_string(),
            command_id: uuid::Uuid::new_v4().to_string(),
            event_type,
            payload,
        }
    }

    #[tokio::test]
    async fn test_event_routing() {
        let (router, mut channels) = EventRouter::new(16, 16);
        let (tx, rx) = broadcast::channel(16);

        // Spawn router
        tokio::spawn(async move {
            router.run(rx).await;
        });

        // Send ItemsAdded event
        let items_added = make_test_event(OrderEventType::ItemsAdded, 1);
        tx.send(items_added).unwrap();

        // Should receive on sync and print channels (as Arc)
        assert!(channels.sync_rx.recv().await.is_some());
        assert!(channels.print_rx.recv().await.is_some());

        // Send OrderCompleted event
        let completed = make_test_event(OrderEventType::OrderCompleted, 2);
        tx.send(completed).unwrap();

        // Should receive on sync and archive channels
        assert!(channels.sync_rx.recv().await.is_some());
        assert!(channels.archive_rx.recv().await.is_some());
    }

    #[tokio::test]
    async fn test_archive_priority() {
        // Archive channel should not be affected by slow sync channel
        let (router, mut channels) = EventRouter::new(16, 1); // sync buffer = 1
        let (tx, rx) = broadcast::channel(16);

        tokio::spawn(async move {
            router.run(rx).await;
        });

        // Fill sync channel (buffer = 1)
        tx.send(make_test_event(OrderEventType::ItemsAdded, 1))
            .unwrap();
        tx.send(make_test_event(OrderEventType::ItemsAdded, 2))
            .unwrap();

        // Send terminal event - should still reach archive
        tx.send(make_test_event(OrderEventType::OrderCompleted, 3))
            .unwrap();

        // Archive should receive the terminal event
        let archived = channels.archive_rx.recv().await;
        assert!(archived.is_some());
        assert_eq!(archived.unwrap().sequence, 3);
    }
}
