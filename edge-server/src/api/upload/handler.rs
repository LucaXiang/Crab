//! Image Upload Handler
//!
//! Handles image uploads from authenticated users.
//! Supports multiple image formats (PNG, JPEG, WebP) and converts to JPG.

use axum::Json;
use axum::extract::{Extension, Multipart, State};
use image::DynamicImage;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::{fs, io::Cursor};
use uuid::Uuid;

use crate::audit_log;
use crate::utils::AppResponse;
use crate::{AppError, CurrentUser, ServerState};

/// Maximum file size (5MB)
const MAX_FILE_SIZE: usize = 5 * 1024 * 1024;

/// Supported image formats
const SUPPORTED_FORMATS: &[&str] = &["png", "jpg", "jpeg", "webp"];

/// Upload response
#[derive(Debug, Serialize)]
pub struct UploadResponse {
    pub file_id: String,
    pub filename: String,
    pub original_name: String,
    pub size: usize,
    pub format: String,
    pub url: String,
}

/// Calculate SHA256 hash of data
fn calculate_hash(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

/// Find existing file by content hash
fn find_file_by_hash(images_dir: &Path, hash: &str) -> Option<String> {
    let hash_dir = images_dir.join("by_hash");
    if !hash_dir.exists() {
        return None;
    }

    // Hash directory uses first 2 chars as subdir (e.g., "ab/abc123...")
    let prefix = &hash[..2];
    let hash_path = hash_dir.join(format!("{}/{}", prefix, hash));

    if hash_path.exists() {
        // Read the symlink to get original filename
        if let Ok(target) = fs::read_link(&hash_path) {
            return target.file_name().map(|s| s.to_string_lossy().to_string());
        }
    }
    None
}

/// Create hash-based symlink for deduplication
fn create_hash_symlink(images_dir: &Path, hash: &str, filename: &str) -> Result<(), AppError> {
    let hash_dir = images_dir.join("by_hash");
    fs::create_dir_all(&hash_dir)
        .map_err(|e| AppError::internal(format!("Failed to create hash dir: {}", e)))?;

    let prefix = &hash[..2];
    let hash_subdir = hash_dir.join(prefix);
    fs::create_dir_all(&hash_subdir)
        .map_err(|e| AppError::internal(format!("Failed to create hash subdir: {}", e)))?;

    let hash_path = hash_subdir.join(hash);
    let target_path = PathBuf::from("../").join(filename);

    symlink::symlink_auto(&target_path, &hash_path)
        .map_err(|e| AppError::internal(format!("Failed to create symlink: {}", e)))?;

    Ok(())
}

/// JPEG quality for dish images (85% - maintains color appeal while controlling file size)
const JPEG_QUALITY: u8 = 85;

/// Process and compress image
fn process_and_compress_image(
    data: Vec<u8>,
    _original_ext: String,
) -> Result<(DynamicImage, Vec<u8>), AppError> {
    // Load image from bytes
    let img = image::load_from_memory(&data)
        .map_err(|e| AppError::validation(format!("Invalid image: {}", e)))?;

    // Save to buffer as JPG with quality setting
    let mut buffer = Vec::new();
    {
        let mut cursor = Cursor::new(&mut buffer);
        let rgb_img = img.to_rgb8();
        let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut cursor, JPEG_QUALITY);
        rgb_img
            .write_with_encoder(encoder)
            .map_err(|e| AppError::internal(format!("Failed to compress image: {}", e)))?;
    }

    Ok((img, buffer))
}

/// Validate image file
fn validate_image(data: &[u8], ext: &str) -> Result<(), AppError> {
    // Check file size
    if data.len() > MAX_FILE_SIZE {
        return Err(AppError::validation(format!(
            "File too large. Maximum size is {} bytes ({}MB)",
            MAX_FILE_SIZE,
            MAX_FILE_SIZE / 1024 / 1024
        )));
    }

    // Check file extension
    let ext_lower = ext.to_lowercase();
    if !SUPPORTED_FORMATS.contains(&ext_lower.as_str()) {
        return Err(AppError::validation(format!(
            "Unsupported file format '{}'. Supported: {}",
            ext_lower,
            SUPPORTED_FORMATS.join(", ")
        )));
    }

    // Verify it's actually an image by trying to load it
    if let Err(e) = image::load_from_memory(data) {
        return Err(AppError::validation(format!(
            "Invalid image file ({}): {}",
            ext_lower, e
        )));
    }

    Ok(())
}

