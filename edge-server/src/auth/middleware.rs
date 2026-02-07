//! 认证中间件
//!
//! 为 JWT 认证和授权提供 Axum 中间件

use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};

use crate::AppError;
use crate::auth::{CurrentUser, JwtService};
use crate::core::ServerState;
use crate::security_log;

/// 认证中间件 - 要求用户登录
///
/// 从 `Authorization: Bearer <token>` 头提取并验证 JWT。
/// 验证成功后将 [`CurrentUser`] 注入请求扩展 (`req.extensions_mut().insert(user)`)。
///
/// # 跳过认证的路径
///
/// - `OPTIONS *` (CORS 预检)
/// - 非 `/api/` 路径
/// - `/api/auth/login` (登录接口)
/// - `/api/message/emit` (消息发布接口)
///
/// # 错误处理
///
/// | 错误 | HTTP 状态码 |
/// |------|------------|
/// | 无 Authorization 头 | 401 Unauthorized |
/// | 令牌过期 | 401 TokenExpired |
/// | 无效令牌 | 401 InvalidToken |
pub async fn require_auth(
    State(state): State<ServerState>,
    mut req: Request,
    next: Next,
) -> Result<Response, AppError> {
    let path = req.uri().path();

    // 允许 CORS 预检的 OPTIONS 请求 (跳过认证)
    if req.method() == http::Method::OPTIONS {
        return Ok(next.run(req).await);
    }

    // 非 API 路由跳过认证 (让它们正常返回 404)
    if !path.starts_with("/api/") {
        return Ok(next.run(req).await);
    }

    // 公共 API 路由跳过认证
    let is_public_api_route = path == "/api/auth/login" || path == "/api/message/emit";
    if is_public_api_route {
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
            security_log!("WARN", "auth_missing", uri = format!("{:?}", req.uri()));
            return Err(AppError::unauthorized());
        }
    };

    // 验证令牌
    match jwt_service.validate_token(token) {
        Ok(claims) => {
            let user = CurrentUser::from(claims);
            req.extensions_mut().insert(user);
            Ok(next.run(req).await)
        }
        Err(e) => {
            security_log!(
                "WARN",
                "auth_failed",
                error = format!("{}", e),
                uri = format!("{:?}", req.uri())
            );

            match e {
                crate::auth::JwtError::ExpiredToken => Err(AppError::token_expired()),
                _ => Err(AppError::invalid_token("Invalid token")),
            }
        }
    }
}

/// 权限检查中间件 - 要求特定权限
///
/// # 参数
///
/// - `permission`: 所需权限，如 `"products:write"`, `"users:read"`
///
/// # 支持的通配符
///
/// - `"products:*"` 匹配所有 products 相关操作
/// - `"all"` 匹配所有权限
///
/// # 用法
///
/// ```ignore
/// use axum::middleware;
/// Router::new()
///     .route("/api/products", get(handler::list))
///     .layer(middleware::from_fn(require_permission("products:read")));
/// ```
///
/// # 错误
///
/// 无权限返回 403 Forbidden
pub fn require_permission(
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
                .ok_or(AppError::unauthorized())?;

            if !user.has_permission(permission) {
                security_log!(
                    "WARN",
                    "permission_denied",
                    user_id = user.id.clone(),
                    username = user.username.clone(),
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

/// 管理员中间件 - 要求管理员角色
///
/// 检查 `CurrentUser.role == "admin"`
///
/// # 错误
///
/// 非管理员返回 403 Forbidden
pub async fn require_admin(req: Request, next: Next) -> Result<Response, AppError> {
    let user = req
        .extensions()
        .get::<CurrentUser>()
        .ok_or(AppError::unauthorized())?;
    if !user.is_admin() {
        security_log!(
            "WARN",
            "admin_required",
            user_id = user.id.clone(),
            username = user.username.clone(),
            user_role = user.role_name.clone()
        );
        return Err(AppError::new(shared::ErrorCode::AdminRequired));
    }

    Ok(next.run(req).await)
}

/// 从请求中提取 CurrentUser 的扩展方法
///
/// # 示例
///
/// ```ignore
/// async fn handler(req: Request) -> Result<Json<()>, AppError> {
///     let user = req.current_user()?;
///     println!("当前用户: {}", user.username);
///     Ok(Json(()))
/// }
/// ```
pub trait CurrentUserExt {
    /// 从请求扩展中获取 CurrentUser
    ///
    /// # 错误
    ///
    /// 未认证返回 401 Unauthorized
    fn current_user(&self) -> Result<&CurrentUser, AppError>;
}

impl CurrentUserExt for Request {
    fn current_user(&self) -> Result<&CurrentUser, AppError> {
        self.extensions()
            .get::<CurrentUser>()
            .ok_or(AppError::unauthorized())
    }
}
