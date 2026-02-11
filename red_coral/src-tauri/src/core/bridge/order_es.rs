//! Order event sourcing API

use super::*;

impl ClientBridge {
    /// Execute an order command (event sourcing)
    pub async fn execute_order_command(
        &self,
        command: OrderCommand,
    ) -> Result<CommandResponse, BridgeError> {
        let mode_guard = self.mode.read().await;

        match &*mode_guard {
            ClientMode::Server { server_state, .. } => {
                // 保存需要加载规则的命令信息
                let open_table_info = if let OrderCommandPayload::OpenTable {
                    zone_id, is_retail, ..
                } = &command.payload
                {
                    Some((*zone_id, *is_retail))
                } else {
                    None
                };
                let move_order_info = if let OrderCommandPayload::MoveOrder {
                    order_id,
                    target_zone_id,
                    ..
                } = &command.payload
                {
                    Some((order_id.clone(), *target_zone_id))
                } else {
                    None
                };

                let (response, events) = server_state
                    .orders_manager()
                    .execute_command_with_events(command);

                if response.success {
                    // OpenTable 成功后加载并缓存价格规则
                    if let Some((zone_id, is_retail)) = open_table_info {
                        if let Some(ref order_id) = response.order_id {
                            let rules = edge_server::orders::actions::open_table::load_matching_rules(
                                &server_state.pool,
                                zone_id,
                                is_retail,
                            )
                            .await;

                            if !rules.is_empty() {
                                tracing::debug!(
                                    order_id = %order_id,
                                    rule_count = rules.len(),
                                    "Cached order price rules (server mode)"
                                );
                                server_state.orders_manager().cache_rules(order_id, rules);
                            }
                        }
                    }

                    // MoveOrder 成功后：用新区域重新加载规则
                    if let Some((ref order_id, ref target_zone_id)) = move_order_info {
                        if let Ok(Some(snapshot)) = server_state.orders_manager().get_snapshot(order_id) {
                            let rules = edge_server::orders::actions::open_table::load_matching_rules(
                                &server_state.pool,
                                *target_zone_id,
                                snapshot.is_retail,
                            )
                            .await;
                            tracing::debug!(
                                order_id = %order_id,
                                target_zone_id = ?target_zone_id,
                                rule_count = rules.len(),
                                "Reloaded zone rules after table move (server mode)"
                            );
                            server_state.orders_manager().cache_rules(order_id, rules);
                        }
                    }
                }

                if let Some(handle) = &self.app_handle {
                    for event in events {
                        if let Err(e) = handle.emit("order-event", &event) {
                            tracing::warn!("Failed to emit order event: {}", e);
                        }
                    }
                }

                Ok(response)
            }
            ClientMode::Client { client, .. } => {
                // Send command via MessageBus RequestCommand protocol
                match client {
                    Some(RemoteClientState::Authenticated(auth)) => {
                        // Map OrderCommand payload type to action string
                        let action = match &command.payload {
                            shared::order::OrderCommandPayload::OpenTable { .. } => {
                                "order.open_table"
                            }
                            shared::order::OrderCommandPayload::CompleteOrder { .. } => {
                                "order.complete"
                            }
                            shared::order::OrderCommandPayload::VoidOrder { .. } => "order.void",
                            shared::order::OrderCommandPayload::AddItems { .. } => {
                                "order.add_items"
                            }
                            shared::order::OrderCommandPayload::ModifyItem { .. } => {
                                "order.modify_item"
                            }
                            shared::order::OrderCommandPayload::RemoveItem { .. } => {
                                "order.remove_item"
                            }
                            shared::order::OrderCommandPayload::AddPayment { .. } => {
                                "order.add_payment"
                            }
                            shared::order::OrderCommandPayload::CancelPayment { .. } => {
                                "order.cancel_payment"
                            }
                            shared::order::OrderCommandPayload::SplitByItems { .. } => {
                                "order.split_by_items"
                            }
                            shared::order::OrderCommandPayload::SplitByAmount { .. } => {
                                "order.split_by_amount"
                            }
                            shared::order::OrderCommandPayload::StartAaSplit { .. } => {
                                "order.start_aa_split"
                            }
                            shared::order::OrderCommandPayload::PayAaSplit { .. } => {
                                "order.pay_aa_split"
                            }
                            shared::order::OrderCommandPayload::MoveOrder { .. } => "order.move",
                            shared::order::OrderCommandPayload::MergeOrders { .. } => "order.merge",
                            shared::order::OrderCommandPayload::UpdateOrderInfo { .. } => {
                                "order.update_info"
                            }
                            shared::order::OrderCommandPayload::ToggleRuleSkip { .. } => {
                                "order.toggle_rule_skip"
                            }
                            shared::order::OrderCommandPayload::ApplyOrderDiscount { .. } => {
                                "order.apply_order_discount"
                            }
                            shared::order::OrderCommandPayload::ApplyOrderSurcharge { .. } => {
                                "order.apply_order_surcharge"
                            }
                            shared::order::OrderCommandPayload::CompItem { .. } => {
                                "order.comp_item"
                            }
                            shared::order::OrderCommandPayload::UncompItem { .. } => {
                                "order.uncomp_item"
                            }
                            shared::order::OrderCommandPayload::AddOrderNote { .. } => {
                                "order.add_order_note"
                            }
                            shared::order::OrderCommandPayload::LinkMember { .. } => {
                                "order.link_member"
                            }
                            shared::order::OrderCommandPayload::UnlinkMember { .. } => {
                                "order.unlink_member"
                            }
                            shared::order::OrderCommandPayload::RedeemStamp { .. } => {
                                "order.redeem_stamp"
                            }
                        };

                        // Build RequestCommand message with full command (preserves command_id, operator info)
                        let request_payload = shared::message::RequestCommandPayload {
                            action: action.to_string(),
                            params: serde_json::to_value(&command).ok(),
                        };
                        let request_msg =
                            shared::message::BusMessage::request_command(&request_payload);

                        // Send via MessageClient and wait for response
                        let response_msg = auth
                            .request(&request_msg)
                            .await
                            .map_err(|e| BridgeError::Server(format!("Request failed: {}", e)))?;

                        // Parse response
                        let response_payload: shared::message::ResponsePayload = response_msg
                            .parse_payload()
                            .map_err(|e| BridgeError::Server(format!("Invalid response: {}", e)))?;

                        if response_payload.success {
                            // Extract CommandResponse from data if present
                            if let Some(data) = response_payload.data {
                                let cmd_response: CommandResponse = serde_json::from_value(data)
                                    .unwrap_or_else(|_| CommandResponse {
                                        command_id: command.command_id.clone(),
                                        success: true,
                                        order_id: None,
                                        error: None,
                                    });
                                Ok(cmd_response)
                            } else {
                                Ok(CommandResponse {
                                    command_id: command.command_id,
                                    success: true,
                                    order_id: None,
                                    error: None,
                                })
                            }
                        } else {
                            Ok(CommandResponse {
                                command_id: command.command_id,
                                success: false,
                                order_id: None,
                                error: Some(shared::order::CommandError::new(
                                    shared::order::CommandErrorCode::InternalError,
                                    response_payload.message,
                                )),
                            })
                        }
                    }
                    Some(RemoteClientState::Connected(_)) => Err(BridgeError::NotAuthenticated),
                    None => Err(BridgeError::NotInitialized),
                }
            }
            ClientMode::Disconnected => Err(BridgeError::NotInitialized),
        }
    }

