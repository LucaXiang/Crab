//! 认证中间件
//!
//! 为 JWT 认证和授权提供 Axum 中间件

use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};

use crate::common::AppError;
use crate::security_log;
use crate::server::ServerState;
use crate::server::auth::{CurrentUser, JwtService};

/// 需要认证中间件
///
/// 此中间件从 Authorization 头中提取并验证 JWT 令牌。
/// 如果有效，将 CurrentUser 添加到请求扩展中。
pub async fn require_auth(
    State(state): State<ServerState>,
    mut req: Request,
    next: Next,
) -> Result<Response, AppError> {
    let path = req.uri().path();

    // 允许 CORS 预检的 OPTIONS 请求 (跳过认证)
    if req.method() == http::Method::OPTIONS {
        tracing::info!("[require_auth] OPTIONS request, skipping");
        return Ok(next.run(req).await);
    }

    // 非 API 路由跳过认证 (让它们正常返回 404)
    if !path.starts_with("/api/") {
        return Ok(next.run(req).await);
    }

    // 公共 API 路由跳过认证
    let is_public_api_route = path == "/api/auth/login" || path == "/api/message/emit";
    if is_public_api_route {
        tracing::info!("[require_auth] Public API route, skipping auth: {}", path);
        return Ok(next.run(req).await);
    }

    let jwt_service = state.get_jwt_service();
    let auth_header = req
        .headers()
        .get(http::header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok());

    let token = match auth_header {
        Some(header) => JwtService::extract_from_header(header)
            .ok_or_else(|| AppError::invalid_token("Invalid authorization header"))?,
        None => {
            security_log!(WARN, "auth_missing", uri = ?req.uri());
            return Err(AppError::Unauthorized);
        }
    };

    // 验证令牌
    match jwt_service.validate_token(token) {
        Ok(claims) => {
            let user = CurrentUser::from(claims);

            tracing::info!(
                user_id = %user.id,
                username = %user.username,
                role = %user.role,
                "User authenticated successfully"
            );

            req.extensions_mut().insert(user);
            Ok(next.run(req).await)
        }
        Err(e) => {
            security_log!(WARN, "auth_failed", error = %e, uri = ?req.uri());

            match e {
                crate::server::auth::JwtError::ExpiredToken => Err(AppError::TokenExpired),
                _ => Err(AppError::invalid_token("Invalid token")),
            }
        }
    }
}

/// 需要特定权限中间件
pub async fn require_permission(
    permission: &'static str,
) -> impl Fn(
    Request,
    Next,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Response, AppError>> + Send>>
+ Clone {
    move |req: Request, next: Next| {
        Box::pin(async move {
            let user = req
                .extensions()
                .get::<CurrentUser>()
                .ok_or(AppError::Unauthorized)?;

            if !user.has_permission(permission) {
                security_log!(
                    WARN,
                    "permission_denied",
                    user_id = %user.id,
                    username = %user.username,
                    required_permission = permission
                );
                return Err(AppError::forbidden(format!(
                    "Permission denied: {}",
                    permission
                )));
            }

            Ok(next.run(req).await)
        })
    }
}

/// 需要管理员角色中间件
pub async fn require_admin(req: Request, next: Next) -> Result<Response, AppError> {
    tracing::info!("[require_admin] Called for path: {}", req.uri().path());
    let user = req
        .extensions()
        .get::<CurrentUser>()
        .ok_or(AppError::Unauthorized)?;
    if !user.is_admin() {
        security_log!(
            WARN,
            "admin_required",
            user_id = %user.id,
            username = %user.username,
            user_role = %user.role
        );
        return Err(AppError::forbidden("Admin access required".to_string()));
    }

    Ok(next.run(req).await)
}

/// 从请求中获取 CurrentUser 的扩展特征
pub trait CurrentUserExt {
    fn current_user(&self) -> Result<&CurrentUser, AppError>;
}

impl CurrentUserExt for Request {
    fn current_user(&self) -> Result<&CurrentUser, AppError> {
        self.extensions()
            .get::<CurrentUser>()
            .ok_or(AppError::Unauthorized)
    }
}
