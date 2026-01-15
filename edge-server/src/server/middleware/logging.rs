//! 请求日志中间件
//!
//! 记录所有进入的 HTTP 请求，包含时间、用户信息和状态码

use axum::{
    extract::{MatchedPath, Request},
    middleware::Next,
    response::Response,
};
use std::time::Instant;
use tracing::{info, warn};

/// 请求日志中间件
///
/// 记录请求开始和结束，包含以下信息：
/// - 请求 ID (x-request-id)
/// - HTTP 方法和路径
/// - 用户代理 (User Agent)
/// - 认证用户 (如果存在)
/// - 响应状态码
/// - 请求延迟 (毫秒)
pub async fn logging_middleware(req: Request, next: Next) -> Response {
    let start = Instant::now();

    // 从请求头获取 Request ID，如果不存在则生成一个
    let request_id = req
        .headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    let method = req.method().clone();
    let uri = req.uri().clone();
    let path = req
        .extensions()
        .get::<MatchedPath>()
        .map(|p| p.as_str().to_string())
        .unwrap_or_else(|| uri.path().to_string());

    let user_agent = req
        .headers()
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown");

    let client_version = req
        .headers()
        .get("x-client-version")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown");

    // 如果已认证，提取用户信息
    let user_info = req
        .extensions()
        .get::<crate::server::auth::CurrentUser>()
        .map(|u| format!("{}({})", u.username, u.id));

    info!(
        request_id = %request_id,
        method = %method,
        path = %path,
        user_agent = %user_agent,
        client_version = %client_version,
        user = ?user_info,
        "Request started"
    );

    let response = next.run(req).await;

    let latency = start.elapsed();
    let status = response.status();

    // 根据状态码使用不同级别记录日志
    if status.is_server_error() {
        warn!(
            request_id = %request_id,
            method = %method,
            path = %path,
            status = %status.as_u16(),
            latency_ms = %latency.as_millis(),
            user = ?user_info,
            "Request completed with server error"
        );
    } else if status.is_client_error() {
        warn!(
            request_id = %request_id,
            method = %method,
            path = %path,
            status = %status.as_u16(),
            latency_ms = %latency.as_millis(),
            user = ?user_info,
            "Request completed with client error"
        );
    } else {
        info!(
            request_id = %request_id,
            method = %method,
            path = %path,
            status = %status.as_u16(),
            latency_ms = %latency.as_millis(),
            user = ?user_info,
            "Request completed successfully"
        );
    }

    response
}