    /// Get all active order snapshots (event sourcing)
    pub async fn get_active_orders(&self) -> Result<Vec<OrderSnapshot>, BridgeError> {
        let mode_guard = self.mode.read().await;

        match &*mode_guard {
            ClientMode::Server { server_state, .. } => server_state
                .orders_manager()
                .get_active_orders()
                .map_err(|e| BridgeError::Server(e.to_string())),
            ClientMode::Client { client, .. } => match client {
                Some(RemoteClientState::Authenticated(auth)) => {
                    // Use sync.orders request to get active orders
                    let request_payload = shared::message::RequestCommandPayload {
                        action: "sync.orders".to_string(),
                        params: Some(serde_json::json!({ "since_sequence": 0 })),
                    };
                    let request_msg =
                        shared::message::BusMessage::request_command(&request_payload);

                    let response_msg = auth
                        .request(&request_msg)
                        .await
                        .map_err(|e| BridgeError::Server(format!("Request failed: {}", e)))?;

                    let response_payload: shared::message::ResponsePayload = response_msg
                        .parse_payload()
                        .map_err(|e| BridgeError::Server(format!("Invalid response: {}", e)))?;

                    if response_payload.success {
                        if let Some(data) = response_payload.data {
                            let sync_response: SyncResponse = serde_json::from_value(data)
                                .map_err(|e| {
                                    BridgeError::Server(format!("Invalid sync response: {}", e))
                                })?;
                            Ok(sync_response.active_orders)
                        } else {
                            Ok(vec![])
                        }
                    } else {
                        Err(BridgeError::Server(response_payload.message))
                    }
                }
                _ => Err(BridgeError::NotAuthenticated),
            },
            ClientMode::Disconnected => Err(BridgeError::NotInitialized),
        }
    }

