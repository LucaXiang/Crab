//! 班次过期检测调度器
//!
//! 在 `business_day_cutoff` 时间点检测跨营业日的未关闭班次，
//! 广播 `settlement_required` 通知前端弹窗要求操作员手动结算。
//!
//! 支持 `config_notify` 信号：修改 cutoff 后立即重算下次触发时间。

use std::sync::Arc;

use chrono::NaiveTime;
use chrono_tz::Tz;
use tokio::sync::Notify;
use tokio_util::sync::CancellationToken;

use crate::core::ServerState;
use crate::db::repository::{ShiftRepository, StoreInfoRepository};
use crate::utils::time;

const RESOURCE: &str = "shift";

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
            let sleep_duration = Self::duration_until_next_cutoff(cutoff_time, tz);

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

        let repo = ShiftRepository::new(self.state.db.clone());
        match repo.find_stale_shifts(business_day_start).await {
            Ok(shifts) if shifts.is_empty() => {
                tracing::debug!("No stale shifts detected");
            }
            Ok(shifts) => {
                tracing::info!(
                    "Detected {} stale shift(s), broadcasting settlement_required",
                    shifts.len()
                );
                for shift in &shifts {
                    let id = shift
                        .id
                        .as_ref()
                        .map(|id| id.to_string())
                        .unwrap_or_default();

                    self.state
                        .broadcast_sync(RESOURCE, "settlement_required", &id, Some(shift))
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
        let store_repo = StoreInfoRepository::new(self.state.db.clone());
        let cutoff_str = store_repo
            .get()
            .await
            .ok()
            .flatten()
            .map(|s| s.business_day_cutoff)
            .unwrap_or_else(|| "02:00".to_string());

        time::parse_cutoff(&cutoff_str)
    }

    /// 计算距离下一次 cutoff 的 Duration
    fn duration_until_next_cutoff(cutoff_time: NaiveTime, tz: Tz) -> std::time::Duration {
        let now = chrono::Utc::now().with_timezone(&tz);
        let today = now.date_naive();

        let target_date = if now.time() >= cutoff_time {
            // 今天的 cutoff 已过，等明天
            today + chrono::Duration::days(1)
        } else {
            today
        };

        let target_datetime = target_date
            .and_time(cutoff_time)
            .and_local_timezone(tz)
            .single()
            .unwrap_or_else(|| {
                // DST edge case: fallback to +1 min
                (target_date.and_time(cutoff_time) + chrono::Duration::minutes(1))
                    .and_local_timezone(tz)
                    .latest()
                    .unwrap_or_else(|| {
                        // Ultimate fallback: use current time + 1 hour
                        tracing::error!("Cannot resolve local time for shift close, using fallback");
                        now + chrono::Duration::hours(1)
                    })
            });

        let duration = target_datetime.signed_duration_since(now);
        if duration.num_seconds() <= 0 {
            // Safety: 不应该发生，但以防万一用 1 分钟兜底
            std::time::Duration::from_secs(60)
        } else {
            duration
                .to_std()
                .unwrap_or(std::time::Duration::from_secs(60))
        }
    }
}
