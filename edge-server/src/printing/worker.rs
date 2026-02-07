//! Kitchen Print Worker
//!
//! 监听打印事件通道，执行厨房打印。
//! 通过 EventRouter 解耦，不直接依赖 OrdersManager。

use crate::db::repository::PrintDestinationRepository;
use crate::orders::OrdersManager;
use crate::printing::{KitchenPrintService, PrintExecutor};
use crate::services::CatalogService;
use shared::order::OrderEvent;
use std::collections::HashMap;
use std::sync::Arc;
use surrealdb::engine::local::Db;
use surrealdb::Surreal;
use tokio::sync::mpsc;

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
    db: Surreal<Db>,
}

impl KitchenPrintWorker {
    pub fn new(
        orders_manager: Arc<OrdersManager>,
        kitchen_print_service: Arc<KitchenPrintService>,
        catalog_service: Arc<CatalogService>,
        db: Surreal<Db>,
    ) -> Self {
        Self {
            orders_manager,
            kitchen_print_service,
            catalog_service,
            db,
        }
    }

    /// 运行工作者（阻塞直到通道关闭）
    ///
    /// 接收来自 EventRouter 的 mpsc 通道（已过滤为仅 ItemsAdded）
    pub async fn run(self, mut event_rx: mpsc::Receiver<ArcOrderEvent>) {
        tracing::info!("Kitchen print worker started");
        let executor = PrintExecutor::new();

        while let Some(event) = event_rx.recv().await {
            // EventRouter 已过滤，这里都是 ItemsAdded 事件
            self.handle_items_added(&event, &executor).await;
        }

        tracing::info!("Print channel closed, kitchen print worker stopping");
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
        let order = match self.kitchen_print_service.get_kitchen_order(kitchen_order_id) {
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
        let repo = PrintDestinationRepository::new(self.db.clone());
        let destinations = match repo.find_all().await {
            Ok(d) => d,
            Err(e) => {
                tracing::error!(error = ?e, "Failed to load print destinations");
                return;
            }
        };

        let dest_map: HashMap<String, _> = destinations
            .into_iter()
            .filter_map(|d| d.id.as_ref().map(|id| (id.to_string(), d.clone())))
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
