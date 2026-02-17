//! Cloud command executor — executes commands received from crab-cloud
//!
//! Only read-only / safe operations are supported.
//! Unknown commands return an error result.

use shared::cloud::{CloudCommand, CloudCommandResult};

use crate::core::state::ServerState;

/// Execute a cloud command and return the result
pub async fn execute(state: &ServerState, cmd: &CloudCommand) -> CloudCommandResult {
    let executed_at = shared::util::now_millis();

    match cmd.command_type.as_str() {
        "get_status" => get_status(state, cmd, executed_at),
        "refresh_subscription" => refresh_subscription(state, cmd, executed_at).await,
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