    /// Get a single order snapshot by ID
    pub async fn get_order_snapshot(
        &self,
        order_id: &str,
    ) -> Result<Option<OrderSnapshot>, BridgeError> {
        let mode_guard = self.mode.read().await;

        match &*mode_guard {
            ClientMode::Server { server_state, .. } => server_state
                .orders_manager()
                .get_snapshot(order_id)
                .map_err(|e| BridgeError::Server(e.to_string())),
            ClientMode::Client { client, .. } => match client {
                Some(RemoteClientState::Authenticated(auth)) => {
                    // Use sync.order_snapshot request via MessageBus
                    let request_payload = shared::message::RequestCommandPayload {
                        action: "sync.order_snapshot".to_string(),
                        params: Some(serde_json::json!({ "order_id": order_id })),
                    };
                    let request_msg =
                        shared::message::BusMessage::request_command(&request_payload);

                    let response_msg = auth
                        .request(&request_msg)
                        .await
                        .map_err(|e| BridgeError::Server(format!("Request failed: {}", e)))?;

                    let response_payload: shared::message::ResponsePayload = response_msg
                        .parse_payload()
                        .map_err(|e| BridgeError::Server(format!("Invalid response: {}", e)))?;

                    if response_payload.success {
                        if let Some(data) = response_payload.data {
                            let snapshot: OrderSnapshot =
                                serde_json::from_value(data).map_err(|e| {
                                    BridgeError::Server(format!("Invalid snapshot: {}", e))
                                })?;
                            Ok(Some(snapshot))
                        } else {
                            Ok(None)
                        }
                    } else {
                        // Not found is not an error, just return None
                        if response_payload.error_code.as_deref() == Some("NOT_FOUND") {
                            Ok(None)
                        } else {
                            Err(BridgeError::Server(response_payload.message))
                        }
                    }
                }
                _ => Err(BridgeError::NotAuthenticated),
            },
            ClientMode::Disconnected => Err(BridgeError::NotInitialized),
        }
    }

    /// Sync orders since a given sequence (for reconnection)
    pub async fn sync_orders_since(
        &self,
        since_sequence: u64,
    ) -> Result<SyncResponse, BridgeError> {
        let mode_guard = self.mode.read().await;

        match &*mode_guard {
            ClientMode::Server { server_state, .. } => {
                let orders_manager = server_state.orders_manager();

                let events = orders_manager
                    .get_events_since(since_sequence)
                    .map_err(|e| BridgeError::Server(e.to_string()))?;

                let active_orders = orders_manager
                    .get_active_orders()
                    .map_err(|e| BridgeError::Server(e.to_string()))?;

                let server_sequence = orders_manager
                    .get_current_sequence()
                    .map_err(|e| BridgeError::Server(e.to_string()))?;

                Ok(SyncResponse {
                    events,
                    active_orders,
                    server_sequence,
                    requires_full_sync: since_sequence == 0,
                })
            }
            ClientMode::Client { client, .. } => match client {
                Some(RemoteClientState::Authenticated(auth)) => {
                    // Use sync.orders request via MessageBus
                    let request_payload = shared::message::RequestCommandPayload {
                        action: "sync.orders".to_string(),
                        params: Some(serde_json::json!({ "since_sequence": since_sequence })),
                    };
                    let request_msg =
                        shared::message::BusMessage::request_command(&request_payload);

                    let response_msg = auth
                        .request(&request_msg)
                        .await
                        .map_err(|e| BridgeError::Server(format!("Request failed: {}", e)))?;

                    let response_payload: shared::message::ResponsePayload = response_msg
                        .parse_payload()
                        .map_err(|e| BridgeError::Server(format!("Invalid response: {}", e)))?;

                    if response_payload.success {
                        if let Some(data) = response_payload.data {
                            let sync_response: SyncResponse = serde_json::from_value(data)
                                .map_err(|e| {
                                    BridgeError::Server(format!("Invalid sync response: {}", e))
                                })?;
                            Ok(sync_response)
                        } else {
                            Ok(SyncResponse {
                                events: vec![],
                                active_orders: vec![],
                                server_sequence: 0,
                                requires_full_sync: true,
                            })
                        }
                    } else {
                        Err(BridgeError::Server(response_payload.message))
                    }
                }
                _ => Err(BridgeError::NotAuthenticated),
            },
            ClientMode::Disconnected => Err(BridgeError::NotInitialized),
        }
    }

