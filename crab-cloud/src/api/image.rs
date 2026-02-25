//! Image upload API — Console uploads product images to S3
//!
//! POST /api/tenant/images — multipart upload → validate → JPEG compress → SHA256 → S3
//!
//! Images stored at: s3://{bucket}/images/{tenant_id}/{hash}.jpg
//! Returns: { hash } for use in ProductCreate/Update.image field

use axum::{Extension, Json, extract::Multipart, extract::State};
use image::codecs::jpeg::JpegEncoder;
use sha2::{Digest, Sha256};
use std::io::Cursor;

use shared::error::{AppError, ErrorCode};

use crate::auth::tenant_auth::TenantIdentity;
use crate::state::AppState;

/// Maximum file size (20MB)
const MAX_FILE_SIZE: usize = 20 * 1024 * 1024;

/// Maximum images per tenant (across all stores)
const MAX_IMAGES_PER_TENANT: i32 = 5000;

/// JPEG quality (matches edge-server)
const JPEG_QUALITY: u8 = 85;

/// Supported image formats
const SUPPORTED_FORMATS: &[&str] = &["png", "jpg", "jpeg", "webp"];

/// S3 key prefix for images
fn s3_image_key(tenant_id: &str, hash: &str) -> String {
    format!("images/{tenant_id}/{hash}.jpg")
}

/// Upload response
#[derive(serde::Serialize)]
pub struct ImageUploadResponse {
    pub hash: String,
}

/// POST /api/tenant/images — upload product image
pub async fn upload_image(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    mut multipart: Multipart,
) -> Result<Json<ImageUploadResponse>, AppError> {
    // Extract file from multipart
    let mut file_data: Option<Vec<u8>> = None;
    let mut original_filename: Option<String> = None;

    while let Some(field) = multipart.next_field().await.map_err(|e| {
        AppError::with_message(ErrorCode::InvalidRequest, format!("Multipart error: {e}"))
    })? {
        let name = field.name().map(|s| s.to_string());
        if name.as_deref() == Some("file") || name.as_deref() == Some("") {
            original_filename = field.file_name().map(|s| s.to_string());
            file_data = Some(
                field
                    .bytes()
                    .await
                    .map_err(|e| {
                        AppError::with_message(
                            ErrorCode::InvalidRequest,
                            format!("Read error: {e}"),
                        )
                    })?
                    .to_vec(),
            );
            break;
        }
    }

    let data = file_data
        .ok_or_else(|| AppError::with_message(ErrorCode::InvalidRequest, "No file provided"))?;

    if data.is_empty() {
        return Err(AppError::with_message(
            ErrorCode::InvalidRequest,
            "Empty file",
        ));
    }

    if data.len() > MAX_FILE_SIZE {
        return Err(AppError::with_message(
            ErrorCode::InvalidRequest,
            format!(
                "File too large: {} bytes (max {})",
                data.len(),
                MAX_FILE_SIZE
            ),
        ));
    }

    // Validate file extension
    let filename = original_filename.unwrap_or_default();
    let ext = std::path::Path::new(&filename)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .unwrap_or_default();

    if !SUPPORTED_FORMATS.contains(&ext.as_str()) {
        return Err(AppError::with_message(
            ErrorCode::InvalidRequest,
            format!("Unsupported format: {ext}. Supported: png, jpg, jpeg, webp"),
        ));
    }

    // Check tenant image count limit
    let prefix = format!("images/{}/", identity.tenant_id);
    let list_result = state
        .s3
        .client
        .list_objects_v2()
        .bucket(&state.s3.bucket)
        .prefix(&prefix)
        .max_keys(MAX_IMAGES_PER_TENANT)
        .send()
        .await;
    if let Ok(output) = list_result {
        let count = output.key_count().unwrap_or(0);
        if count >= MAX_IMAGES_PER_TENANT {
            return Err(AppError::with_message(
                ErrorCode::ResourceLimitExceeded,
                format!("Image limit reached ({count}/{MAX_IMAGES_PER_TENANT})"),
            ));
        }
    }

    // Load and validate image content
    let img = image::load_from_memory(&data).map_err(|e| {
        AppError::with_message(ErrorCode::InvalidRequest, format!("Invalid image: {e}"))
    })?;

    // Compress to JPEG
    let mut buffer = Vec::new();
    {
        let mut cursor = Cursor::new(&mut buffer);
        let rgb_img = img.to_rgb8();
        let encoder = JpegEncoder::new_with_quality(&mut cursor, JPEG_QUALITY);
        rgb_img.write_with_encoder(encoder).map_err(|e| {
            AppError::with_message(
                ErrorCode::InternalError,
                format!("Image compression failed: {e}"),
            )
        })?;
    }

    // SHA256 hash
    let mut hasher = Sha256::new();
    hasher.update(&buffer);
    let hash = hex::encode(hasher.finalize());

    // Upload to S3 (idempotent — same hash = same content)
    let key = s3_image_key(&identity.tenant_id, &hash);

    state
        .s3
        .client
        .put_object()
        .bucket(&state.s3.bucket)
        .key(&key)
        .body(buffer.into())
        .content_type("image/jpeg")
        .send()
        .await
        .map_err(|e| {
            tracing::error!(hash = %hash, error = %e, "S3 upload failed");
            AppError::with_message(ErrorCode::InternalError, "Image upload failed")
        })?;

    tracing::info!(
        tenant_id = %identity.tenant_id,
        hash = %hash,
        "Product image uploaded to S3"
    );

    Ok(Json(ImageUploadResponse { hash }))
}

/// GET /api/tenant/images/:hash — get presigned S3 URL for an image
pub async fn get_image_url(
    State(state): State<AppState>,
    Extension(identity): Extension<TenantIdentity>,
    axum::extract::Path(hash): axum::extract::Path<String>,
) -> Result<Json<ImageUrlResponse>, AppError> {
    // Validate hash is exactly 64 hex characters (SHA256) to prevent path traversal
    if hash.len() != 64 || !hash.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(AppError::with_message(
            ErrorCode::InvalidRequest,
            "Invalid image hash",
        ));
    }
    let url = presigned_get_url(&state, &identity.tenant_id, &hash).await?;
    Ok(Json(ImageUrlResponse { url }))
}

/// Get image URL response
#[derive(serde::Serialize)]
pub struct ImageUrlResponse {
    pub url: String,
}

/// Generate a presigned GET URL for an image in S3
pub async fn presigned_get_url(
    state: &AppState,
    tenant_id: &str,
    hash: &str,
) -> Result<String, AppError> {
    use aws_sdk_s3::presigning::PresigningConfig;
    use std::time::Duration;

    let key = s3_image_key(tenant_id, hash);
    let presigning = PresigningConfig::expires_in(Duration::from_secs(3600)).map_err(|e| {
        tracing::error!(error = %e, "Failed to create presigning config");
        AppError::new(ErrorCode::InternalError)
    })?;

    let presigned = state
        .s3
        .client
        .get_object()
        .bucket(&state.s3.bucket)
        .key(&key)
        .presigned(presigning)
        .await
        .map_err(|e| {
            tracing::error!(hash = %hash, error = %e, "Failed to generate presigned URL");
            AppError::new(ErrorCode::InternalError)
        })?;

    Ok(presigned.uri().to_string())
}
