//! Cloud RPC executor — dispatches CloudRpc to domain-specific handlers
//!
//! Handles all RPC types: GetStatus, GetOrderDetail, RefreshSubscription, StoreOp.

use shared::cloud::store_op::{StoreOp, StoreOpResult};
use shared::cloud::{CloudRpc, CloudRpcResult};

use crate::core::state::ServerState;
use crate::db::repository::order;

use super::ops::{attribute, catalog, provisioning, resource};

/// Execute a CloudRpc and return the result
pub async fn execute_rpc(state: &ServerState, rpc: &CloudRpc) -> CloudRpcResult {
    match rpc {
        CloudRpc::GetStatus => {
            let active_orders = state
                .orders_manager
                .get_active_orders()
                .map(|o| o.len())
                .unwrap_or(0);
            let products = state.catalog_service.list_products().len();
            let categories = state.catalog_service.list_categories().len();

            CloudRpcResult::Json {
                success: true,
                data: Some(serde_json::json!({
                    "active_orders": active_orders,
                    "products": products,
                    "categories": categories,
                    "epoch": state.epoch,
                })),
                error: None,
            }
        }
        CloudRpc::GetOrderDetail { order_key } => {
            // Resolve order_key → pk
            let order_pk = match sqlx::query_scalar::<_, i64>(
                "SELECT id FROM archived_order WHERE order_key = ? LIMIT 1",
            )
            .bind(order_key)
            .fetch_optional(&state.pool)
            .await
            {
                Ok(Some(pk)) => pk,
                Ok(None) => {
                    return CloudRpcResult::Json {
                        success: false,
                        data: None,
                        error: Some(format!("Order not found: {order_key}")),
                    };
                }
                Err(e) => {
                    return CloudRpcResult::Json {
                        success: false,
                        data: None,
                        error: Some(format!("DB query failed: {e}")),
                    };
                }
            };

            match order::build_order_detail_sync(&state.pool, order_pk).await {
                Ok(detail_sync) => CloudRpcResult::Json {
                    success: true,
                    data: serde_json::to_value(&detail_sync).ok(),
                    error: None,
                },
                Err(e) => CloudRpcResult::Json {
                    success: false,
                    data: None,
                    error: Some(format!("Order query failed: {e}")),
                },
            }
        }
        CloudRpc::RefreshSubscription => {
            state.activation.sync_subscription().await;
            CloudRpcResult::Json {
                success: true,
                data: Some(serde_json::json!({ "message": "Subscription refresh triggered" })),
                error: None,
            }
        }
        CloudRpc::StoreOp(op) => {
            let result = execute_catalog_op(state, op).await;
            CloudRpcResult::StoreOp(Box::new(result))
        }
    }
}

/// Execute a single StoreOp and broadcast changes to POS
pub async fn execute_catalog_op(state: &ServerState, op: &StoreOp) -> StoreOpResult {
    match op {
        // ── Product ──
        StoreOp::CreateProduct { id, data } => {
            catalog::create_product(state, *id, data.clone()).await
        }
        StoreOp::UpdateProduct { id, data } => {
            catalog::update_product(state, *id, data.clone()).await
        }
        StoreOp::DeleteProduct { id } => catalog::delete_product(state, *id).await,

        // ── Category ──
        StoreOp::CreateCategory { id, data } => {
            catalog::create_category(state, *id, data.clone()).await
        }
        StoreOp::UpdateCategory { id, data } => {
            catalog::update_category(state, *id, data.clone()).await
        }
        StoreOp::DeleteCategory { id } => catalog::delete_category(state, *id).await,

        // ── Attribute ──
        StoreOp::CreateAttribute { id, data } => attribute::create(state, *id, data.clone()).await,
        StoreOp::UpdateAttribute { id, data } => attribute::update(state, *id, data.clone()).await,
        StoreOp::DeleteAttribute { id } => attribute::delete(state, *id).await,

        // ── Attribute Binding ──
        StoreOp::BindAttribute {
            owner,
            attribute_id,
            is_required,
            display_order,
            default_option_ids,
        } => {
            attribute::bind(
                state,
                owner,
                *attribute_id,
                *is_required,
                *display_order,
                default_option_ids.clone(),
            )
            .await
        }
        StoreOp::UnbindAttribute { binding_id } => attribute::unbind(state, *binding_id).await,

        // ── Tag ──
        StoreOp::CreateTag { id, data } => attribute::create_tag(state, *id, data).await,
        StoreOp::UpdateTag { id, data } => attribute::update_tag(state, *id, data).await,
        StoreOp::DeleteTag { id } => attribute::delete_tag(state, *id).await,

        // ── Price Rule ──
        StoreOp::CreatePriceRule { id, data } => {
            resource::create_price_rule(state, *id, data.clone()).await
        }
        StoreOp::UpdatePriceRule { id, data } => {
            resource::update_price_rule(state, *id, data.clone()).await
        }
        StoreOp::DeletePriceRule { id } => resource::delete_price_rule(state, *id).await,

        // ── Employee ──
        StoreOp::CreateEmployee { id, data } => {
            resource::create_employee(state, *id, data.clone()).await
        }
        StoreOp::UpdateEmployee { id, data } => {
            resource::update_employee(state, *id, data.clone()).await
        }
        StoreOp::DeleteEmployee { id } => resource::delete_employee(state, *id).await,

        // ── Zone ──
        StoreOp::CreateZone { id, data } => resource::create_zone(state, *id, data.clone()).await,
        StoreOp::UpdateZone { id, data } => resource::update_zone(state, *id, data.clone()).await,
        StoreOp::DeleteZone { id } => resource::delete_zone(state, *id).await,

        // ── DiningTable ──
        StoreOp::CreateTable { id, data } => resource::create_table(state, *id, data.clone()).await,
        StoreOp::UpdateTable { id, data } => resource::update_table(state, *id, data.clone()).await,
        StoreOp::DeleteTable { id } => resource::delete_table(state, *id).await,

        // ── LabelTemplate ──
        StoreOp::CreateLabelTemplate { id, data } => {
            resource::create_label_template(state, *id, data.clone()).await
        }
        StoreOp::UpdateLabelTemplate { id, data } => {
            resource::update_label_template(state, *id, data.clone()).await
        }
        StoreOp::DeleteLabelTemplate { id } => resource::delete_label_template(state, *id).await,

        // ── Image ──
        StoreOp::EnsureImage {
            presigned_url,
            hash,
        } => provisioning::ensure_image(state, presigned_url, hash),

        // ── Full Sync (initial provisioning) ──
        StoreOp::FullSync { snapshot } => provisioning::apply_full_sync(state, snapshot).await,
    }
}
