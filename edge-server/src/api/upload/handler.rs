//! Image Upload Handler
//!
//! Handles image uploads from authenticated users.
//! Supports multiple image formats (PNG, JPEG, WebP) and converts to JPG.
//!
//! Uses content hash (SHA256) as filename for natural deduplication.
//! Same content = same hash = same file (no duplicates).

use axum::Json;
use axum::extract::{Extension, Multipart, State};
use image::DynamicImage;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use std::{fs, io::Cursor};

use crate::{AppError, CurrentUser, ServerState};
use shared::ErrorCode;

/// Maximum file size (20MB)
const MAX_FILE_SIZE: usize = 20 * 1024 * 1024;

/// Supported image formats
const SUPPORTED_FORMATS: &[&str] = &["png", "jpg", "jpeg", "webp"];

/// Upload response
#[derive(Debug, Serialize)]
pub struct UploadResponse {
    /// Content hash (SHA256) - use this as the image identifier
    pub hash: String,
    /// Filename on disk ({hash}.jpg)
    pub filename: String,
    /// Original filename from upload
    pub original_name: String,
    /// File size in bytes
    pub size: usize,
    /// Output format (always "jpg")
    pub format: String,
    /// URL to access the image
    pub url: String,
}

/// Calculate SHA256 hash of data
fn calculate_hash(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

/// JPEG quality for dish images (85% - maintains color appeal while controlling file size)
const JPEG_QUALITY: u8 = 85;

/// Process and compress image
fn process_and_compress_image(
    data: Vec<u8>,
    _original_ext: String,
) -> Result<(DynamicImage, Vec<u8>), AppError> {
    // Load image from bytes
    let img = image::load_from_memory(&data).map_err(|e| {
        AppError::with_message(ErrorCode::ValidationFailed, format!("Invalid image: {}", e))
    })?;

    // Save to buffer as JPG with quality setting
    let mut buffer = Vec::new();
    {
        let mut cursor = Cursor::new(&mut buffer);
        let rgb_img = img.to_rgb8();
        let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut cursor, JPEG_QUALITY);
        rgb_img.write_with_encoder(encoder).map_err(|e| {
            AppError::with_message(
                ErrorCode::InternalError,
                format!("Image processing failed: {}", e),
            )
        })?;
    }

    Ok((img, buffer))
}

/// Validate image file
fn validate_image(data: &[u8], ext: &str) -> Result<(), AppError> {
    // Check file size
    if data.len() > MAX_FILE_SIZE {
        return Err(
            AppError::with_message(ErrorCode::ValidationFailed, "File too large")
                .with_detail("max_bytes", MAX_FILE_SIZE as i64)
                .with_detail("max_mb", (MAX_FILE_SIZE / 1024 / 1024) as i64)
                .with_detail("actual_bytes", data.len() as i64),
        );
    }

    // Check file extension
    let ext_lower = ext.to_lowercase();
    if !SUPPORTED_FORMATS.contains(&ext_lower.as_str()) {
        return Err(AppError::with_message(
            ErrorCode::ValidationFailed,
            format!("Unsupported file format: {}", ext_lower),
        )
        .with_detail("format", ext_lower)
        .with_detail("supported", SUPPORTED_FORMATS.join(", ")));
    }

    // Verify it's actually an image by trying to load it
    if let Err(e) = image::load_from_memory(data) {
        return Err(AppError::with_message(
            ErrorCode::ValidationFailed,
            format!("Invalid image file: {}", e),
        ));
    }

    Ok(())
}

/// Upload image handler
///
/// Uses content hash as filename for natural deduplication:
/// - Same content → same hash → same file (no duplicates)
/// - Database stores the hash, not the full path
pub async fn upload(
    State(state): State<ServerState>,
    Extension(current_user): Extension<CurrentUser>,
    mut multipart: Multipart,
) -> Result<Json<UploadResponse>, AppError> {
    // Images dir: {tenant}/server/images/
    let work_dir = state.work_dir().clone();
    let images_dir = work_dir.join("images");
    fs::create_dir_all(&images_dir).map_err(|e| {
        AppError::with_message(ErrorCode::InternalError, format!("create directory: {}", e))
    })?;

    // Find the file field
    let mut field_data: Option<Vec<u8>> = None;
    let mut original_filename = None;

    while let Some(f) = multipart.next_field().await.map_err(|e| {
        AppError::new(ErrorCode::InvalidRequest)
            .with_detail("reason", format!("Invalid multipart request: {}", e))
    })? {
        let name = f.name().map(|s| s.to_string());
        if name.as_deref() == Some("file") || name.as_deref() == Some("") {
            original_filename = f.file_name().map(|s| s.to_string());
            field_data = Some(
                f.bytes()
                    .await
                    .map_err(|e| {
                        AppError::new(ErrorCode::InvalidRequest)
                            .with_detail("reason", format!("Multipart error: {}", e))
                    })?
                    .to_vec(),
            );
            break;
        }
    }

    let data = field_data.ok_or_else(|| {
        AppError::new(ErrorCode::ValidationFailed).with_detail("reason", "No file provided")
    })?;

    let original_name = original_filename.ok_or_else(|| {
        AppError::new(ErrorCode::ValidationFailed).with_detail("reason", "No filename provided")
    })?;

    // Check if data is empty
    if data.is_empty() {
        return Err(AppError::new(ErrorCode::ValidationFailed).with_detail("reason", "Empty file"));
    }

    // Extract file extension
    let ext = PathBuf::from(&original_name)
        .extension()
        .and_then(|ext| ext.to_str().map(|s| s.to_string()))
        .ok_or_else(|| {
            AppError::with_message(ErrorCode::ValidationFailed, "Invalid file extension")
                .with_detail("filename", original_name.clone())
        })?;

    // Validate image
    validate_image(&data, &ext)?;

    // Process and compress image
    let (_original_img, compressed_data) = process_and_compress_image(data, ext)?;

    // Calculate hash as filename (content-addressable storage)
    let hash = calculate_hash(&compressed_data);
    let filename = format!("{}.jpg", hash);
    let file_path = images_dir.join(&filename);

    // Check if file already exists (natural deduplication)
    if file_path.exists() {
        tracing::debug!(
            original_name = %original_name,
            hash = %hash,
            "Duplicate image detected, returning existing file"
        );

        let response = UploadResponse {
            hash: hash.clone(),
            filename,
            original_name,
            size: compressed_data.len(),
            format: "jpg".to_string(),
            url: format!("/api/image/{}.jpg", hash),
        };

        return Ok(Json(response));
    }

    // Save compressed image with hash as filename
    fs::write(&file_path, &compressed_data).map_err(|e| {
        AppError::with_message(ErrorCode::InternalError, format!("write file: {}", e))
    })?;

    // Log audit event
    state
        .audit_service
        .log(
            crate::audit::AuditAction::StoreInfoChanged,
            "upload",
            hash.clone(),
            Some(current_user.id),
            Some(current_user.name.clone()),
            serde_json::json!({
                "original_name": original_name,
                "filename": filename,
                "size": compressed_data.len(),
            }),
        )
        .await;

    tracing::info!(
        original_name = %original_name,
        size = %compressed_data.len(),
        hash = %hash,
        "Image uploaded successfully"
    );

    let response = UploadResponse {
        hash: hash.clone(),
        filename,
        original_name,
        size: compressed_data.len(),
        format: "jpg".to_string(),
        url: format!("/api/image/{}.jpg", hash),
    };

    Ok(Json(response))
}
