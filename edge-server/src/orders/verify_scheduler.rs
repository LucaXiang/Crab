//! 归档验证调度器
//!
//! 启动时补扫未验证的营业日，运行期间按 `business_day_cutoff` 每日触发。
//!
//! 验证结果持久化到 SurrealDB `archive_verification` 表。

use chrono::{Local, NaiveDate, NaiveTime};
use serde::{Deserialize, Serialize};
use surrealdb::engine::local::Db;
use surrealdb::Surreal;
use tokio_util::sync::CancellationToken;

use crate::db::repository::StoreInfoRepository;
use crate::orders::archive::OrderArchiveService;

// ============================================================================
// Verification Record (持久化到 SurrealDB)
// ============================================================================

const TABLE: &str = "archive_verification";

/// 验证记录（存入 SurrealDB）
#[derive(Debug, Serialize, Deserialize)]
struct VerificationRecord {
    /// "daily"
    verification_type: String,
    /// 营业日标签 "2026-01-29"
    date: Option<String>,
    total_orders: usize,
    verified_orders: usize,
    chain_intact: bool,
    chain_resets_count: usize,
    chain_breaks_count: usize,
    invalid_orders_count: usize,
    /// 有异常时存储完整 JSON，无异常时 None
    details: Option<serde_json::Value>,
}

/// 用于查询最近验证日期
#[derive(Debug, Deserialize)]
struct LastDateRow {
    date: Option<String>,
}

// ============================================================================
// VerifyScheduler
// ============================================================================

/// 归档验证调度器
///
/// 注册为 `TaskKind::Periodic`，在 `start_background_tasks()` 中启动。
pub struct VerifyScheduler {
    archive_service: OrderArchiveService,
    db: Surreal<Db>,
    shutdown: CancellationToken,
}

impl VerifyScheduler {
    pub fn new(
        archive_service: OrderArchiveService,
        db: Surreal<Db>,
        shutdown: CancellationToken,
    ) -> Self {
        Self {
            archive_service,
            db,
            shutdown,
        }
    }

    /// 主循环：启动补扫 → 周期触发
    pub async fn run(self) {
        tracing::info!("🔍 Verify scheduler started");

        // 1. 启动补扫
        if let Err(e) = self.catch_up().await {
            tracing::error!("Verify scheduler catch-up failed: {}", e);
        }

        // 2. 周期循环
        self.periodic_loop().await;

        tracing::info!("🔍 Verify scheduler stopped");
    }

    // ========================================================================
    // Startup Catch-up
    // ========================================================================

    /// 启动时补扫未验证的营业日
    async fn catch_up(&self) -> Result<(), String> {
        let (cutoff_str, cutoff_time) = self.get_cutoff().await;
        let yesterday = Self::yesterday_business_date(cutoff_time);
        self.catch_up_daily(&cutoff_str, yesterday).await?;
        Ok(())
    }

    /// 补扫缺失的每日验证
    async fn catch_up_daily(
        &self,
        cutoff: &str,
        yesterday: NaiveDate,
    ) -> Result<(), String> {
        let last_date = self.last_daily_date().await?;

        let start_date = match last_date {
            Some(d) => {
                let parsed = NaiveDate::parse_from_str(&d, "%Y-%m-%d")
                    .map_err(|e| format!("Invalid last date: {}", e))?;
                // 从 last + 1 天开始
                parsed + chrono::Duration::days(1)
            }
            None => {
                // 从未执行过：从最早的订单日期开始
                match self.earliest_order_date().await? {
                    Some(d) => d,
                    None => {
                        tracing::info!("No archived orders, skipping daily catch-up");
                        return Ok(());
                    }
                }
            }
        };

        if start_date > yesterday {
            tracing::debug!("Daily verification up to date");
            return Ok(());
        }

        let days = (yesterday - start_date).num_days() + 1;
        tracing::info!(
            "🔍 Catching up {} day(s) of daily verification ({} → {})",
            days,
            start_date,
            yesterday
        );

        let mut date = start_date;
        while date <= yesterday {
            // Fix #2: 响应 shutdown 信号
            if self.shutdown.is_cancelled() {
                tracing::info!("Verify scheduler catch-up interrupted by shutdown");
                return Ok(());
            }

            let date_str = date.format("%Y-%m-%d").to_string();
            let next = date + chrono::Duration::days(1);
            let start = format!("{}T{}:00Z", date, cutoff);
            let end = format!("{}T{}:00Z", next, cutoff);

            match self.archive_service.verify_daily_chain(&date_str, &start, &end).await {
                Ok(result) => {
                    let intact = result.chain_intact;
                    let total = result.total_orders;
                    self.save_daily_result(&date_str, &result).await;
                    if !intact {
                        tracing::warn!(
                            "⚠️ Daily chain verification for {} found issues (resets: {}, breaks: {}, invalid: {})",
                            date_str,
                            result.chain_resets.len(),
                            result.chain_breaks.len(),
                            result.invalid_orders.len()
                        );
                    } else {
                        tracing::info!("✅ Daily chain verification for {}: {} orders OK", date_str, total);
                    }
                }
                Err(e) => {
                    // Fix #4: 失败也写入记录，推进 last_daily_date 进度
                    tracing::error!("Failed to verify daily chain for {}: {}", date_str, e);
                    self.save_error_daily(&date_str, &e.to_string()).await;
                }
            }

            date = next;
        }

        Ok(())
    }

