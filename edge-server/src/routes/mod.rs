//! HTTP 路由和应用构造
//!
//! 该模块集中管理所有 HTTP 路由定义、中间件配置和请求处理逻辑。
//! 支持标准 REST API 和 Oneshot (内存中) 路由调用。

use axum::Router;
use axum::middleware as axum_middleware;
use http::{HeaderName, HeaderValue};
use tower_http::compression::CompressionLayer;
use tower_http::cors::CorsLayer;
use tower_http::request_id::{
    MakeRequestId, PropagateRequestIdLayer, RequestId, SetRequestIdLayer,
};
use tower_http::trace::TraceLayer;
use uuid::Uuid;

use crate::server::ServerState;
use crate::server::middleware;

pub mod audit;
pub mod auth;
pub mod health;
pub mod role;
pub mod upload;

pub mod router_ext;
pub use router_ext::{OneshotResult, OneshotRouter};

/// 自定义请求 ID 生成器
#[derive(Clone)]
struct XRequestId;

impl MakeRequestId for XRequestId {
    fn make_request_id<B>(&mut self, _request: &http::Request<B>) -> Option<RequestId> {
        let id = Uuid::new_v4().to_string();
        Some(RequestId::new(HeaderValue::from_str(&id).unwrap()))
    }
}

/// 构建包含所有已注册路由的路由器 (无中间件，无状态)
///
/// 用于组合各个模块的路由。
pub fn build_router() -> Router<ServerState> {
    Router::new()
        // 审计 API - 需要认证
        .merge(audit::router())
        // 管理 API - 需要管理员权限
        .merge(role::router())
        // 认证 API - 需要认证 (部分端点例外)
        .merge(auth::router())
        // 上传 API - 需要认证
        .merge(upload::router())
        // 健康检查 API - 公开路由
        .merge(health::router())
}

/// 构建配置完整的应用程序，包含所有中间件和状态
///
/// 同时用于 HTTP 服务器和 Oneshot (内存直接调用) 场景。
/// 确保所有入口点都经过相同的中间件链处理。
pub fn build_app(state: &ServerState) -> Router<ServerState> {
    build_router()
        // ========== Tower HTTP 中间件 ==========
        // CORS - 处理跨域请求
        .layer(CorsLayer::permissive())
        // 压缩 - Gzip 压缩响应
        .layer(CompressionLayer::new())
        // 请求日志 - 最外层，最先执行
        .layer(axum_middleware::from_fn(middleware::logging_middleware))
        // 追踪 - 请求追踪 (INFO 级别日志)
        .layer(TraceLayer::new_for_http())
        // ========== 应用程序中间件 ==========
        // 请求 ID - 为每个请求生成唯一 ID
        .layer(SetRequestIdLayer::new(
            HeaderName::from_static("x-request-id"),
            XRequestId,
        ))
        // 将请求 ID 传播到响应头
        .layer(PropagateRequestIdLayer::new(HeaderName::from_static(
            "x-request-id",
        )))
        // 获取用户上下文 (JWT 认证) - 在路由之前执行，注入 CurrentUser
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            crate::server::auth::require_auth,
        ))
}