    /// Get events for active orders since a given sequence
    pub async fn get_active_events_since(
        &self,
        since_sequence: u64,
    ) -> Result<Vec<OrderEvent>, BridgeError> {
        let mode_guard = self.mode.read().await;

        match &*mode_guard {
            ClientMode::Server { server_state, .. } => server_state
                .orders_manager()
                .get_active_events_since(since_sequence)
                .map_err(|e| BridgeError::Server(e.to_string())),
            ClientMode::Client { client, .. } => match client {
                Some(RemoteClientState::Authenticated(auth)) => {
                    // Use sync.active_events request via MessageBus
                    let request_payload = shared::message::RequestCommandPayload {
                        action: "sync.active_events".to_string(),
                        params: Some(serde_json::json!({ "since_sequence": since_sequence })),
                    };
                    let request_msg =
                        shared::message::BusMessage::request_command(&request_payload);

                    let response_msg = auth
                        .request(&request_msg)
                        .await
                        .map_err(|e| BridgeError::Server(format!("Request failed: {}", e)))?;

                    let response_payload: shared::message::ResponsePayload = response_msg
                        .parse_payload()
                        .map_err(|e| BridgeError::Server(format!("Invalid response: {}", e)))?;

                    if response_payload.success {
                        if let Some(data) = response_payload.data {
                            let events: Vec<OrderEvent> =
                                serde_json::from_value(data).map_err(|e| {
                                    BridgeError::Server(format!("Invalid events: {}", e))
                                })?;
                            Ok(events)
                        } else {
                            Ok(vec![])
                        }
                    } else {
                        Err(BridgeError::Server(response_payload.message))
                    }
                }
                _ => Err(BridgeError::NotAuthenticated),
            },
            ClientMode::Disconnected => Err(BridgeError::NotInitialized),
        }
    }

    /// Get all events for a specific order (event sourcing)
    ///
    /// Used to reconstruct full order history including timeline.
    pub async fn get_events_for_order(
        &self,
        order_id: &str,
    ) -> Result<Vec<OrderEvent>, BridgeError> {
        let mode_guard = self.mode.read().await;

        match &*mode_guard {
            ClientMode::Server { server_state, .. } => server_state
                .orders_manager()
                .storage()
                .get_events_for_order(order_id)
                .map_err(|e| BridgeError::Server(e.to_string())),
            ClientMode::Client { client, .. } => match client {
                Some(RemoteClientState::Authenticated(auth)) => {
                    // Use sync.order_events request via MessageBus
                    let request_payload = shared::message::RequestCommandPayload {
                        action: "sync.order_events".to_string(),
                        params: Some(serde_json::json!({ "order_id": order_id })),
                    };
                    let request_msg =
                        shared::message::BusMessage::request_command(&request_payload);

                    let response_msg = auth
                        .request(&request_msg)
                        .await
                        .map_err(|e| BridgeError::Server(format!("Request failed: {}", e)))?;

                    let response_payload: shared::message::ResponsePayload = response_msg
                        .parse_payload()
                        .map_err(|e| BridgeError::Server(format!("Invalid response: {}", e)))?;

                    if response_payload.success {
                        if let Some(data) = response_payload.data {
                            let events: Vec<OrderEvent> =
                                serde_json::from_value(data).map_err(|e| {
                                    BridgeError::Server(format!("Invalid events: {}", e))
                                })?;
                            Ok(events)
                        } else {
                            Ok(vec![])
                        }
                    } else {
                        Err(BridgeError::Server(response_payload.message))
                    }
                }
                _ => Err(BridgeError::NotAuthenticated),
            },
            ClientMode::Disconnected => Err(BridgeError::NotInitialized),
        }
    }
}
