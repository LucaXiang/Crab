//! Message bus testing routes

use axum::{
    Json, Router,
    extract::{Query, State},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};

use crate::message::BusMessage;
use crate::server::ServerState;
use shared::message::*;
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
            let payload = NotificationPayload::info(title, req.body);
            BusMessage::notification(&payload)
        }
        "intent" | "order_intent" => {
            let payload = OrderIntentPayload::add_dish(
                TableId::new_unchecked("T01"),
                vec![DishItem::simple("D001", 1)],
                Some(OperatorId::new("test_api")),
            );
            BusMessage::order_intent(&payload)
        }
        "sync" | "order_sync" => {
            let payload = OrderSyncPayload {
                action: OrderAction::AddDish {
                    dishes: vec![DishItem::simple("D001", 1)],
                },
                table_id: TableId::new_unchecked("T01"),
                order_id: Some(OrderId::new_unchecked("ORD123")),
                status: OrderStatus::Confirmed,
                source: OperatorId::new("server"),
                data: None,
            };
            BusMessage::order_sync(&payload)
        }
        "data" | "data_sync" => {
            let payload = DataSyncPayload::DishPrice {
                dish_id: DishId::new("D001"),
                old_price: 3800,
                new_price: 4200,
            };
            BusMessage::data_sync(&payload)
        }
        "server" | "server_command" | "command" => {
            let payload = ServerCommandPayload {
                command: ServerCommand::ConfigUpdate {
                    key: "test.key".to_string(),
                    value: serde_json::json!(req.body),
                },
            };
            BusMessage::server_command(&payload)
        }
        _ => {
            let payload = NotificationPayload::info("Unknown", req.body);
            BusMessage::notification(&payload)
        }
    };

    // Publish to bus
    let message_type = req.message_type.clone();
    match bus.publish(msg).await {
        Ok(_) => {
            tracing::info!("Message emitted: {}", message_type);
            Json(ApiResponse::ok(EmitResponse {
                success: true,
                message: format!("Message emitted successfully: {}", message_type),
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
            let payload = NotificationPayload::info(title, req.body);
            BusMessage::notification(&payload)
        }
        "intent" | "order_intent" => {
            let payload = OrderIntentPayload::add_dish(
                TableId::new_unchecked("T01"),
                vec![DishItem::simple("D001", 1)],
                Some(OperatorId::new("test_api")),
            );
            BusMessage::order_intent(&payload)
        }
        "sync" | "order_sync" => {
            let payload = OrderSyncPayload {
                action: OrderAction::AddDish {
                    dishes: vec![DishItem::simple("D001", 1)],
                },
                table_id: TableId::new_unchecked("T01"),
                order_id: Some(OrderId::new_unchecked("ORD123")),
                status: OrderStatus::Confirmed,
                source: OperatorId::new("server"),
                data: None,
            };
            BusMessage::order_sync(&payload)
        }
        "data" | "data_sync" => {
            let payload = DataSyncPayload::DishPrice {
                dish_id: DishId::new("D001"),
                old_price: 3800,
                new_price: 4200,
            };
            BusMessage::data_sync(&payload)
        }
        "server" | "server_command" | "command" => {
            let payload = ServerCommandPayload {
                command: if req.body == "activate" || req.body == "ping" {
                    ServerCommand::Ping
                } else {
                    ServerCommand::ConfigUpdate {
                        key: "test.key".to_string(),
                        value: serde_json::json!(req.body),
                    }
                },
            };
            BusMessage::server_command(&payload)
        }
        _ => {
            let payload = NotificationPayload::info("Unknown", req.body);
            BusMessage::notification(&payload)
        }
    };

    // Publish to bus
    let message_type = req.message_type.clone();
    match bus.publish(msg).await {
        Ok(_) => {
            tracing::info!("Message emitted: {}", message_type);
            Json(ApiResponse::ok(EmitResponse {
                success: true,
                message: format!("Message emitted successfully: {}", message_type),
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
