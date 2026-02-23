//! Cloud RPC executor — dispatches CloudRpc to domain-specific handlers
//!
//! Handles all RPC types: GetStatus, GetOrderDetail, RefreshSubscription, CatalogOp.

use shared::cloud::catalog::{CatalogOp, CatalogOpResult};
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
        CloudRpc::CatalogOp(op) => {
            let result = execute_catalog_op(state, op).await;
            CloudRpcResult::CatalogOp(Box::new(result))
        }
    }
}

/// Execute a single CatalogOp and broadcast changes to POS
pub async fn execute_catalog_op(state: &ServerState, op: &CatalogOp) -> CatalogOpResult {
    match op {
        // ── Product ──
        CatalogOp::CreateProduct { data } => catalog::create_product(state, data.clone()).await,
        CatalogOp::UpdateProduct { id, data } => {
            catalog::update_product(state, *id, data.clone()).await
        }
        CatalogOp::DeleteProduct { id } => catalog::delete_product(state, *id).await,

        // ── Category ──
        CatalogOp::CreateCategory { data } => catalog::create_category(state, data.clone()).await,
        CatalogOp::UpdateCategory { id, data } => {
            catalog::update_category(state, *id, data.clone()).await
        }
        CatalogOp::DeleteCategory { id } => catalog::delete_category(state, *id).await,

        // ── Attribute ──
        CatalogOp::CreateAttribute { data } => attribute::create(state, data.clone()).await,
        CatalogOp::UpdateAttribute { id, data } => {
            attribute::update(state, *id, data.clone()).await
        }
        CatalogOp::DeleteAttribute { id } => attribute::delete(state, *id).await,

        // ── Attribute Binding ──
        CatalogOp::BindAttribute {
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
        CatalogOp::UnbindAttribute { binding_id } => attribute::unbind(state, *binding_id).await,

        // ── Tag ──
        CatalogOp::CreateTag { data } => attribute::create_tag(state, data).await,
        CatalogOp::UpdateTag { id, data } => attribute::update_tag(state, *id, data).await,
        CatalogOp::DeleteTag { id } => attribute::delete_tag(state, *id).await,

        // ── Price Rule ──
        CatalogOp::CreatePriceRule { data } => {
            resource::create_price_rule(state, data.clone()).await
        }
        CatalogOp::UpdatePriceRule { id, data } => {
            resource::update_price_rule(state, *id, data.clone()).await
        }
        CatalogOp::DeletePriceRule { id } => resource::delete_price_rule(state, *id).await,

        // ── Employee ──
        CatalogOp::CreateEmployee { data } => resource::create_employee(state, data.clone()).await,
        CatalogOp::UpdateEmployee { id, data } => {
            resource::update_employee(state, *id, data.clone()).await
        }
        CatalogOp::DeleteEmployee { id } => resource::delete_employee(state, *id).await,

        // ── Zone ──
        CatalogOp::CreateZone { data } => resource::create_zone(state, data.clone()).await,
        CatalogOp::UpdateZone { id, data } => resource::update_zone(state, *id, data.clone()).await,
        CatalogOp::DeleteZone { id } => resource::delete_zone(state, *id).await,

        // ── DiningTable ──
        CatalogOp::CreateTable { data } => resource::create_table(state, data.clone()).await,
        CatalogOp::UpdateTable { id, data } => {
            resource::update_table(state, *id, data.clone()).await
        }
        CatalogOp::DeleteTable { id } => resource::delete_table(state, *id).await,

        // ── Image ──
        CatalogOp::EnsureImage {
            presigned_url,
            hash,
        } => provisioning::ensure_image(state, presigned_url, hash),

        // ── Full Sync (initial provisioning) ──
        CatalogOp::FullSync { snapshot } => provisioning::apply_full_sync(state, snapshot).await,
    }
}
