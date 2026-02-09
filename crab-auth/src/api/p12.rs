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

    // 2. Upload to S3 with SSE-KMS
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

    // 3. Save metadata to PG
    if let Err(e) = p12::upsert(
        &state.db,
        &tenant.id,
        &s3_key,
        &p12_password,
        None,
        None,
        None,
    )
    .await
    {
        tracing::error!(error = %e, "Failed to save P12 metadata");
        return Json(serde_json::json!({
            "success": false,
            "error": "Failed to save certificate metadata"
        }));
    }

    tracing::info!(tenant_id = %tenant.id, "P12 certificate uploaded");

    Json(serde_json::json!({
        "success": true,
        "s3_key": s3_key
    }))
}
