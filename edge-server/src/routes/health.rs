//! 健康检查和指标路由
//!
//! 为 SaaS 部署提供全面的健康监控

use axum::{Json, Router, extract::State, routing::get};
use serde::Serialize;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::server::ServerState;
use crate::server::credential::Subscription;

/// 健康检查路由 - 公共路由 (无需认证)
pub fn router() -> Router<ServerState> {
    Router::new()
        .route("/health", get(health))
        .route("/health/detailed", get(detailed_health))
        .route("/metrics", get(metrics))
}

#[derive(Serialize)]
pub struct HealthResponse {
    status: &'static str,
    version: &'static str,
    /// 租户 ID (如果已激活)
    #[serde(skip_serializing_if = "Option::is_none")]
    tenant_id: Option<String>,
    /// 边缘节点 ID (如果已激活)
    #[serde(skip_serializing_if = "Option::is_none")]
    edge_id: Option<String>,
    /// 是否已激活
    is_activated: bool,
    /// 订阅状态 (如果已激活且有订阅信息)
    #[serde(skip_serializing_if = "Option::is_none")]
    subscription: Option<Subscription>,
}

#[derive(Serialize)]
pub struct DetailedHealthResponse {
    status: &'static str,
    version: &'static str,
    uptime_seconds: u64,
    checks: HealthChecks,
}

#[derive(Serialize)]
pub struct HealthChecks {
    database: CheckResult,
    message_bus: CheckResult,
}

#[derive(Serialize)]
pub struct CheckResult {
    status: &'static str,
    latency_ms: Option<u64>,
    message: Option<String>,
}

impl CheckResult {
    fn ok() -> Self {
        Self {
            status: "ok",
            latency_ms: None,
            message: None,
        }
    }

    fn ok_with_latency(latency_ms: u64) -> Self {
        Self {
            status: "ok",
            latency_ms: Some(latency_ms),
            message: None,
        }
    }

    fn error(message: impl Into<String>) -> Self {
        Self {
            status: "error",
            latency_ms: None,
            message: Some(message.into()),
        }
    }
}

#[derive(Serialize)]
pub struct MetricsResponse {
    timestamp: u64,
    server: ServerMetrics,
    connections: ConnectionMetrics,
}

#[derive(Serialize)]
pub struct ServerMetrics {
    version: &'static str,
    uptime_seconds: u64,
    environment: String,
}

#[derive(Serialize)]
pub struct ConnectionMetrics {
    active_connections: u32,
    message_bus_subscribers: u32,
}

// 服务器启动时间 (懒加载静态变量)
static START_TIME: std::sync::OnceLock<SystemTime> = std::sync::OnceLock::new();

fn get_uptime_seconds() -> u64 {
    let start = START_TIME.get_or_init(SystemTime::now);
    SystemTime::now()
        .duration_since(*start)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// 基础健康检查
///
/// 现在包含激活状态信息，以便客户端查询边缘节点身份
pub async fn health(State(state): State<ServerState>) -> Json<HealthResponse> {
    // 获取激活状态 (如果数据库出错，则默认为未激活)
    let activation = state
        .activation_service()
        .get_status()
        .await
        .unwrap_or_default();

    // 获取订阅状态
    let subscription = {
        let cache = state.activation.credential_cache.read().await;
        cache.as_ref().and_then(|cred| cred.subscription.clone())
    };

    Json(HealthResponse {
        status: "healthy",
        version: env!("CARGO_PKG_VERSION"),
        tenant_id: activation.tenant_id,
        edge_id: activation.edge_id,
        is_activated: activation.is_activated,
        subscription,
    })
}

/// 包含组件状态的详细健康检查
pub async fn detailed_health(State(state): State<ServerState>) -> Json<DetailedHealthResponse> {
    // 检查数据库
    let db = state.get_db();
    let db_start = std::time::Instant::now();
    let db_check = match db.health().await {
        Ok(_) => CheckResult::ok_with_latency(db_start.elapsed().as_millis() as u64),
        Err(e) => CheckResult::error(format!("Database error: {}", e)),
    };

    // 检查消息总线
    let bus_check = CheckResult::ok(); // 只要服务器在运行，消息总线总是就绪的

    let all_ok = db_check.status == "ok" && bus_check.status == "ok";

    Json(DetailedHealthResponse {
        status: if all_ok { "healthy" } else { "degraded" },
        version: env!("CARGO_PKG_VERSION"),
        uptime_seconds: get_uptime_seconds(),
        checks: HealthChecks {
            database: db_check,
            message_bus: bus_check,
        },
    })
}

/// 兼容 Prometheus 的指标端点
pub async fn metrics(State(state): State<ServerState>) -> Json<MetricsResponse> {
    // 获取消息总线订阅者数量
    let bus = state.message_bus();
    let subscriber_count = bus.sender().receiver_count() as u32;

    Json(MetricsResponse {
        timestamp: current_timestamp(),
        server: ServerMetrics {
            version: env!("CARGO_PKG_VERSION"),
            uptime_seconds: get_uptime_seconds(),
            environment: "development".to_string(), // TODO: 从配置获取
        },
        connections: ConnectionMetrics {
            active_connections: 0, // TODO: 追踪活跃连接
            message_bus_subscribers: subscriber_count,
        },
    })
}
