//! Archive Verify API Handlers

use axum::{
    extract::{Path, State},
    Json,
};

use crate::core::ServerState;
use crate::db::repository::StoreInfoRepository;
use crate::orders::archive::{DailyChainVerification, OrderVerification};
use crate::utils::{AppError, AppResult};

/// GET /api/archive/verify/order/:receipt_number
/// 验证单个订单的事件哈希链完整性
pub async fn verify_order(
    State(state): State<ServerState>,
    Path(receipt_number): Path<String>,
) -> AppResult<Json<OrderVerification>> {
    let archive_service = state
        .orders_manager
        .archive_service()
        .ok_or_else(|| AppError::internal("Archive service not available"))?;

    let verification = archive_service
        .verify_order(&receipt_number)
        .await
        ?;

    Ok(Json(verification))
}

/// GET /api/archive/verify/daily/:date
/// 验证指定营业日的订单哈希链连续性
/// date 格式: YYYY-MM-DD，按 business_day_cutoff 计算时间范围
pub async fn verify_daily_chain(
    State(state): State<ServerState>,
    Path(date): Path<String>,
) -> AppResult<Json<DailyChainVerification>> {
    let archive_service = state
        .orders_manager
        .archive_service()
        .ok_or_else(|| AppError::internal("Archive service not available"))?;

    // 从 store_info 获取营业日分割时间
    let store_repo = StoreInfoRepository::new(state.db.clone());
    let cutoff = store_repo
        .get()
        .await
        .ok()
        .flatten()
        .map(|s| s.business_day_cutoff)
        .unwrap_or_else(|| "00:00".to_string());

    // 营业日结束 = 下一天的 cutoff
    let parsed_date = chrono::NaiveDate::parse_from_str(&date, "%Y-%m-%d")
        .map_err(|e| AppError::validation(format!("Invalid date format: {}", e)))?;
    let end_date = parsed_date + chrono::Duration::days(1);

    let cutoff_time = chrono::NaiveTime::parse_from_str(&format!("{}:00", cutoff), "%H:%M:%S")
        .unwrap_or(chrono::NaiveTime::MIN);
    let start = parsed_date.and_time(cutoff_time).and_utc().timestamp_millis();
    let end = end_date.and_time(cutoff_time).and_utc().timestamp_millis();

    let verification = archive_service
        .verify_daily_chain(&date, start, end)
        .await
        ?;

    Ok(Json(verification))
}


