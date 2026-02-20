//! 租户管理命令

use std::sync::Arc;
use tauri::State;

use crate::core::response::{ActivationResultData, ApiResponse, ErrorCode};
use crate::core::{AppState, ClientBridge};

/// P12 上传结果
#[derive(Debug, Clone, serde::Serialize)]
pub struct P12UploadResult {
    pub fingerprint: String,
    pub common_name: String,
    pub organization: Option<String>,
    pub tax_id: Option<String>,
    pub issuer: String,
    pub expires_at: Option<i64>,
}

/// 激活 Server 模式设备
///
/// 调用 /api/server/activate，保存 Server 证书到磁盘。
/// auth_url 从 AppConfig 全局配置获取，前端不再传递。
#[tauri::command]
pub async fn activate_server_tenant(
    bridge: State<'_, Arc<ClientBridge>>,
    token: String,
    replace_entity_id: Option<String>,
) -> Result<ApiResponse<ActivationResultData>, String> {
    let auth_url = bridge.get_auth_url().await;

    match bridge
        .handle_activation_with_replace(&auth_url, &token, replace_entity_id.as_deref())
        .await
    {
        Ok((tenant_id, subscription_status)) => Ok(ApiResponse::success(ActivationResultData {
            tenant_id,
            subscription_status,
            quota_info: None,
        })),
        Err(crate::core::bridge::BridgeError::Tenant(
            crate::core::tenant_manager::TenantError::DeviceLimitReached(quota_info),
        )) => {
            let mut details = std::collections::HashMap::new();
            details.insert(
                "quota_info".to_string(),
                serde_json::to_value(&quota_info).unwrap_or_default(),
            );
            Ok(ApiResponse {
                code: Some(ErrorCode::DeviceLimitReached.code()),
                message: ErrorCode::DeviceLimitReached.message().to_string(),
                data: None,
                details: Some(details),
            })
        }
        Err(e) => Ok(ApiResponse::from_bridge_error(e)),
    }
}

/// 激活 Client 模式设备
///
/// 调用 /api/client/activate，保存 Client 证书到磁盘。
#[tauri::command]
pub async fn activate_client_tenant(
    bridge: State<'_, Arc<ClientBridge>>,
    token: String,
    replace_entity_id: Option<String>,
) -> Result<ApiResponse<ActivationResultData>, String> {
    let auth_url = bridge.get_auth_url().await;

    match bridge
        .handle_client_activation_with_replace(&auth_url, &token, replace_entity_id.as_deref())
        .await
    {
        Ok((tenant_id, subscription_status)) => Ok(ApiResponse::success(ActivationResultData {
            tenant_id,
            subscription_status,
            quota_info: None,
        })),
        Err(crate::core::bridge::BridgeError::Tenant(
            crate::core::tenant_manager::TenantError::ClientLimitReached(quota_info),
        )) => {
            let mut details = std::collections::HashMap::new();
            details.insert(
                "quota_info".to_string(),
                serde_json::to_value(&quota_info).unwrap_or_default(),
            );
            Ok(ApiResponse {
                code: Some(ErrorCode::ClientLimitReached.code()),
                message: ErrorCode::ClientLimitReached.message().to_string(),
                data: None,
                details: Some(details),
            })
        }
        Err(e) => Ok(ApiResponse::from_bridge_error(e)),
    }
}

/// 验证租户凭据 (不签发证书，返回租户信息和配额)
#[tauri::command]
pub async fn verify_tenant(
    bridge: State<'_, Arc<ClientBridge>>,
    username: String,
    password: String,
) -> Result<ApiResponse<shared::activation::TenantVerifyData>, String> {
    match bridge.verify_tenant(&username, &password).await {
        Ok(data) => Ok(ApiResponse::success(data)),
        Err(e) => Ok(ApiResponse::from_bridge_error(e)),
    }
}

/// 注销当前模式 (释放配额 + 删除本地证书)
///
/// 后端自动通过 refresh_token 获取 JWT，无需前端传 token。
#[tauri::command]
pub async fn deactivate_current_mode(
    bridge: State<'_, Arc<ClientBridge>>,
) -> Result<ApiResponse<()>, String> {
    match bridge.deactivate_current_mode().await {
        Ok(()) => Ok(ApiResponse::success(())),
        Err(e) => Ok(ApiResponse::error_with_code(
            ErrorCode::InternalError,
            e.to_string(),
        )),
    }
}

/// 退出当前租户（停止服务器 + 移除租户数据）
#[tauri::command]
pub async fn exit_tenant(bridge: State<'_, Arc<ClientBridge>>) -> Result<ApiResponse<()>, String> {
    match bridge.exit_tenant().await {
        Ok(_) => Ok(ApiResponse::success(())),
        Err(e) => Ok(ApiResponse::error_with_code(
            ErrorCode::InternalError,
            e.to_string(),
        )),
    }
}

