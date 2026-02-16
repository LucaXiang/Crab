use crate::db::{p12, tenants};
use crate::state::AppState;
use aws_sdk_s3::primitives::ByteStream;
use axum::Json;
use axum::extract::{Multipart, State};
use std::sync::Arc;

/// 上传 P12 证书 (multipart/form-data)
///
/// Fields:
/// - username: 租户 ID
/// - password: 租户密码
/// - p12_password: .p12 文件的解锁密码
/// - p12_file: .p12 文件
///
/// 安全措施:
/// - P12 文件在上传前先验证格式和密码正确性
/// - 验证证书由西班牙认可的 CA 签发 (FNMT, Camerfirma, ANF 等)
/// - 提取证书元数据 (指纹、税号、公司名、过期时间等) 存入 PG
/// - P12 密码存入 AWS Secrets Manager (不存 PG)
/// - P12 文件存入 S3 (SSE-KMS 加密)
pub async fn upload_p12(
    State(state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> Json<serde_json::Value> {
    let mut username = None;
    let mut password = None;
    let mut p12_password = None;
    let mut p12_data = None;

    // 解析 multipart fields
    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.name().unwrap_or_default().to_string();
        match name.as_str() {
            "username" => {
                username = field.text().await.ok();
            }
            "password" => {
                password = field.text().await.ok();
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

    let (Some(username), Some(password), Some(p12_password), Some(p12_data)) =
        (username, password, p12_password, p12_data)
    else {
        return Json(serde_json::json!({
            "success": false,
            "error": "Missing required fields: username, password, p12_password, p12_file"
        }));
    };

    // 1. Authenticate tenant
    let tenant = match tenants::authenticate(&state.db, &username, &password).await {
        Ok(Some(t)) => t,
        Ok(None) => {
            return Json(serde_json::json!({
                "success": false,
                "error": "Invalid credentials"
            }));
        }
        Err(e) => {
            tracing::error!(error = %e, "Database error during authentication");
            return Json(serde_json::json!({
                "success": false,
                "error": "Internal error"
            }));
        }
    };

    // 2. 验证 P12 文件 + 提取证书元数据 + 验证西班牙 CA 签发
    let cert_info = match crab_cert::parse_p12(&p12_data, &p12_password) {
        Ok(info) => info,
        Err(e) => {
            tracing::warn!(
                tenant_id = %tenant.id,
                error = %e,
                "P12 validation failed"
            );
            return Json(serde_json::json!({
                "success": false,
                "error": format!("Invalid P12 file: {e}")
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

    // 3. 将 P12 密码存入 Secrets Manager (先于 S3，避免密码丢失)
    if let Err(e) = state.store_p12_password(&tenant.id, &p12_password).await {
        tracing::error!(error = %e, tenant_id = %tenant.id, "Failed to store P12 password in Secrets Manager");
        return Json(serde_json::json!({
            "success": false,
            "error": "Failed to secure certificate password"
        }));
    }

    // 4. Upload to S3 with SSE-KMS
    let s3_key = format!("{}/verifactu.p12", tenant.id);

    let mut put_builder = state
        .s3
        .put_object()
        .bucket(&state.s3_bucket)
        .key(&s3_key)
        .body(ByteStream::from(p12_data.to_vec()))
        .server_side_encryption(aws_sdk_s3::types::ServerSideEncryption::AwsKms);

    if let Some(ref kms_key) = state.kms_key_id {
        put_builder = put_builder.ssekms_key_id(kms_key);
    }

    if let Err(e) = put_builder.send().await {
        tracing::error!(error = %e, tenant_id = %tenant.id, "Failed to upload .p12 to S3");
        return Json(serde_json::json!({
            "success": false,
            "error": "Failed to store certificate"
        }));
    }

    // 5. Save metadata to PG (不含密码)
    if let Err(e) = p12::upsert(&state.db, &tenant.id, &s3_key, &cert_info).await {
        tracing::error!(error = %e, "Failed to save P12 metadata");
        return Json(serde_json::json!({
            "success": false,
            "error": "Failed to save certificate metadata"
        }));
    }

    tracing::info!(
        tenant_id = %tenant.id,
        fingerprint = %cert_info.fingerprint,
        "P12 certificate uploaded and secured"
    );

    Json(serde_json::json!({
        "success": true,
        "s3_key": s3_key,
        "fingerprint": cert_info.fingerprint,
        "common_name": cert_info.common_name,
        "organization": cert_info.organization,
        "tax_id": cert_info.tax_id(),
        "issuer": cert_info.issuer,
        "expires_at": cert_info.expires_at
    }))
}
