use crate::auth::tenant_auth;
use crate::db::{p12, tenants};
use crate::state::AppState;
use axum::Json;
use axum::extract::{Multipart, State};
use axum::http::HeaderMap;
use base64::Engine;
use shared::error::ErrorCode;
use zeroize::Zeroize;

pub async fn upload_p12(
    State(state): State<AppState>,
    headers: HeaderMap,
    mut multipart: Multipart,
) -> Json<serde_json::Value> {
    let mut p12_password = None;
    let mut p12_data = None;
    // Legacy: also accept token from form body for backwards compatibility
    let mut form_token = None;

    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.name().unwrap_or_default().to_string();
        match name.as_str() {
            "token" => {
                form_token = field.text().await.ok();
            }
            "p12_password" => {
                p12_password = field.text().await.ok();
            }
            "p12_file" => {
                p12_data = field.bytes().await.ok();
            }
            _ => {}
        }
    }

    // Prefer Authorization header, fallback to form body token
    let token = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(String::from)
        .or(form_token);

    let (Some(token), Some(mut p12_password), Some(p12_data)) = (token, p12_password, p12_data)
    else {
        return Json(serde_json::json!({
            "success": false,
            "error": "Missing required fields: p12_password, p12_file (and Authorization header)",
            "error_code": ErrorCode::RequiredField
        }));
    };

    // JWT 认证
    let tenant_id = match tenant_auth::verify_token(&token, &state.jwt_secret) {
        Ok(claims) => claims.sub,
        Err(_) => {
            p12_password.zeroize();
            return Json(serde_json::json!({
                "success": false,
                "error": "Invalid or expired token",
                "error_code": ErrorCode::TokenExpired
            }));
        }
    };

    let tenant = match tenants::find_by_id(&state.pool, &tenant_id).await {
        Ok(Some(t)) => t,
        Ok(None) => {
            p12_password.zeroize();
            return Json(serde_json::json!({
                "success": false,
                "error": "Tenant not found",
                "error_code": ErrorCode::TenantCredentialsInvalid
            }));
        }
        Err(e) => {
            p12_password.zeroize();
            tracing::error!(error = %e, "Database error finding tenant");
            return Json(serde_json::json!({
                "success": false,
                "error": "Internal error",
                "error_code": ErrorCode::InternalError
            }));
        }
    };

    // 验证 P12 文件有效性（密码正确、证书链可解析、FNMT 签发）
    let cert_info = match crab_cert::parse_p12(&p12_data, &p12_password) {
        Ok(info) => info,
        Err(e) => {
            p12_password.zeroize();
            let (error_code, detail) = map_p12_error(&e);
            tracing::warn!(
                tenant_id = %tenant.id,
                error_code = ?error_code,
                detail = %detail,
                "P12 validation failed"
            );
            return Json(serde_json::json!({
                "success": false,
                "error": error_code.message(),
                "error_detail": detail,
                "error_code": error_code
            }));
        }
    };

    tracing::info!(
        tenant_id = %tenant.id,
        fingerprint = %cert_info.fingerprint,
        common_name = %cert_info.common_name,
        issuer = %cert_info.issuer,
        tax_id = ?cert_info.tax_id(),
        expires_at = cert_info.expires_at,
        "P12 validated: issued by trusted Spanish CA"
    );

    // Base64 编码 P12 数据，加密后存入 PostgreSQL
    let p12_base64 = base64::engine::general_purpose::STANDARD.encode(&p12_data);

    if let Err(e) = p12::upsert(
        &state.pool,
        &state.master_key,
        &tenant.id,
        &p12_base64,
        &p12_password,
        &cert_info,
    )
    .await
    {
        p12_password.zeroize();
        tracing::error!(error = %e, "Failed to save P12 to database");
        return Json(serde_json::json!({
            "success": false,
            "error": "Failed to save certificate",
            "error_code": ErrorCode::InternalError
        }));
    }

    // P12 已存入数据库，清零内存中的密码
    p12_password.zeroize();

    tracing::info!(
        tenant_id = %tenant.id,
        fingerprint = %cert_info.fingerprint,
        "P12 certificate uploaded and encrypted in database"
    );

    Json(serde_json::json!({
        "success": true,
        "fingerprint": cert_info.fingerprint,
        "common_name": cert_info.common_name,
        "organization": cert_info.organization,
        "tax_id": cert_info.tax_id(),
        "issuer": cert_info.issuer,
        "expires_at": cert_info.expires_at
    }))
}

/// Map crab_cert P12 errors to specific ErrorCode + detail string
fn map_p12_error(e: &crab_cert::CertError) -> (ErrorCode, String) {
    use crab_cert::CertError;
    match e {
        CertError::P12InvalidFormat(detail) => (ErrorCode::P12InvalidFormat, detail.clone()),
        CertError::P12WrongPassword(detail) => (ErrorCode::P12WrongPassword, detail.clone()),
        CertError::P12MissingPrivateKey => (ErrorCode::P12MissingPrivateKey, e.to_string()),
        CertError::P12MissingCertificate => (ErrorCode::P12MissingCertificate, e.to_string()),
        CertError::P12ChainVerifyFailed(detail) => {
            (ErrorCode::P12ChainVerifyFailed, detail.clone())
        }
        CertError::P12UntrustedCa(detail) => (ErrorCode::P12UntrustedCa, detail.clone()),
        // Fallback for any other cert error
        other => (ErrorCode::ValidationFailed, other.to_string()),
    }
}