    // ========================================================================
    // Periodic Loop
    // ========================================================================

    /// 周期循环：每天在 business_day_cutoff 时间触发
    async fn periodic_loop(&self) {
        loop {
            let (cutoff_str, cutoff_time) = self.get_cutoff().await;

            // 计算下一次触发时间
            let sleep_duration = Self::duration_until_next_cutoff(cutoff_time);
            tracing::info!(
                "🔍 Next verification trigger in {} minutes",
                sleep_duration.as_secs() / 60
            );

            // 等待触发或 shutdown
            tokio::select! {
                _ = tokio::time::sleep(sleep_duration) => {}
                _ = self.shutdown.cancelled() => {
                    tracing::info!("Verify scheduler received shutdown signal");
                    return;
                }
            }

            // 触发：验证昨天的营业日
            let yesterday = Self::yesterday_business_date(cutoff_time);
            let date_str = yesterday.format("%Y-%m-%d").to_string();
            let next = yesterday + chrono::Duration::days(1);
            let start = format!("{}T{}:00Z", yesterday, cutoff_str);
            let end = format!("{}T{}:00Z", next, cutoff_str);

            tracing::info!("🔍 Running daily verification for {}", date_str);
            match self.archive_service.verify_daily_chain(&date_str, &start, &end).await {
                Ok(result) => {
                    let intact = result.chain_intact;
                    let total = result.total_orders;
                    self.save_daily_result(&date_str, &result).await;
                    if !intact {
                        tracing::warn!(
                            "⚠️ Daily verification for {}: issues found",
                            date_str
                        );
                    } else {
                        tracing::info!("✅ Daily verification for {}: {} orders OK", date_str, total);
                    }
                }
                Err(e) => {
                    tracing::error!("Daily verification failed for {}: {}", date_str, e);
                    self.save_error_daily(&date_str, &e.to_string()).await;
                }
            }
        }
    }

    // ========================================================================
    // Persistence
    // ========================================================================

    /// 保存每日验证结果
    async fn save_daily_result(
        &self,
        date: &str,
        result: &crate::orders::archive::DailyChainVerification,
    ) {
        let details = if !result.chain_intact {
            serde_json::to_value(result).ok()
        } else {
            None
        };

        let record = VerificationRecord {
            verification_type: "daily".to_string(),
            date: Some(date.to_string()),
            total_orders: result.total_orders,
            verified_orders: result.verified_orders,
            chain_intact: result.chain_intact,
            chain_resets_count: result.chain_resets.len(),
            chain_breaks_count: result.chain_breaks.len(),
            invalid_orders_count: result.invalid_orders.len(),
            details,
        };

        if let Err(e) = self.save_record(date, record).await {
            tracing::error!("Failed to save daily verification record: {}", e);
        }
    }

    /// Fix #4: 验证失败时也写入记录（chain_intact = false, 0 订单），推进进度
    async fn save_error_daily(&self, date: &str, error: &str) {
        let record = VerificationRecord {
            verification_type: "daily".to_string(),
            date: Some(date.to_string()),
            total_orders: 0,
            verified_orders: 0,
            chain_intact: false,
            chain_resets_count: 0,
            chain_breaks_count: 0,
            invalid_orders_count: 0,
            details: Some(serde_json::json!({ "error": error })),
        };

        if let Err(e) = self.save_record(date, record).await {
            tracing::error!("Failed to save error verification record: {}", e);
        }
    }

