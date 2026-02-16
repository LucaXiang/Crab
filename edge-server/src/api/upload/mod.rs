//! Upload Routes
//!
//! Provides image upload endpoints for authenticated users.

mod handler;

use axum::{
    Router,
    body::Bytes,
    extract::{Path, State},
    middleware,
    response::IntoResponse,
    routing::post,
};
use http::header;

use crate::auth::require_permission;
use crate::core::ServerState;

/// Upload file response
enum UploadFileResponse {
    Ok(Bytes),
    NotFound,
    BadRequest(&'static str),
}

impl IntoResponse for UploadFileResponse {
    fn into_response(self) -> axum::response::Response {
        match self {
            UploadFileResponse::Ok(content) => (
                http::StatusCode::OK,
                [(header::CONTENT_TYPE, "image/jpeg")],
                content,
            )
                .into_response(),
            UploadFileResponse::NotFound => {
                (http::StatusCode::NOT_FOUND, "File not found").into_response()
            }
            UploadFileResponse::BadRequest(msg) => {
                (http::StatusCode::BAD_REQUEST, msg).into_response()
            }
        }
    }
}

/// Serve uploaded file handler
async fn serve_uploaded_file(
    State(state): State<ServerState>,
    Path(filename): Path<String>,
) -> UploadFileResponse {
    // Security check: prevent path traversal
    if filename.is_empty()
        || filename.contains("..")
        || filename.contains('/')
        || filename.contains('\\')
    {
        return UploadFileResponse::BadRequest("Invalid filename");
    }

    // Images dir: {tenant}/server/images/
    let file_path = state.work_dir().join("images").join(&filename);

    // Read file
    match tokio::fs::read(&file_path).await {
        Ok(content) => UploadFileResponse::Ok(content.into()),
        Err(_) => UploadFileResponse::NotFound,
    }
}

/// Build upload router
pub fn router() -> Router<ServerState> {
    Router::new()
        // Upload image API - requires menu:manage permission
        .route("/api/image/upload", post(handler::upload))
        .layer(middleware::from_fn(require_permission("menu:manage")))
        // Serve uploaded images - any authenticated user can read
        .merge(Router::new().route(
            "/api/image/{filename}",
            axum::routing::get(serve_uploaded_file),
        ))
}
