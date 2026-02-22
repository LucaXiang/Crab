//! App update check endpoint for Tauri updater
//!
//! Reads update manifest from S3: `updates/latest.json`
//! Returns Tauri updater-compatible JSON response.

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;

use crate::state::AppState;

/// S3 manifest structure (uploaded by CI)
///
/// `mandatory_min_version`: 低于此版本的客户端强制更新。
/// 例如 v1.1 是关键修复，设为 "1.1.0"；之后发 v1.2/v1.3 时保留该值，
/// v1.0 用户仍会被强制更新，v1.1+ 用户则正常提示。
#[derive(serde::Deserialize)]
struct UpdateManifest {
    version: String,
    notes: String,
    pub_date: String,
    #[serde(default)]
    mandatory_min_version: Option<String>,
    platforms: std::collections::HashMap<String, PlatformEntry>,
}

#[derive(serde::Deserialize, serde::Serialize)]
struct PlatformEntry {
    signature: String,
    url: String,
}

/// Tauri updater response
#[derive(serde::Serialize)]
struct UpdateResponse {
    version: String,
    notes: String,
    pub_date: String,
    platforms: std::collections::HashMap<String, PlatformEntry>,
}

/// GET /api/update/:target/:arch/:current_version
///
/// Tauri updater calls this endpoint to check for updates.
/// Returns 200 with update info if newer version available, 204 if up-to-date.
pub async fn check_update(
    State(state): State<AppState>,
    Path((target, arch, current_version)): Path<(String, String, String)>,
) -> impl IntoResponse {
    // Fetch latest.json from S3
    let manifest = match state
        .s3
        .get_object()
        .bucket(&state.update_s3_bucket)
        .key("updates/latest.json")
        .send()
        .await
    {
        Ok(output) => {
            let bytes = match output.body.collect().await {
                Ok(b) => b.to_vec(),
                Err(e) => {
                    tracing::error!("Failed to read S3 body: {e}");
                    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
                }
            };
            match serde_json::from_slice::<UpdateManifest>(&bytes) {
                Ok(m) => m,
                Err(e) => {
                    tracing::error!("Failed to parse update manifest: {e}");
                    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
                }
            }
        }
        Err(e) => {
            tracing::warn!("No update manifest found in S3: {e}");
            return StatusCode::NO_CONTENT.into_response();
        }
    };

    // Compare versions — if manifest version <= current, no update
    if !is_newer(&manifest.version, &current_version) {
        return StatusCode::NO_CONTENT.into_response();
    }

    // Build platform key: e.g. "windows-x86_64"
    let platform_key = format!("{target}-{arch}");

    // Check if this platform has an update
    let Some(platform) = manifest.platforms.get(&platform_key) else {
        tracing::info!(
            platform = platform_key,
            "No update available for this platform"
        );
        return StatusCode::NO_CONTENT.into_response();
    };

    tracing::info!(
        current = current_version,
        latest = manifest.version,
        platform = platform_key,
        "Update available"
    );

    let mut platforms = std::collections::HashMap::new();
    platforms.insert(
        platform_key,
        PlatformEntry {
            signature: platform.signature.clone(),
            url: platform.url.clone(),
        },
    );

    // Client version < mandatory_min_version → forced update
    let is_mandatory = manifest
        .mandatory_min_version
        .as_deref()
        .is_some_and(|min| {
            !is_newer(&current_version, min)
                && current_version
                    .strip_prefix('v')
                    .unwrap_or(&current_version)
                    != min.strip_prefix('v').unwrap_or(min)
        });

    if is_mandatory {
        tracing::info!(
            current = %current_version,
            mandatory_min = ?manifest.mandatory_min_version,
            "Mandatory update required"
        );
    }

    let notes = if is_mandatory {
        format!("[MANDATORY]\n{}", manifest.notes)
    } else {
        manifest.notes
    };

    Json(UpdateResponse {
        version: manifest.version,
        notes,
        pub_date: manifest.pub_date,
        platforms,
    })
    .into_response()
}

/// GET /api/download/latest
///
/// Reads latest.json from S3 and redirects to the Windows installer URL.
/// Used by the portal download button.
pub async fn download_latest(State(state): State<AppState>) -> impl IntoResponse {
    let manifest = match state
        .s3
        .get_object()
        .bucket(&state.update_s3_bucket)
        .key("updates/latest.json")
        .send()
        .await
    {
        Ok(output) => {
            let bytes = match output.body.collect().await {
                Ok(b) => b.to_vec(),
                Err(e) => {
                    tracing::error!("Failed to read S3 body: {e}");
                    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
                }
            };
            match serde_json::from_slice::<UpdateManifest>(&bytes) {
                Ok(m) => m,
                Err(e) => {
                    tracing::error!("Failed to parse update manifest: {e}");
                    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
                }
            }
        }
        Err(e) => {
            tracing::warn!("No update manifest in S3: {e}");
            return StatusCode::NOT_FOUND.into_response();
        }
    };

    // Build the release download URL (not the updater URL)
    let version = &manifest.version;
    let download_url = format!(
        "{}/releases/v{}/redcoral-pos_v{}_x64-setup.exe",
        state.update_download_base_url, version, version
    );

    (
        StatusCode::FOUND,
        [(axum::http::header::LOCATION, download_url)],
    )
        .into_response()
}

/// Simple semver comparison: returns true if `latest` > `current`
fn is_newer(latest: &str, current: &str) -> bool {
    let parse = |v: &str| -> (u32, u32, u32) {
        let v = v.strip_prefix('v').unwrap_or(v);
        let parts: Vec<u32> = v.split('.').filter_map(|p| p.parse().ok()).collect();
        (
            parts.first().copied().unwrap_or(0),
            parts.get(1).copied().unwrap_or(0),
            parts.get(2).copied().unwrap_or(0),
        )
    };
    parse(latest) > parse(current)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_newer() {
        assert!(is_newer("1.2.0", "1.1.2"));
        assert!(is_newer("2.0.0", "1.9.9"));
        assert!(!is_newer("1.1.2", "1.1.2"));
        assert!(!is_newer("1.1.1", "1.1.2"));
        assert!(is_newer("v1.2.0", "1.1.2"));
    }

    /// Helper to test mandatory logic in isolation (mirrors check_update logic)
    fn is_mandatory(mandatory_min_version: Option<&str>, current: &str) -> bool {
        mandatory_min_version.is_some_and(|min| {
            !is_newer(current, min)
                && current.strip_prefix('v').unwrap_or(current)
                    != min.strip_prefix('v').unwrap_or(min)
        })
    }

    #[test]
    fn test_mandatory_min_version() {
        // v1.0 < min 1.1.0 → forced
        assert!(is_mandatory(Some("1.1.0"), "1.0.0"));
        // v1.1.0 == min 1.1.0 → NOT forced (already at min)
        assert!(!is_mandatory(Some("1.1.0"), "1.1.0"));
        // v1.2.0 > min 1.1.0 → NOT forced
        assert!(!is_mandatory(Some("1.1.0"), "1.2.0"));
        // no mandatory_min_version → NOT forced
        assert!(!is_mandatory(None, "1.0.0"));
        // v prefix handling
        assert!(is_mandatory(Some("1.1.0"), "v1.0.0"));
        assert!(!is_mandatory(Some("1.1.0"), "v1.1.0"));
    }
}
