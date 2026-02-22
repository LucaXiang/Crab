//! Kitchen Print Worker
//!
//! 监听打印事件通道，执行厨房打印。
//! 通过 EventRouter 解耦，不直接依赖 OrdersManager。

use crate::db::repository::print_destination;
use crate::orders::OrdersManager;
use crate::printing::{KitchenPrintService, PrintExecutor};
use crate::services::CatalogService;
use shared::order::OrderEvent;
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

/// Arc-wrapped OrderEvent (from EventRouter)
type ArcOrderEvent = Arc<OrderEvent>;

/// 厨房打印工作者
///
/// 监听打印事件通道（仅 ItemsAdded），执行厨房打印。
/// 通过 EventRouter 解耦，不直接依赖 OrdersManager 的 broadcast。
pub struct KitchenPrintWorker {
    orders_manager: Arc<OrdersManager>,
    kitchen_print_service: Arc<KitchenPrintService>,
    catalog_service: Arc<CatalogService>,
    pool: SqlitePool,
}

impl KitchenPrintWorker {
    pub fn new(
        orders_manager: Arc<OrdersManager>,
        kitchen_print_service: Arc<KitchenPrintService>,
        catalog_service: Arc<CatalogService>,
        pool: SqlitePool,
    ) -> Self {
        Self {
            orders_manager,
            kitchen_print_service,
            catalog_service,
            pool,
        }
    }

    /// 运行工作者（阻塞直到通道关闭）
    ///
    /// 接收来自 EventRouter 的 mpsc 通道（已过滤为仅 ItemsAdded）
    pub async fn run(
        self,
        mut event_rx: mpsc::Receiver<ArcOrderEvent>,
        shutdown: CancellationToken,
    ) {
        tracing::info!("Kitchen print worker started");
        let executor = PrintExecutor::new();

        loop {
            tokio::select! {
                _ = shutdown.cancelled() => {
                    tracing::info!("Kitchen print worker received shutdown signal");
                    break;
                }
                event = event_rx.recv() => {
                    let Some(event) = event else {
                        tracing::info!("Print channel closed, kitchen print worker stopping");
                        break;
                    };
                    self.handle_items_added(&event, &executor).await;
                }
            }
        }
    }

    /// 处理 ItemsAdded 事件
    async fn handle_items_added(
        &self,
        event: &shared::order::OrderEvent,
        executor: &PrintExecutor,
    ) {
        // Get table name from order snapshot
        let table_name = self
            .orders_manager
            .get_snapshot(&event.order_id)
            .ok()
            .flatten()
            .and_then(|s| s.table_name);

        // Process the event (create KitchenOrder record)
        match self.kitchen_print_service.process_items_added(
            event,
            table_name,
            &self.catalog_service,
        ) {
            Ok(Some(kitchen_order_id)) => {
                tracing::info!(
                    order_id = %event.order_id,
                    kitchen_order_id = %kitchen_order_id,
                    "Created kitchen order"
                );

                // Execute actual printing
                self.execute_print(&kitchen_order_id, executor).await;
            }
            Ok(None) => {
                // Printing not enabled or no items to print
            }
            Err(e) => {
                tracing::error!(
                    order_id = %event.order_id,
                    error = ?e,
                    "Failed to process ItemsAdded for printing"
                );
            }
        }
    }

    /// 执行打印
    async fn execute_print(&self, kitchen_order_id: &str, executor: &PrintExecutor) {
        let order = match self
            .kitchen_print_service
            .get_kitchen_order(kitchen_order_id)
        {
            Ok(Some(o)) => o,
            Ok(None) => return,
            Err(e) => {
                tracing::error!(
                    kitchen_order_id = %kitchen_order_id,
                    error = ?e,
                    "Failed to get kitchen order"
                );
                return;
            }
        };

        // Load print destinations
        let destinations = match print_destination::find_all(&self.pool).await {
            Ok(d) => d,
            Err(e) => {
                tracing::error!(error = ?e, "Failed to load print destinations");
                return;
            }
        };

        let dest_map: HashMap<String, _> = destinations
            .into_iter()
            .map(|d| (d.id.to_string(), d))
            .collect();

        if let Err(e) = executor.print_kitchen_order(&order, &dest_map).await {
            tracing::error!(
                kitchen_order_id = %kitchen_order_id,
                error = %e,
                "Failed to execute print job"
            );
        }
    }
}
