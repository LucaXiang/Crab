//! Archive Verify API Handlers

use axum::{
    Json,
    extract::{Path, State},
};

use crate::archiving::{DailyChainVerification, OrderVerification};
use crate::core::ServerState;
use crate::db::repository::store_info;
use crate::utils::time;
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

    let verification = archive_service.verify_order(&receipt_number).await?;

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
    let cutoff = store_info::get(&state.pool)
        .await
        .ok()
        .flatten()
        .map(|s| s.business_day_cutoff)
        .unwrap_or_else(|| "02:00".to_string());

    // 营业日结束 = 下一天的 cutoff
    let parsed_date = time::parse_date(&date)?;
    let end_date = parsed_date + chrono::Duration::days(1);

    let cutoff_time = time::parse_cutoff(&cutoff);
    let tz = state.config.timezone;
    let start = time::date_cutoff_millis(parsed_date, cutoff_time, tz);
    let end = time::date_cutoff_millis(end_date, cutoff_time, tz);

    let verification = archive_service
        .verify_daily_chain(&date, start, end)
        .await?;

    Ok(Json(verification))
}