/// 上传 P12 电子签名证书到云端
///
/// **安全注意**: P12 是客户的电子签名，极度敏感。
/// - 文件内容仅在内存中短暂存在（读取 → 上传 → 丢弃）
/// - 密码不记录到日志
/// - 仅通过 HTTPS 传输到 cloud API
/// - Tauri 侧不缓存/不存储 P12 二进制或密码
///
/// 流程: refresh_token 换取 JWT → 读取本地 P12 文件 → multipart POST → 丢弃
/// 无需重新输入邮箱密码 — 使用已存储的 refresh_token 自动获取 access token
#[tauri::command]
pub async fn upload_p12(
    bridge: State<'_, Arc<ClientBridge>>,
    p12_file_path: String,
    p12_password: String,
) -> Result<ApiResponse<P12UploadResult>, String> {
    const MAX_P12_SIZE: u64 = 50 * 1024; // 50KB — P12 证书通常 < 10KB

    tracing::info!(p12_file_path = %p12_file_path, "P12 upload requested (password redacted)");

    // 1. 使用 refresh_token 获取 fresh JWT
    let token = match bridge.get_fresh_token().await {
        Ok(t) => t,
        Err(e) => {
            tracing::warn!("P12 upload: failed to get fresh token: {e}");
            return Ok(ApiResponse::error_with_code(
                ErrorCode::TokenExpired,
                format!("Session expired, please re-login: {e}"),
            ));
        }
    };

    let auth_url = bridge.get_auth_url().await;

    // 2. 读取 P12 文件（校验大小上限）
    let metadata = std::fs::metadata(&p12_file_path).map_err(|e| e.to_string())?;
    if metadata.len() > MAX_P12_SIZE {
        return Ok(ApiResponse::error_with_code(
            ErrorCode::ValidationFailed,
            format!(
                "P12 file too large ({} bytes, max {} bytes)",
                metadata.len(),
                MAX_P12_SIZE
            ),
        ));
    }
    let p12_data = match std::fs::read(&p12_file_path) {
        Ok(data) => data,
        Err(e) => {
            return Ok(ApiResponse::error_with_code(
                ErrorCode::ValidationFailed,
                format!("Failed to read P12 file: {e}"),
            ));
        }
    };

    tracing::info!(
        size_bytes = p12_data.len(),
        "P12 file loaded, uploading to cloud..."
    );

    // 3. 构造 multipart form 上传到 cloud API (HTTPS)
    let form = reqwest::multipart::Form::new()
        .text("token", token)
        .text("p12_password", p12_password)
        .part(
            "p12_file",
            reqwest::multipart::Part::bytes(p12_data)
                .file_name("certificate.p12")
                .mime_str("application/x-pkcs12")
                .unwrap(),
        );
    // p12_data 已 move 进 Part，原变量不再持有

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| e.to_string())?;

    let resp = match client
        .post(format!("{auth_url}/api/p12/upload"))
        .multipart(form)
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            tracing::error!("P12 upload network error: {e}");
            return Ok(ApiResponse::error_with_code(
                ErrorCode::NetworkError,
                format!("Upload failed: {e}"),
            ));
        }
    };

    let body: serde_json::Value = match resp.json().await {
        Ok(v) => v,
        Err(e) => {
            return Ok(ApiResponse::error_with_code(
                ErrorCode::InternalError,
                format!("Invalid response: {e}"),
            ));
        }
    };

    if body.get("success").and_then(|v| v.as_bool()) != Some(true) {
        let err_msg = body
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or("Upload failed");
        let err_code = body
            .get("error_code")
            .and_then(|v| v.as_u64())
            .unwrap_or(9001);
        tracing::warn!(error = %err_msg, code = err_code, "P12 upload rejected by cloud");
        return Ok(ApiResponse {
            code: Some(err_code as u16),
            message: err_msg.to_string(),
            data: None,
            details: None,
        });
    }

    let result = P12UploadResult {
        fingerprint: body["fingerprint"].as_str().unwrap_or("").to_string(),
        common_name: body["common_name"].as_str().unwrap_or("").to_string(),
        organization: body["organization"].as_str().map(|s| s.to_string()),
        tax_id: body["tax_id"].as_str().map(|s| s.to_string()),
        issuer: body["issuer"].as_str().unwrap_or("").to_string(),
        expires_at: body["expires_at"].as_i64(),
    };

    tracing::info!(
        fingerprint = %result.fingerprint,
        common_name = %result.common_name,
        issuer = %result.issuer,
        "P12 certificate uploaded and secured in cloud"
    );

    Ok(ApiResponse::success(result))
}

/// 获取当前租户ID
#[tauri::command]
pub async fn get_current_tenant(
    bridge: State<'_, Arc<ClientBridge>>,
) -> Result<ApiResponse<Option<String>>, String> {
    let tenant_manager = bridge.tenant_manager().read().await;
    Ok(ApiResponse::success(
        tenant_manager.current_tenant_id().map(|s| s.to_string()),
    ))
}

/// 重新检查订阅状态
///
/// 从 auth-server 同步最新订阅信息，返回更新后的 AppState。
/// 用于 SubscriptionBlockedScreen 的"重新检查"按钮。
#[tauri::command]
pub async fn check_subscription(
    bridge: State<'_, Arc<ClientBridge>>,
) -> Result<ApiResponse<AppState>, String> {
    match bridge.check_subscription().await {
        Ok(state) => Ok(ApiResponse::success(state)),
        Err(e) => Ok(ApiResponse::error_with_code(
            ErrorCode::InternalError,
            e.to_string(),
        )),
    }
}