/// Upload image handler
pub async fn upload(
    State(state): State<ServerState>,
    Extension(_current_user): Extension<CurrentUser>,
    mut multipart: Multipart,
) -> Result<Json<AppResponse<UploadResponse>>, AppError> {
    // Use work_dir from state (relative path since we change to it at startup)
    let work_dir = state.work_dir().clone();
    let images_dir = work_dir.join("uploads/images");
    fs::create_dir_all(&images_dir)
        .map_err(|e| AppError::internal(format!("Failed to create images directory: {}", e)))?;

    // Find the file field
    let mut field_data: Option<Vec<u8>> = None;
    let mut original_filename = None;

    while let Some(f) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::validation(format!("Invalid multipart request: {}", e)))?
    {
        let name = f.name().map(|s| s.to_string());
        if name.as_deref() == Some("file") || name.as_deref() == Some("") {
            original_filename = f.file_name().map(|s| s.to_string());
            field_data = Some(
                f.bytes()
                    .await
                    .map_err(|e| AppError::validation(format!("Multipart error: {}", e)))?
                    .to_vec(),
            );
            break;
        }
    }

    let data = field_data.ok_or_else(|| {
        AppError::validation("No 'file' field found. Field name must be 'file'".to_string())
    })?;

    let filename = original_filename
        .ok_or_else(|| AppError::validation("No filename provided in file field".to_string()))?;

    // Check if data is empty
    if data.is_empty() {
        return Err(AppError::validation("Empty file provided".to_string()));
    }

    // Extract file extension
    let ext = PathBuf::from(&filename)
        .extension()
        .and_then(|ext| ext.to_str().map(|s| s.to_string()))
        .ok_or_else(|| AppError::validation(format!("Invalid file extension for: {}", filename)))?;

    // Validate image
    validate_image(&data, &ext)?;

    // Process and compress image
    let (_original_img, compressed_data) = process_and_compress_image(data, ext)?;

    // Calculate hash for deduplication
    let file_hash = calculate_hash(&compressed_data);

    // Check if file already exists by hash
    if let Some(existing_filename) = find_file_by_hash(&images_dir, &file_hash) {
        tracing::info!(
            original_name = %filename,
            existing_file = %existing_filename,
            "Duplicate image detected, returning existing file"
        );

        let file_id = existing_filename
            .strip_suffix(".jpg")
            .map(|s| s.to_string())
            .unwrap_or_else(|| Uuid::new_v4().to_string());

        let url = format!("/api/image/{}", existing_filename);
        let response = UploadResponse {
            file_id,
            filename: existing_filename,
            original_name: filename,
            size: compressed_data.len(),
            format: "jpg".to_string(),
            url,
        };

        return Ok(crate::ok!(response));
    }

    // Generate unique filename for new file
    let file_id = Uuid::new_v4().to_string();
    let new_filename = format!("{}.jpg", file_id);
    let file_path = images_dir.join(&new_filename);

    // Save compressed image
    fs::write(&file_path, &compressed_data)
        .map_err(|e| AppError::internal(format!("Failed to save file: {}", e)))?;

    // Create hash-based symlink for deduplication
    create_hash_symlink(&images_dir, &file_hash, &new_filename)?;

    // Log audit event
    audit_log!(
        "system",
        "upload",
        &file_id,
        format!("Uploaded image: {} -> {}", filename, new_filename)
    );

    tracing::info!(
        original_name = %filename,
        size = %compressed_data.len(),
        hash = %file_hash,
        "Image uploaded successfully"
    );

    let url = format!("/api/image/{}", new_filename);
    let response = UploadResponse {
        file_id,
        filename: new_filename,
        original_name: filename,
        size: compressed_data.len(),
        format: "jpg".to_string(),
        url,
    };

    Ok(crate::ok!(response))
}
