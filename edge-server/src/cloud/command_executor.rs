//! Cloud command executor — executes commands received from crab-cloud
//!
//! Only read-only / safe operations are supported.
//! Unknown commands return an error result.

use shared::cloud::{CloudCommand, CloudCommandResult};

use crate::core::state::ServerState;
use crate::db::repository::order;

/// Execute a cloud command and return the result
pub async fn execute(state: &ServerState, cmd: &CloudCommand) -> CloudCommandResult {
    let executed_at = shared::util::now_millis();

    match cmd.command_type.as_str() {
        "get_status" => get_status(state, cmd, executed_at),
        "refresh_subscription" => refresh_subscription(state, cmd, executed_at).await,
        "get_order_detail" => get_order_detail(state, cmd, executed_at).await,
        _ => CloudCommandResult {
            command_id: cmd.id.clone(),
            success: false,
            data: None,
            error: Some(format!("Unknown command type: {}", cmd.command_type)),
            executed_at,
        },
    }
}

/// Return server status summary (read-only)
fn get_status(state: &ServerState, cmd: &CloudCommand, executed_at: i64) -> CloudCommandResult {
    let active_orders = state
        .orders_manager
        .get_active_orders()
        .map(|o| o.len())
        .unwrap_or(0);

    let products = state.catalog_service.list_products().len();
    let categories = state.catalog_service.list_categories().len();

    CloudCommandResult {
        command_id: cmd.id.clone(),
        success: true,
        data: Some(serde_json::json!({
            "active_orders": active_orders,
            "products": products,
            "categories": categories,
            "epoch": state.epoch,
        })),
        error: None,
        executed_at,
    }
}

/// Fetch order detail by order_key or order_pk (on-demand from cloud)
async fn get_order_detail(
    state: &ServerState,
    cmd: &CloudCommand,
    executed_at: i64,
) -> CloudCommandResult {
    // Accept either order_key (UUID string) or order_id (i64 pk)
    let order_pk: Option<i64> =
        if let Some(pk) = cmd.payload.get("order_id").and_then(|v| v.as_i64()) {
            Some(pk)
        } else if let Some(key) = cmd.payload.get("order_key").and_then(|v| v.as_str()) {
            match sqlx::query_scalar::<_, i64>(
                "SELECT id FROM archived_order WHERE order_key = ? LIMIT 1",
            )
            .bind(key)
            .fetch_optional(&state.pool)
            .await
            {
                Ok(pk) => pk,
                Err(e) => {
                    return CloudCommandResult {
                        command_id: cmd.id.clone(),
                        success: false,
                        data: None,
                        error: Some(format!("DB query failed: {e}")),
                        executed_at,
                    };
                }
            }
        } else {
            None
        };

    let Some(pk) = order_pk else {
        return CloudCommandResult {
            command_id: cmd.id.clone(),
            success: false,
            data: None,
            error: Some("Missing order_id or order_key in payload".to_string()),
            executed_at,
        };
    };

    match order::build_order_detail_sync(&state.pool, pk).await {
        Ok(detail_sync) => CloudCommandResult {
            command_id: cmd.id.clone(),
            success: true,
            data: serde_json::to_value(&detail_sync).ok(),
            error: None,
            executed_at,
        },
        Err(e) => CloudCommandResult {
            command_id: cmd.id.clone(),
            success: false,
            data: None,
            error: Some(format!("Order not found or query failed: {e}")),
            executed_at,
        },
    }
}

/// Trigger subscription refresh (read from auth server)
async fn refresh_subscription(
    state: &ServerState,
    cmd: &CloudCommand,
    executed_at: i64,
) -> CloudCommandResult {
    state.activation.sync_subscription().await;

    CloudCommandResult {
        command_id: cmd.id.clone(),
        success: true,
        data: Some(serde_json::json!({ "message": "Subscription refresh triggered" })),
        error: None,
        executed_at,
    }
}