    /// Fix #3: 使用 upsert 写入 SurrealDB，按 (type, date) 去重
    async fn save_record(&self, date: &str, record: VerificationRecord) -> Result<(), String> {
        // 用 verification_type + date 作为确定性 ID，天然去重
        let id = format!("daily_{}", date.replace('-', ""));
        let _: Option<VerificationRecord> = self
            .db
            .upsert((TABLE, &id))
            .content(record)
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    // ========================================================================
    // Query Helpers
    // ========================================================================

    /// 查询最近一次 daily 验证的日期
    async fn last_daily_date(&self) -> Result<Option<String>, String> {
        let mut result = self
            .db
            .query(
                r#"
                SELECT date
                FROM archive_verification
                WHERE verification_type = "daily"
                ORDER BY date DESC
                LIMIT 1
                "#,
            )
            .await
            .map_err(|e| e.to_string())?;

        let rows: Vec<LastDateRow> = result.take(0).map_err(|e| e.to_string())?;
        Ok(rows.into_iter().next().and_then(|r| r.date))
    }

    /// 查询最早的归档订单日期
    async fn earliest_order_date(&self) -> Result<Option<NaiveDate>, String> {
        #[derive(Debug, Deserialize)]
        struct DateRow {
            created_at: String,
        }

        let mut result = self
            .db
            .query("SELECT string::slice(<string>created_at, 0, 10) AS created_at FROM order ORDER BY created_at LIMIT 1")
            .await
            .map_err(|e| e.to_string())?;

        let rows: Vec<DateRow> = result.take(0).map_err(|e| e.to_string())?;
        match rows.into_iter().next() {
            Some(r) => {
                let date = NaiveDate::parse_from_str(&r.created_at, "%Y-%m-%d")
                    .map_err(|e| format!("Invalid order date: {}", e))?;
                Ok(Some(date))
            }
            None => Ok(None),
        }
    }

    // ========================================================================
    // Time Helpers
    // ========================================================================

    /// 获取 business_day_cutoff（字符串 + NaiveTime）
    async fn get_cutoff(&self) -> (String, NaiveTime) {
        let store_repo = StoreInfoRepository::new(self.db.clone());
        let cutoff_str = store_repo
            .get()
            .await
            .ok()
            .flatten()
            .map(|s| s.business_day_cutoff)
            .unwrap_or_else(|| "00:00".to_string());
        // Fix #6: 解析失败增加 warn 日志
        let cutoff_time = match NaiveTime::parse_from_str(&cutoff_str, "%H:%M") {
            Ok(t) => t,
            Err(e) => {
                tracing::warn!(
                    "Failed to parse business_day_cutoff '{}': {}, falling back to 00:00",
                    cutoff_str,
                    e
                );
                NaiveTime::MIN
            }
        };
        (cutoff_str, cutoff_time)
    }

    /// 计算"昨天"的营业日
    ///
    /// 当前时间 >= cutoff → 当前营业日 = 今天 → 昨天 = today - 1
    /// 当前时间 < cutoff → 当前营业日 = 昨天 → 昨天 = today - 2
    fn yesterday_business_date(cutoff_time: NaiveTime) -> NaiveDate {
        let now = Local::now();
        let today_business = if now.time() < cutoff_time {
            now.date_naive() - chrono::Duration::days(1)
        } else {
            now.date_naive()
        };
        today_business - chrono::Duration::days(1)
    }

    /// 计算距离下一次 cutoff 的 Duration
    fn duration_until_next_cutoff(cutoff_time: NaiveTime) -> std::time::Duration {
        let now = Local::now();
        let today = now.date_naive();

        let target_date = if now.time() >= cutoff_time {
            // 今天的 cutoff 已过，等明天
            today + chrono::Duration::days(1)
        } else {
            today
        };

        let target_datetime = target_date
            .and_time(cutoff_time)
            .and_local_timezone(Local)
            .single()
            .unwrap_or_else(|| {
                // DST edge case: fallback to +1 min
                (target_date.and_time(cutoff_time) + chrono::Duration::minutes(1))
                    .and_local_timezone(Local)
                    .latest()
                    .expect("Cannot resolve local time")
            });

        let duration = target_datetime.signed_duration_since(now);
        if duration.num_seconds() <= 0 {
            // Safety: 不应该发生，但以防万一用 1 分钟兜底
            std::time::Duration::from_secs(60)
        } else {
            duration.to_std().unwrap_or(std::time::Duration::from_secs(60))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveTime;

    #[test]
    fn test_yesterday_business_date_after_cutoff() {
        // 测试逻辑：cutoff=06:00, 当前时间假设在 cutoff 之后
        // yesterday_business_date 返回 today_business - 1
        let cutoff = NaiveTime::from_hms_opt(6, 0, 0).unwrap();
        let result = VerifyScheduler::yesterday_business_date(cutoff);
        // 结果应该是某一天，具体取决于运行时间，这里只验证不 panic
        assert!(result < Local::now().date_naive());
    }

    #[test]
    fn test_duration_until_next_cutoff_positive() {
        // 使用一个未来的时间点
        let cutoff = NaiveTime::from_hms_opt(23, 59, 0).unwrap();
        let duration = VerifyScheduler::duration_until_next_cutoff(cutoff);
        // 应该是正值（除非恰好在 23:59 运行）
        assert!(duration.as_secs() > 0);
    }
}
