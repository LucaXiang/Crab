//! Message bus testing routes

use axum::{
    Json, Router,
    extract::{Query, State},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};

use crate::message::BusMessage;
use crate::server::ServerState;
use shared::response::ApiResponse;

/// Request to emit a message
#[derive(Debug, Deserialize)]
pub struct EmitRequest {
    /// Message type: "notification", "broadcast", "transaction", etc.
    #[serde(default = "default_message_type")]
    pub message_type: String,
    /// Message title (for notifications)
    pub title: Option<String>,
    /// Message body
    pub body: String,
}

fn default_message_type() -> String {
    "notification".to_string()
}

/// Response from emit endpoint
#[derive(Debug, Serialize)]
pub struct EmitResponse {
    pub success: bool,
    pub message: String,
}

/// POST /api/message/emit - Emit a test message to the bus
///
/// This endpoint allows testing the message bus by emitting messages
/// that will be received by all subscribers (both in-process and TCP clients).
///
/// # Examples
///
/// ```json
/// {
///   "message_type": "notification",
///   "title": "Test",
///   "body": "Hello from message bus!"
/// }
/// ```
async fn emit_message(
    State(state): State<ServerState>,
    Json(req): Json<EmitRequest>,
) -> Json<ApiResponse<EmitResponse>> {
    let bus = state.get_message_bus();

    // Create message based on type
    let msg = match req.message_type.as_str() {
        "notification" => {
            let title = req.title.unwrap_or_else(|| "Notification".to_string());
            BusMessage::notification(&title, &req.body)
        }
        "intent" | "order_intent" => {
            BusMessage::order_intent(&crate::message::OrderIntentPayload {
                action: "add_dish".to_string(),
                table_id: "T01".to_string(),
                order_id: None,
                data: serde_json::json!({
                    "dishes": [{"id": "D001", "name": req.body}]
                }),
                operator: None,
            })
        }
        "sync" | "order_sync" => BusMessage::order_sync(&crate::message::OrderSyncPayload {
            action: "dish_added".to_string(),
            table_id: "T01".to_string(),
            order_id: Some("ORD123".to_string()),
            status: "updated".to_string(),
            source: "server".to_string(),
            data: Some(serde_json::json!({
                "message": req.body
            })),
        }),
        "data" | "data_sync" => BusMessage::data_sync(
            "dish_price",
            serde_json::json!({
                "dish_id": "D001",
                "data": req.body
            }),
        ),
        "server" | "server_command" | "command" => BusMessage::server_command(
            "config_update",
            serde_json::json!({
                "command": req.body
            }),
        ),
        _ => BusMessage::notification("Unknown", &req.body),
    };

    // Publish to bus
    match bus.publish(msg).await {
        Ok(_) => {
            tracing::info!("Message emitted: {} - {}", req.message_type, req.body);
            Json(ApiResponse::ok(EmitResponse {
                success: true,
                message: format!("Message emitted successfully: {}", req.message_type),
            }))
        }
        Err(e) => {
            tracing::error!("Failed to emit message: {}", e);
            Json(ApiResponse::error(
                "EMIT_FAILED",
                format!("Failed to emit message: {}", e),
            ))
        }
    }
}

/// GET /api/message/emit - Emit a test message using query parameters
///
/// Simpler version using query parameters for quick testing.
///
/// # Examples
///
/// - `/api/message/emit?body=Hello`
/// - `/api/message/emit?type=notification&title=Test&body=Hello`
/// - `/api/message/emit?type=broadcast&body=Hello%20World`
async fn emit_message_get(
    State(state): State<ServerState>,
    Query(params): Query<EmitQueryParams>,
) -> Json<ApiResponse<EmitResponse>> {
    let req = EmitRequest {
        message_type: params.r#type.unwrap_or_else(default_message_type),
        title: params.title,
        body: params.body,
    };

    let bus = state.get_message_bus();
    bus.memory_transport();
    // Create message based on type
    let msg = match req.message_type.as_str() {
        "notification" => {
            let title = req.title.unwrap_or_else(|| "Notification".to_string());
            BusMessage::notification(&title, &req.body)
        }
        "intent" | "order_intent" => {
            BusMessage::order_intent(&crate::message::OrderIntentPayload {
                action: "add_dish".to_string(),
                table_id: "T01".to_string(),
                order_id: None,
                data: serde_json::json!({
                    "dishes": [{"id": "D001", "name": req.body}]
                }),
                operator: None,
            })
        }
        "sync" | "order_sync" => BusMessage::order_sync(&crate::message::OrderSyncPayload {
            action: "dish_added".to_string(),
            table_id: "T01".to_string(),
            order_id: Some("ORD123".to_string()),
            status: "updated".to_string(),
            source: "server".to_string(),
            data: Some(serde_json::json!({
                "message": req.body
            })),
        }),
        "data" | "data_sync" => BusMessage::data_sync(
            "dish_price",
            serde_json::json!({
                "dish_id": "D001",
                "data": req.body
            }),
        ),
        "server" | "server_command" | "command" => BusMessage::server_command(
            "config_update",
            serde_json::json!({
                "command": req.body
            }),
        ),
        _ => BusMessage::notification("Unknown", &req.body),
    };

    // Publish to bus
    match bus.publish(msg).await {
        Ok(_) => {
            tracing::info!("Message emitted: {} - {}", req.message_type, req.body);
            Json(ApiResponse::ok(EmitResponse {
                success: true,
                message: format!("Message emitted successfully: {}", req.message_type),
            }))
        }
        Err(e) => {
            tracing::error!("Failed to emit message: {}", e);
            Json(ApiResponse::error(
                "EMIT_FAILED",
                format!("Failed to emit message: {}", e),
            ))
        }
    }
}

/// Query parameters for emit endpoint
#[derive(Debug, Deserialize)]
pub struct EmitQueryParams {
    /// Message type: "notification", "broadcast", "transaction"
    #[serde(rename = "type")]
    pub r#type: Option<String>,
    /// Message title (for notifications)
    pub title: Option<String>,
    /// Message body (required)
    pub body: String,
}

/// Build message routes
pub fn router() -> Router<ServerState> {
    Router::new()
        .route("/api/message/emit", post(emit_message))
        .route("/api/message/emit", get(emit_message_get))
}
