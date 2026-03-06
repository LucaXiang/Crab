//! 日报自动生成调度器
//!
//! 在 `business_day_cutoff` 时间点自动生成前一营业日的日报。
//! 启动时补漏最近 7 天缺失的日报，定期清理超过 30 天的旧日报。
//!
//! 支持 `config_notify` 信号：修改 cutoff 后立即重算下次触发时间。

use std::sync::Arc;

use chrono::NaiveTime;
use tokio::sync::Notify;
use tokio_util::sync::CancellationToken;

use crate::core::ServerState;
use crate::db::repository::{daily_report, store_info};
use crate::utils::time;
use shared::message::SyncChangeType;
use shared::models::DailyReportGenerate;

use shared::cloud::SyncResource;
const RESOURCE: SyncResource = SyncResource::DailyReport;

/// 日报保留天数 (Edge 本地)
const RETENTION_DAYS: i32 = 30;

/// 启动补漏天数
const CATCHUP_DAYS: i64 = 7;

/// 日报自动生成调度器
///
/// 注册为 `TaskKind::Periodic`，在 `start_background_tasks()` 中启动。
pub struct DailyReportScheduler {
    state: ServerState,
    shutdown: CancellationToken,
    config_notify: Arc<Notify>,
}

impl DailyReportScheduler {
    pub fn new(state: ServerState, shutdown: CancellationToken) -> Self {
        let config_notify = state.config_notify.clone();
        Self {
            state,
            shutdown,
            config_notify,
        }
    }

    /// 主循环：启动补漏 + cutoff 定点触发 + 配置变更响应
    pub async fn run(self) {
        tracing::info!("Daily report scheduler started");

        // 启动时补漏 + 清理
        self.catchup_missing_reports().await;
        self.cleanup_old_reports().await;

        loop {
            let cutoff_time = self.get_cutoff_time().await;
            let tz = self.state.config.timezone;
            let sleep_duration = time::duration_until_next_cutoff(cutoff_time, tz);

            tracing::info!(
                "Next daily report generation in {} minutes (cutoff={})",
                sleep_duration.as_secs() / 60,
                cutoff_time.format("%H:%M")
            );

            tokio::select! {
                // 等到下次 cutoff 时间点
                _ = tokio::time::sleep(sleep_duration) => {
                    self.generate_for_previous_day().await;
                    self.cleanup_old_reports().await;
                }
                // 配置变更 → 重新计算 sleep
                _ = self.config_notify.notified() => {
                    tracing::info!("Config changed, recalculating next daily report cutoff");
                }
                // 关机信号
                _ = self.shutdown.cancelled() => {
                    tracing::info!("Daily report scheduler received shutdown signal");
                    return;
                }
            }
        }
    }

    /// 为前一营业日自动生成日报
    async fn generate_for_previous_day(&self) {
        let cutoff_time = self.get_cutoff_time().await;
        let tz = self.state.config.timezone;
        let today = time::current_business_date(cutoff_time, tz);

        // 前一营业日
        let prev_day = today - chrono::Duration::days(1);
        self.generate_for_date(prev_day).await;
    }

    /// 补漏最近 N 天缺失的日报
    async fn catchup_missing_reports(&self) {
        let cutoff_time = self.get_cutoff_time().await;
        let tz = self.state.config.timezone;
        let today = time::current_business_date(cutoff_time, tz);

        let mut generated = 0u32;
        for i in 1..=CATCHUP_DAYS {
            let date = today - chrono::Duration::days(i);
            let date_str = date.format("%Y-%m-%d").to_string();

            // 跳过已存在的日报
            match daily_report::find_by_date(&self.state.pool, &date_str).await {
                Ok(Some(_)) => continue,
                Ok(None) => {}
                Err(e) => {
                    tracing::warn!("Failed to check daily report for {}: {}", date_str, e);
                    continue;
                }
            }

            self.generate_for_date(date).await;
            generated += 1;
        }

        if generated > 0 {
            tracing::info!("Catchup: generated {} missing daily report(s)", generated);
        } else {
            tracing::debug!("Catchup: no missing daily reports");
        }
    }

    /// 为指定日期生成日报（幂等：已存在则跳过）
    async fn generate_for_date(&self, date: chrono::NaiveDate) {
        let date_str = date.format("%Y-%m-%d").to_string();
        let tz = self.state.config.timezone;

        // 幂等检查
        match daily_report::find_by_date(&self.state.pool, &date_str).await {
            Ok(Some(_)) => {
                tracing::debug!("Daily report for {} already exists, skipping", date_str);
                return;
            }
            Ok(None) => {}
            Err(e) => {
                tracing::error!("Failed to check daily report for {}: {}", date_str, e);
                return;
            }
        }

        let start_millis = time::day_start_millis(date, tz);
        let end_millis = time::day_end_millis(date, tz);

        let payload = DailyReportGenerate {
            business_date: date_str.clone(),
            note: None,
        };

        match daily_report::generate(
            &self.state.pool,
            payload,
            start_millis,
            end_millis,
            None, // auto-generated, no operator
            None,
            true, // auto_generated
        )
        .await
        {
            Ok(report) => {
                tracing::info!(
                    "Auto-generated daily report for {} (id={}, orders={}, revenue={:.2})",
                    date_str,
                    report.id,
                    report.total_orders,
                    report.net_revenue
                );
                self.state
                    .broadcast_sync(
                        RESOURCE,
                        SyncChangeType::Created,
                        report.id,
                        Some(&report),
                        false,
                    )
                    .await;
            }
            Err(e) => {
                tracing::error!(
                    "Failed to auto-generate daily report for {}: {}",
                    date_str,
                    e
                );
            }
        }
    }

    /// 清理超过保留期的旧日报
    async fn cleanup_old_reports(&self) {
        match daily_report::cleanup_old_reports(&self.state.pool, RETENTION_DAYS).await {
            Ok(0) => {
                tracing::debug!("No old daily reports to clean up");
            }
            Ok(count) => {
                tracing::info!(
                    "Cleaned up {} old daily report(s) (>{} days)",
                    count,
                    RETENTION_DAYS
                );
            }
            Err(e) => {
                tracing::error!("Failed to clean up old daily reports: {}", e);
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
