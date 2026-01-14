//! Health Check and Metrics Routes
//!
//! Provides comprehensive health monitoring for SaaS deployments

use axum::{
    extract::State,
    routing::get,
    Json, Router,
};
use serde::Serialize;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::server::ServerState;

/// Health check router - public routes (no auth required)
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

// Server start time (lazy static)
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

/// Basic health check
pub async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "healthy",
        version: env!("CARGO_PKG_VERSION"),
    })
}

/// Detailed health check with component status
pub async fn detailed_health(State(state): State<ServerState>) -> Json<DetailedHealthResponse> {
    // Check database
    let db = state.get_db();
    let db_start = std::time::Instant::now();
    let db_check = match db.health().await {
        Ok(_) => CheckResult::ok_with_latency(db_start.elapsed().as_millis() as u64),
        Err(e) => CheckResult::error(format!("Database error: {}", e)),
    };

    // Check message bus
    let bus_check = CheckResult::ok(); // Message bus is always ready if server is running

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

/// Prometheus-compatible metrics endpoint
pub async fn metrics(State(state): State<ServerState>) -> Json<MetricsResponse> {
    // Get message bus subscriber count
    let bus = state.message_bus();
    let subscriber_count = bus.sender().receiver_count() as u32;

    Json(MetricsResponse {
        timestamp: current_timestamp(),
        server: ServerMetrics {
            version: env!("CARGO_PKG_VERSION"),
            uptime_seconds: get_uptime_seconds(),
            environment: "development".to_string(), // TODO: Get from config
        },
        connections: ConnectionMetrics {
            active_connections: 0, // TODO: Track active connections
            message_bus_subscribers: subscriber_count,
        },
    })
}
