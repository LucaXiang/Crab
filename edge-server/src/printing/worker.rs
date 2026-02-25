//! Kitchen Print Worker
//!
//! 监听打印事件通道，执行厨房打印。
//! 通过 EventRouter 解耦，不直接依赖 OrdersManager。

use crate::db::repository::print_destination;
use crate::orders::OrdersManager;
use crate::printing::{KitchenPrintService, PrintExecutor};
use crate::services::CatalogService;
use chrono_tz::Tz;
use shared::order::{OrderEvent, OrderEventType};
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

/// Arc-wrapped OrderEvent (from EventRouter)
type ArcOrderEvent = Arc<OrderEvent>;

/// 厨房打印工作者
///
/// 监听打印事件通道（ItemsAdded + OrderCompleted），执行厨房打印。
/// - ItemsAdded: 堂食立即打印，零售创建记录但跳过打印
/// - OrderCompleted: 零售订单完成时执行打印
pub struct KitchenPrintWorker {
    orders_manager: Arc<OrdersManager>,
    kitchen_print_service: Arc<KitchenPrintService>,
    catalog_service: Arc<CatalogService>,
    pool: SqlitePool,
    timezone: Tz,
}

impl KitchenPrintWorker {
    pub fn new(
        orders_manager: Arc<OrdersManager>,
        kitchen_print_service: Arc<KitchenPrintService>,
        catalog_service: Arc<CatalogService>,
        pool: SqlitePool,
        timezone: Tz,
    ) -> Self {
        Self {
            orders_manager,
            kitchen_print_service,
            catalog_service,
            pool,
            timezone,
        }
    }

