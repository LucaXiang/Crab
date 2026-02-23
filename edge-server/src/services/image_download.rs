//! Image download from S3 presigned URL
//!
//! Used by rpc_executor when Cloud sends EnsureImage with a presigned GET URL.
//! Downloads the image file to {work_dir}/images/{hash}.jpg (idempotent).

use sha2::{Digest, Sha256};
use std::path::Path;

/// Download image from presigned URL and save to local images directory.
///
/// Idempotent: skips if file already exists.
/// Atomic: writes to tmp file then renames to prevent corrupt files on crash.
/// Verified: SHA256 hash of downloaded bytes must match expected hash.
pub async fn download_and_save(presigned_url: &str, hash: &str, images_dir: &Path) {
    let file_path = images_dir.join(format!("{hash}.jpg"));

    if file_path.exists() {
        tracing::debug!(hash = %hash, "Image already exists locally, skipping download");
        return;
    }

    // Ensure directory exists
    if let Err(e) = tokio::fs::create_dir_all(images_dir).await {
        tracing::warn!(hash = %hash, error = %e, "Failed to create images directory");
        return;
    }

    // Download from S3 (presigned URL is HTTPS, no mTLS needed)
    let client = reqwest::Client::new();
    let resp = match client.get(presigned_url).send().await {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!(hash = %hash, error = %e, "Failed to download image from presigned URL");
            return;
        }
    };

    if !resp.status().is_success() {
        tracing::warn!(
            hash = %hash,
            status = %resp.status(),
            "Image download returned non-success status"
        );
        return;
    }

    let bytes = match resp.bytes().await {
        Ok(b) => b,
        Err(e) => {
            tracing::warn!(hash = %hash, error = %e, "Failed to read image response body");
            return;
        }
    };

    // Verify SHA256 hash matches
    let actual_hash = hex::encode(Sha256::digest(&bytes));
    if actual_hash != hash {
        tracing::warn!(
            expected = %hash,
            actual = %actual_hash,
            "Image hash mismatch, discarding download"
        );
        return;
    }

    // Atomic write: tmp file + rename
    let tmp_path = images_dir.join(format!("{hash}.jpg.tmp"));
    if let Err(e) = tokio::fs::write(&tmp_path, &bytes).await {
        tracing::warn!(hash = %hash, error = %e, "Failed to write tmp image file");
        return;
    }
    if let Err(e) = tokio::fs::rename(&tmp_path, &file_path).await {
        tracing::warn!(hash = %hash, error = %e, "Failed to rename tmp image file");
        let _ = tokio::fs::remove_file(&tmp_path).await;
        return;
    }

    tracing::info!(hash = %hash, size = bytes.len(), "Image downloaded from cloud");
}
