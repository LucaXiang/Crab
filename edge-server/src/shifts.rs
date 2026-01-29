//! 班次自动关闭调度器
//!
//! 在 `business_day_cutoff` 时间点自动关闭跨营业日的僵尸班次。
//! 启动时立即扫描一次，之后按 cutoff 时间每日触发。

use chrono::{Local, NaiveTime};
use tokio_util::sync::CancellationToken;

use crate::core::ServerState;
use crate::db::repository::{ShiftRepository, StoreInfoRepository};

const RESOURCE: &str = "shift";

/// 班次自动关闭调度器
///
/// 注册为 `TaskKind::Periodic`，在 `start_background_tasks()` 中启动。
pub struct ShiftAutoCloseScheduler {
    state: ServerState,
    shutdown: CancellationToken,
}

impl ShiftAutoCloseScheduler {
    pub fn new(state: ServerState, shutdown: CancellationToken) -> Self {
        Self { state, shutdown }
    }

    /// 主循环：启动扫描 → 每日 cutoff 定点触发
    pub async fn run(self) {
        tracing::info!("🕐 Shift auto-close scheduler started");

        // 1. 启动时立即扫描
        self.recover_and_broadcast().await;

        // 2. 定点循环
        loop {
            let cutoff_time = self.get_cutoff_time().await;
            let sleep_duration = Self::duration_until_next_cutoff(cutoff_time);

            tracing::info!(
                "🕐 Next shift auto-close check in {} minutes",
                sleep_duration.as_secs() / 60
            );

            tokio::select! {
                _ = tokio::time::sleep(sleep_duration) => {}
                _ = self.shutdown.cancelled() => {
                    tracing::info!("Shift auto-close scheduler received shutdown signal");
                    return;
                }
            }

            self.recover_and_broadcast().await;
        }
    }

    /// 执行恢复 + 广播
    async fn recover_and_broadcast(&self) {
        let cutoff_time = self.get_cutoff_time().await;
        let business_day_start = Self::business_day_start(cutoff_time);

        let repo = ShiftRepository::new(self.state.db.clone());
        match repo.recover_stale_shifts(&business_day_start).await {
            Ok(shifts) if shifts.is_empty() => {
                tracing::debug!("No stale shifts to recover");
            }
            Ok(shifts) => {
                tracing::info!(
                    "🕐 Auto-closed {} stale shift(s) (business_day_start={})",
                    shifts.len(),
                    business_day_start
                );
                for shift in &shifts {
                    let id = shift
                        .id
                        .as_ref()
                        .map(|id| id.to_string())
                        .unwrap_or_default();
                    self.state
                        .broadcast_sync(RESOURCE, "recovered", &id, Some(shift))
                        .await;
                }
            }
            Err(e) => {
                tracing::error!("Failed to recover stale shifts: {}", e);
            }
        }
    }

    /// 获取 cutoff 时间
    async fn get_cutoff_time(&self) -> NaiveTime {
        let store_repo = StoreInfoRepository::new(self.state.db.clone());
        let cutoff_str = store_repo
            .get()
            .await
            .ok()
            .flatten()
            .map(|s| s.business_day_cutoff)
            .unwrap_or_else(|| "00:00".to_string());

        match NaiveTime::parse_from_str(&cutoff_str, "%H:%M") {
            Ok(t) => t,
            Err(e) => {
                tracing::warn!(
                    "Failed to parse business_day_cutoff '{}': {}, falling back to 00:00",
                    cutoff_str,
                    e
                );
                NaiveTime::MIN
            }
        }
    }

    /// 计算当前营业日起始时间（ISO 8601 字符串）
    fn business_day_start(cutoff_time: NaiveTime) -> String {
        let now = Local::now();
        let today_business_date = if now.time() < cutoff_time {
            (now - chrono::Duration::days(1)).date_naive()
        } else {
            now.date_naive()
        };
        format!("{}T{}:00Z", today_business_date, cutoff_time.format("%H:%M"))
    }

    /// 计算距离下一次 cutoff 的 Duration
    fn duration_until_next_cutoff(cutoff_time: NaiveTime) -> std::time::Duration {
        let now = Local::now();
        let today = now.date_naive();

        let target_date = if now.time() >= cutoff_time {
            today + chrono::Duration::days(1)
        } else {
            today
        };

        let target_datetime = target_date
            .and_time(cutoff_time)
            .and_local_timezone(Local)
            .single()
            .unwrap_or_else(|| {
                (target_date.and_time(cutoff_time) + chrono::Duration::minutes(1))
                    .and_local_timezone(Local)
                    .latest()
                    .expect("Cannot resolve local time")
            });

        let duration = target_datetime.signed_duration_since(now);
        if duration.num_seconds() <= 0 {
            std::time::Duration::from_secs(60)
        } else {
            duration
                .to_std()
                .unwrap_or(std::time::Duration::from_secs(60))
        }
    }
}