    /// 运行工作者（阻塞直到通道关闭）
    ///
    /// 接收来自 EventRouter 的 mpsc 通道（ItemsAdded + OrderCompleted）
    pub async fn run(
        self,
        mut event_rx: mpsc::Receiver<ArcOrderEvent>,
        shutdown: CancellationToken,
    ) {
        tracing::info!("Kitchen print worker started");
        let executor = PrintExecutor::with_config(48, self.timezone);

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
                    match event.event_type {
                        OrderEventType::ItemsAdded => {
                            self.handle_items_added(&event, &executor).await;
                        }
                        OrderEventType::OrderCompleted => {
                            self.handle_order_completed(&event, &executor).await;
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    /// 处理 ItemsAdded 事件
    async fn handle_items_added(&self, event: &OrderEvent, executor: &PrintExecutor) {
        tracing::debug!(
            order_id = %event.order_id,
            event_id = %event.event_id,
            "handle_items_added: start"
        );

        // Get full snapshot
        let snapshot = match self.orders_manager.get_snapshot(&event.order_id) {
            Ok(Some(s)) => s,
            Ok(None) => {
                tracing::warn!(order_id = %event.order_id, "Snapshot not found");
                return;
            }
            Err(e) => {
                tracing::error!(order_id = %event.order_id, error = ?e, "Failed to get snapshot");
                return;
            }
        };

        tracing::debug!(
            table_name = ?snapshot.table_name,
            queue_number = ?snapshot.queue_number,
            is_retail = snapshot.is_retail,
            "handle_items_added: order context loaded"
        );

        // Process the event (create KitchenOrder + LabelPrintRecord)
        match self.kitchen_print_service.process_items_added(
            event,
            &snapshot,
            &self.catalog_service,
        ) {
            Ok(Some(kitchen_order_id)) => {
                tracing::info!(
                    order_id = %event.order_id,
                    kitchen_order_id = %kitchen_order_id,
                    is_retail = snapshot.is_retail,
                    "Created kitchen order"
                );

                // 零售模式：创建记录但跳过打印（等 OrderCompleted 再打）
                if snapshot.is_retail {
                    tracing::debug!(
                        order_id = %event.order_id,
                        "Retail order: deferring print until OrderCompleted"
                    );
                    return;
                }

                // 堂食模式：立即打印
                self.execute_print(&kitchen_order_id, executor).await;
                self.execute_label_print(&event.order_id, &kitchen_order_id, executor)
                    .await;
            }
            Ok(None) => {
                tracing::debug!(
                    order_id = %event.order_id,
                    "handle_items_added: no print records created"
                );
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

    /// 处理 OrderCompleted 事件（零售订单延迟打印）
    async fn handle_order_completed(&self, event: &OrderEvent, executor: &PrintExecutor) {
        // 读取该订单所有 KitchenOrder
        let kitchen_orders = match self
            .kitchen_print_service
            .get_kitchen_orders_for_order(&event.order_id)
        {
            Ok(orders) => orders,
            Err(e) => {
                tracing::error!(
                    order_id = %event.order_id,
                    error = ?e,
                    "Failed to get kitchen orders for completed order"
                );
                return;
            }
        };

        // 仅处理零售订单（is_retail=true 且 print_count=0 的记录）
        let pending: Vec<_> = kitchen_orders
            .iter()
            .filter(|ko| ko.is_retail && ko.print_count == 0)
            .collect();

        if pending.is_empty() {
            return;
        }

        tracing::info!(
            order_id = %event.order_id,
            pending_count = pending.len(),
            "OrderCompleted: executing deferred retail prints"
        );

        for ko in pending {
            self.execute_print(&ko.id, executor).await;
            self.execute_label_print(&event.order_id, &ko.id, executor)
                .await;
        }
    }

    /// 执行厨房打印
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

    /// 执行标签打印 (Windows: GDI+ 渲染)
    #[cfg(windows)]
    async fn execute_label_print(
        &self,
        order_id: &str,
        kitchen_order_id: &str,
        executor: &PrintExecutor,
    ) {
        use crate::db::repository::label_template;

        // 获取该 kitchen order 关联的标签记录
        let records = match self
            .kitchen_print_service
            .get_label_records_for_order(order_id)
        {
            Ok(r) => r
                .into_iter()
                .filter(|r| r.kitchen_order_id == kitchen_order_id)
                .collect::<Vec<_>>(),
            Err(e) => {
                tracing::error!(error = ?e, "Failed to load label records");
                return;
            }
        };

        if records.is_empty() {
            tracing::debug!(
                order_id = %order_id,
                kitchen_order_id = %kitchen_order_id,
                "execute_label_print: no label records for this kitchen order"
            );
            return;
        }

        tracing::debug!(
            order_id = %order_id,
            kitchen_order_id = %kitchen_order_id,
            label_records_count = records.len(),
            "execute_label_print: printing labels"
        );

        // 加载默认标签模板
        let template = match label_template::get_default(&self.pool).await {
            Ok(Some(db_tmpl)) => super::executor::convert_label_template(&db_tmpl),
            Ok(None) => {
                tracing::warn!("No default label template, using built-in default");
                crab_printer::label::LabelTemplate::default()
            }
            Err(e) => {
                tracing::error!(error = ?e, "Failed to load label template");
                crab_printer::label::LabelTemplate::default()
            }
        };

        // 加载打印目的地
        let destinations = match print_destination::find_all(&self.pool).await {
            Ok(d) => d.into_iter().map(|d| (d.id.to_string(), d)).collect(),
            Err(e) => {
                tracing::error!(error = ?e, "Failed to load print destinations for labels");
                return;
            }
        };

        if let Err(e) = executor
            .print_label_records(&records, &destinations, &template)
            .await
        {
            tracing::error!(order_id = %order_id, error = %e, "Failed to print labels");
        }
    }

    /// 执行标签打印 (非 Windows: 不支持)
    #[cfg(not(windows))]
    async fn execute_label_print(
        &self,
        order_id: &str,
        kitchen_order_id: &str,
        executor: &PrintExecutor,
    ) {
        // 获取该 kitchen order 关联的标签记录
        let records = match self
            .kitchen_print_service
            .get_label_records_for_order(order_id)
        {
            Ok(r) => r
                .into_iter()
                .filter(|r| r.kitchen_order_id == kitchen_order_id)
                .collect::<Vec<_>>(),
            Err(e) => {
                tracing::error!(error = ?e, "Failed to load label records");
                return;
            }
        };

        if records.is_empty() {
            return;
        }

        // 加载打印目的地
        let destinations = match print_destination::find_all(&self.pool).await {
            Ok(d) => d.into_iter().map(|d| (d.id.to_string(), d)).collect(),
            Err(e) => {
                tracing::error!(error = ?e, "Failed to load print destinations for labels");
                return;
            }
        };

        if let Err(e) = executor.print_label_records(&records, &destinations).await {
            tracing::error!(order_id = %order_id, error = %e, "Failed to print labels");
        }
    }
}
