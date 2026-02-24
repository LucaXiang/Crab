//! Kitchen Print Worker
//!
//! 监听打印事件通道，执行厨房打印。
//! 通过 EventRouter 解耦，不直接依赖 OrdersManager。

use crate::db::repository::print_destination;
use crate::orders::OrdersManager;
use crate::printing::{KitchenPrintService, PrintExecutor};
use crate::services::CatalogService;
use chrono_tz::Tz;
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
    /// 接收来自 EventRouter 的 mpsc 通道（已过滤为仅 ItemsAdded）
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
        tracing::debug!(
            order_id = %event.order_id,
            event_id = %event.event_id,
            "handle_items_added: start"
        );

        // Get table name and queue_number from order snapshot
        let (table_name, queue_number) = self
            .orders_manager
            .get_snapshot(&event.order_id)
            .ok()
            .flatten()
            .map(|s| (s.table_name, s.queue_number))
            .unwrap_or((None, None));

        tracing::debug!(
            table_name = ?table_name,
            queue_number = ?queue_number,
            "handle_items_added: order context loaded"
        );

        // Process the event (create KitchenOrder record)
        match self.kitchen_print_service.process_items_added(
            event,
            table_name.clone(),
            &self.catalog_service,
        ) {
            Ok(Some(kitchen_order_id)) => {
                tracing::info!(
                    order_id = %event.order_id,
                    kitchen_order_id = %kitchen_order_id,
                    "Created kitchen order"
                );

                // Execute kitchen printing
                self.execute_print(&kitchen_order_id, executor).await;

                // Execute label printing
                self.execute_label_print(
                    &event.order_id,
                    &kitchen_order_id,
                    executor,
                    table_name.as_deref(),
                    queue_number,
                )
                .await;
            }
            Ok(None) => {
                tracing::debug!(
                    order_id = %event.order_id,
                    "handle_items_added: no print records created (printing disabled or no matching destinations)"
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

    /// 执行标签打印 (Windows: GDI+ 渲染)
    #[cfg(windows)]
    async fn execute_label_print(
        &self,
        order_id: &str,
        kitchen_order_id: &str,
        executor: &PrintExecutor,
        table_name: Option<&str>,
        queue_number: Option<u32>,
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
            .print_label_records(&records, &destinations, &template, table_name, queue_number)
            .await
        {
            tracing::error!(order_id = %order_id, error = %e, "Failed to print labels");
        }
    }

    /// 执行标签打印 (非 Windows: 不支持)
    #[cfg(not(windows))]
    async fn execute_label_print(
        &self,
        _order_id: &str,
        _kitchen_order_id: &str,
        executor: &PrintExecutor,
        table_name: Option<&str>,
        queue_number: Option<u32>,
    ) {
        // 获取该 kitchen order 关联的标签记录
        let records = match self
            .kitchen_print_service
            .get_label_records_for_order(_order_id)
        {
            Ok(r) => r
                .into_iter()
                .filter(|r| r.kitchen_order_id == _kitchen_order_id)
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

        if let Err(e) = executor
            .print_label_records(&records, &destinations, table_name, queue_number)
            .await
        {
            tracing::error!(order_id = %_order_id, error = %e, "Failed to print labels");
        }
    }
}
