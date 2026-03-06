//! 班次过期检测调度器
//!
//! 在 `business_day_cutoff` 时间点检测跨营业日的未关闭班次，
//! 广播 `settlement_required` 通知前端弹窗要求操作员手动结算。
//!
//! 支持 `config_notify` 信号：修改 cutoff 后立即重算下次触发时间。

use std::sync::Arc;

use chrono::NaiveTime;
use tokio::sync::Notify;
use tokio_util::sync::CancellationToken;

use crate::core::ServerState;
use crate::db::repository::{shift, store_info};
use crate::utils::time;
use shared::message::SyncChangeType;

use shared::cloud::SyncResource;
const RESOURCE: SyncResource = SyncResource::Shift;

/// 班次过期检测调度器
///
/// 注册为 `TaskKind::Periodic`，在 `start_background_tasks()` 中启动。
pub struct ShiftAutoCloseScheduler {
    state: ServerState,
    shutdown: CancellationToken,
    config_notify: Arc<Notify>,
}

impl ShiftAutoCloseScheduler {
    pub fn new(state: ServerState, shutdown: CancellationToken) -> Self {
        let config_notify = state.config_notify.clone();
        Self {
            state,
            shutdown,
            config_notify,
        }
    }

    /// 主循环：启动扫描 + cutoff 定点触发 + 配置变更响应
    pub async fn run(self) {
        tracing::info!("Shift settlement detector started");

        // 启动时立即扫描一次
        self.detect_and_notify().await;

        loop {
            let cutoff_time = self.get_cutoff_time().await;
            let tz = self.state.config.timezone;
            let sleep_duration = time::duration_until_next_cutoff(cutoff_time, tz);

            tracing::info!(
                "Next settlement check in {} minutes (cutoff={})",
                sleep_duration.as_secs() / 60,
                cutoff_time.format("%H:%M")
            );

            tokio::select! {
                // 等到下次 cutoff 时间点
                _ = tokio::time::sleep(sleep_duration) => {
                    self.detect_and_notify().await;
                }
                // 配置变更 → 重新计算 sleep（不检测，只重算）
                _ = self.config_notify.notified() => {
                    tracing::info!("Config changed, recalculating next cutoff");
                    // 配置变更后也扫描一次，因为新 cutoff 可能使当前班次变为过期
                    self.detect_and_notify().await;
                }
                // 关机信号
                _ = self.shutdown.cancelled() => {
                    tracing::info!("Shift settlement detector received shutdown signal");
                    return;
                }
            }
        }
    }

    /// 检测过期班次并广播通知（不修改数据）
    async fn detect_and_notify(&self) {
        let cutoff_time = self.get_cutoff_time().await;
        let tz = self.state.config.timezone;
        let today = time::current_business_date(cutoff_time, tz);
        let business_day_start = time::date_cutoff_millis(today, cutoff_time, tz);

        match shift::find_stale_shifts(&self.state.pool, business_day_start).await {
            Ok(shifts) if shifts.is_empty() => {
                tracing::debug!("No stale shifts detected");
            }
            Ok(shifts) => {
                tracing::info!(
                    "Detected {} stale shift(s), broadcasting settlement_required",
                    shifts.len()
                );
                for s in &shifts {
                    self.state
                        .broadcast_sync(
                            RESOURCE,
                            SyncChangeType::SettlementRequired,
                            s.id,
                            Some(s),
                            false,
                        )
                        .await;
                }
            }
            Err(e) => {
                tracing::error!("Failed to detect stale shifts: {}", e);
            }
        }
    }

    /// 获取 cutoff 时间（每次从 DB 读取，支持动态修改）
    async fn get_cutoff_time(&self) -> NaiveTime {
        let cutoff = store_info::get(&self.state.pool)
            .await
            .ok()
            .flatten()
            .map(|s| s.business_day_cutoff)
            .unwrap_or(0);

        time::cutoff_to_time(cutoff)
    }
}
