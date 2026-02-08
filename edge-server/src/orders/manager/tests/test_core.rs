use super::*;


#[test]
fn test_open_table() {
    let manager = create_test_manager();
    let cmd = create_open_table_cmd(1);

    let response = manager.execute_command(cmd);

    assert!(response.success);
    assert!(response.order_id.is_some());

    let order_id = response.order_id.unwrap();
    let snapshot = manager.get_snapshot(&order_id).unwrap();
    assert!(snapshot.is_some());

    let snapshot = snapshot.unwrap();
    assert_eq!(snapshot.status, OrderStatus::Active);
    assert_eq!(snapshot.table_id, Some(1));
}


#[test]
fn test_idempotency() {
    let manager = create_test_manager();
    let cmd = create_open_table_cmd(1);

    let response1 = manager.execute_command(cmd.clone());
    assert!(response1.success);
    let _order_id = response1.order_id.clone();

    // Execute same command again
    let response2 = manager.execute_command(cmd);
    assert!(response2.success);
    assert_eq!(response2.order_id, None); // Duplicate returns no order_id

    // Should still only have one order
    let orders = manager.get_active_orders().unwrap();
    assert_eq!(orders.len(), 1);
}


#[test]
fn test_add_items() {
    let manager = create_test_manager();

    // Open table
    let open_cmd = create_open_table_cmd(1);
    let open_response = manager.execute_command(open_cmd);
    let order_id = open_response.order_id.unwrap();

    // Add items
    let add_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::AddItems {
            order_id: order_id.clone(),
            items: vec![CartItemInput {
                product_id: 1,
                name: "Test Product".to_string(),
                price: 10.0,
                original_price: None,
                quantity: 2,
                selected_options: None,
                selected_specification: None,
                manual_discount_percent: None,
                note: None,
                authorizer_id: None,
                authorizer_name: None,
            }],
        },
    );

    let response = manager.execute_command(add_cmd);
    assert!(response.success);

    let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert_eq!(snapshot.items.len(), 1);
    assert_eq!(snapshot.items[0].quantity, 2);
    assert_eq!(snapshot.subtotal, 20.0);
}


#[test]
fn test_add_payment_and_complete() {
    let manager = create_test_manager();

    // Open table
    let open_cmd = create_open_table_cmd(1);
    let open_response = manager.execute_command(open_cmd);
    let order_id = open_response.order_id.unwrap();

    // Add items
    let add_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::AddItems {
            order_id: order_id.clone(),
            items: vec![CartItemInput {
                product_id: 1,
                name: "Test Product".to_string(),
                price: 10.0,
                original_price: None,
                quantity: 1,
                selected_options: None,
                selected_specification: None,
                manual_discount_percent: None,
                note: None,
                authorizer_id: None,
                authorizer_name: None,
            }],
        },
    );
    manager.execute_command(add_cmd);

    // Add payment
    let pay_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::AddPayment {
            order_id: order_id.clone(),
            payment: PaymentInput {
                method: "CASH".to_string(),
                amount: 10.0,
                tendered: Some(20.0),
                note: None,
            },
        },
    );
    let pay_response = manager.execute_command(pay_cmd);
    assert!(pay_response.success);

    let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert_eq!(snapshot.paid_amount, 10.0);
    assert_eq!(snapshot.payments.len(), 1);
    assert_eq!(snapshot.payments[0].change, Some(10.0));

    // Complete order (receipt_number comes from snapshot)
    let complete_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::CompleteOrder {
            order_id: order_id.clone(),
            service_type: Some(ServiceType::DineIn),
        },
    );
    let complete_response = manager.execute_command(complete_cmd);
    assert!(complete_response.success);

    let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert_eq!(snapshot.status, OrderStatus::Completed);
    assert!(!snapshot.receipt_number.is_empty()); // Server-generated at OpenTable
}


#[test]
fn test_void_order() {
    let manager = create_test_manager();

    // Open table
    let open_cmd = create_open_table_cmd(1);
    let open_response = manager.execute_command(open_cmd);
    let order_id = open_response.order_id.unwrap();

    // Void order
    let void_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::VoidOrder {
            order_id: order_id.clone(),
            void_type: VoidType::Cancelled,
            loss_reason: None,
            loss_amount: None,
            note: Some("Customer cancelled".to_string()),
            authorizer_id: None,
            authorizer_name: None,
        },
    );
    let void_response = manager.execute_command(void_cmd);
    assert!(void_response.success);

    let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert_eq!(snapshot.status, OrderStatus::Void);

    // Order should no longer be active
    let active_orders = manager.get_active_orders().unwrap();
    assert!(active_orders.is_empty());
}


#[test]
fn test_event_broadcast() {
    let manager = create_test_manager();
    let mut rx = manager.subscribe();

    // Open table
    let open_cmd = create_open_table_cmd(1);
    let _ = manager.execute_command(open_cmd);

    // Should receive event
    let event = rx.try_recv().unwrap();
    assert_eq!(event.event_type, OrderEventType::TableOpened);
}


// ========================================================================
// 1. rebuild_snapshot 一致性验证
// ========================================================================

#[test]
fn test_rebuild_snapshot_matches_stored() {
    let manager = create_test_manager();
    let order_id = open_table_with_items(
        &manager,
        111,
        vec![
            simple_item(1, "Coffee", 4.5, 2),
            simple_item(2, "Tea", 3.0, 1),
        ],
    );

    // Add a payment
    let pay_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::AddPayment {
            order_id: order_id.clone(),
            payment: PaymentInput {
                method: "CASH".to_string(),
                amount: 5.0,
                tendered: Some(10.0),
                note: None,
            },
        },
    );
    manager.execute_command(pay_cmd);

    // Get stored snapshot
    let stored = manager.get_snapshot(&order_id).unwrap().unwrap();

    // Rebuild from events
    let rebuilt = manager.rebuild_snapshot(&order_id).unwrap();

    // Core fields should match
    assert_eq!(stored.order_id, rebuilt.order_id);
    assert_eq!(stored.status, rebuilt.status);
    assert_eq!(stored.items.len(), rebuilt.items.len());
    assert_eq!(stored.payments.len(), rebuilt.payments.len());
    assert_eq!(stored.paid_amount, rebuilt.paid_amount);
    assert_eq!(stored.table_id, rebuilt.table_id);
    assert_eq!(stored.last_sequence, rebuilt.last_sequence);
    assert_eq!(stored.state_checksum, rebuilt.state_checksum);
}


// ========================================================================
// 2. MoveOrder — zone 信息正确更新
// ========================================================================

#[test]
fn test_move_order_zone_updates_correctly() {
    let manager = create_test_manager();
    let order_id = open_table_with_items(
        &manager,
        201,
        vec![simple_item(1, "Coffee", 5.0, 1)],
    );

    // Verify initial zone
    let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert_eq!(snapshot.zone_id, Some(1));
    assert_eq!(snapshot.zone_name, Some("Zone A".to_string()));

    // Move to a different table in a different zone
    let move_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::MoveOrder {
            order_id: order_id.clone(),
            target_table_id: 328,
            target_table_name: "Table T-move-2".to_string(),
            target_zone_id: Some(2),
            target_zone_name: Some("Zone B".to_string()),
            authorizer_id: None,
            authorizer_name: None,
        },
    );
    let resp = manager.execute_command(move_cmd);
    assert!(resp.success);

    let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert_eq!(snapshot.table_id, Some(328));
    assert_eq!(
        snapshot.zone_id,
        Some(2),
        "zone_id should be updated after MoveOrder"
    );
    assert_eq!(
        snapshot.zone_name,
        Some("Zone B".to_string()),
        "zone_name should be updated after MoveOrder"
    );
}


// ========================================================================
// 3. Merge 带支付的订单 — 存在支付记录时拒绝合并
// ========================================================================

#[test]
fn test_merge_orders_source_with_payment_rejected() {
    let manager = create_test_manager();

    // Source order with items and partial payment
    let source_id = open_table_with_items(
        &manager,
        202,
        vec![simple_item(1, "Coffee", 10.0, 2)],
    );

    // Pay partially on source
    let pay_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::AddPayment {
            order_id: source_id.clone(),
            payment: PaymentInput {
                method: "CASH".to_string(),
                amount: 5.0,
                tendered: None,
                note: None,
            },
        },
    );
    manager.execute_command(pay_cmd);

    let source_before = manager.get_snapshot(&source_id).unwrap().unwrap();
    assert_eq!(source_before.paid_amount, 5.0);

    // Target order
    let target_id = open_table_with_items(
        &manager,
        203,
        vec![simple_item(2, "Tea", 8.0, 1)],
    );

    // Merge source → target should be rejected
    let merge_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::MergeOrders {
            source_order_id: source_id.clone(),
            target_order_id: target_id.clone(),
            authorizer_id: None,
            authorizer_name: None,
        },
    );
    let resp = manager.execute_command(merge_cmd);
    assert!(!resp.success, "存在支付记录的订单不能合并");

    // Source and target should remain unchanged
    let source_after = manager.get_snapshot(&source_id).unwrap().unwrap();
    assert_eq!(source_after.paid_amount, 5.0);
    assert_eq!(source_after.status, OrderStatus::Active);

    let target_after = manager.get_snapshot(&target_id).unwrap().unwrap();
    assert_eq!(target_after.items.len(), 1, "Target should be unchanged");
    assert_eq!(target_after.paid_amount, 0.0);
}


// ========================================================================
// 4. AddPayment 超额支付
// ========================================================================

#[test]
fn test_add_payment_overpay_is_rejected() {
    let manager = create_test_manager();
    let order_id = open_table_with_items(
        &manager,
        204,
        vec![simple_item(1, "Coffee", 10.0, 1)],
    );

    // Pay way more than the total — should be rejected
    let pay_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::AddPayment {
            order_id: order_id.clone(),
            payment: PaymentInput {
                method: "CASH".to_string(),
                amount: 10000.0,
                tendered: None,
                note: None,
            },
        },
    );
    let resp = manager.execute_command(pay_cmd);
    assert!(
        !resp.success,
        "AddPayment should reject overpayment"
    );

    let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert_eq!(snapshot.paid_amount, 0.0);
}


// ========================================================================
// 5. cancel_payment → re-pay → complete 完整流程
// ========================================================================

#[test]
fn test_cancel_payment_then_repay_then_complete() {
    let manager = create_test_manager();
    let order_id = open_table_with_items(
        &manager,
        205,
        vec![simple_item(1, "Coffee", 10.0, 1)],
    );

    // Pay with CARD
    let pay_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::AddPayment {
            order_id: order_id.clone(),
            payment: PaymentInput {
                method: "CARD".to_string(),
                amount: 10.0,
                tendered: None,
                note: None,
            },
        },
    );
    manager.execute_command(pay_cmd);

    // Get payment_id
    let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
    let payment_id = snapshot.payments[0].payment_id.clone();
    assert_eq!(snapshot.paid_amount, 10.0);

    // Cancel the payment
    let cancel_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::CancelPayment {
            order_id: order_id.clone(),
            payment_id,
            reason: Some("Wrong card".to_string()),
            authorizer_id: None,
            authorizer_name: None,
        },
    );
    let resp = manager.execute_command(cancel_cmd);
    assert!(resp.success);

    let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert_eq!(snapshot.paid_amount, 0.0, "After cancel, paid should be 0");
    assert!(snapshot.payments[0].cancelled);

    // Re-pay with CASH
    let repay_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::AddPayment {
            order_id: order_id.clone(),
            payment: PaymentInput {
                method: "CASH".to_string(),
                amount: 10.0,
                tendered: Some(20.0),
                note: None,
            },
        },
    );
    manager.execute_command(repay_cmd);

    let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert_eq!(snapshot.paid_amount, 10.0);
    assert_eq!(snapshot.payments.len(), 2);

    // Complete
    let complete_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::CompleteOrder {
            order_id: order_id.clone(),
            service_type: Some(ServiceType::DineIn),
        },
    );
    let resp = manager.execute_command(complete_cmd);
    assert!(resp.success);

    let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert_eq!(snapshot.status, OrderStatus::Completed);
}


// ========================================================================
// 6. 空订单 complete
// ========================================================================

#[test]
fn test_complete_empty_order() {
    let manager = create_test_manager();
    let order_id = open_table_with_items(&manager, 100, vec![]);

    let complete_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::CompleteOrder {
            order_id: order_id.clone(),
            service_type: Some(ServiceType::DineIn),
        },
    );
    let resp = manager.execute_command(complete_cmd);
    // Zero-total orders should complete (e.g., complimentary)
    assert!(resp.success, "Zero-total order should complete successfully");

    let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert_eq!(snapshot.status, OrderStatus::Completed);
}


// ========================================================================
// 7. Sequence 单调递增
// ========================================================================

#[test]
fn test_sequence_monotonically_increasing() {
    let manager = create_test_manager();
    let mut rx = manager.subscribe();

    let order_id = open_table_with_items(
        &manager,
        206,
        vec![simple_item(1, "Coffee", 5.0, 1)],
    );

    let add_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::AddItems {
            order_id: order_id.clone(),
            items: vec![simple_item(2, "Tea", 3.0, 1)],
        },
    );
    manager.execute_command(add_cmd);

    let mut sequences = Vec::new();
    while let Ok(event) = rx.try_recv() {
        sequences.push(event.sequence);
    }

    assert!(sequences.len() >= 3, "Should have at least 3 events");
    for window in sequences.windows(2) {
        assert!(
            window[1] > window[0],
            "Sequences must be strictly increasing: {} should be > {}",
            window[1],
            window[0]
        );
    }
}


// ========================================================================
// 8. 重复打开相同桌台应失败
// ========================================================================

#[test]
fn test_open_same_table_twice_fails() {
    let manager = create_test_manager();

    let cmd1 = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::OpenTable {
            table_id: Some(307),
            table_name: Some("Table Dup".to_string()),
            zone_id: None,
            zone_name: None,
            guest_count: 2,
            is_retail: false,
        },
    );
    let resp1 = manager.execute_command(cmd1);
    assert!(resp1.success);

    let cmd2 = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::OpenTable {
            table_id: Some(307),
            table_name: Some("Table Dup".to_string()),
            zone_id: None,
            zone_name: None,
            guest_count: 3,
            is_retail: false,
        },
    );
    let resp2 = manager.execute_command(cmd2);
    assert!(!resp2.success, "Opening the same table twice should fail");
}


// ========================================================================
// 9. void 已 void 的订单应失败
// ========================================================================

#[test]
fn test_void_already_voided_order_fails() {
    let manager = create_test_manager();
    let order_id = open_table_with_items(&manager, 101, vec![]);

    let void_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::VoidOrder {
            order_id: order_id.clone(),
            void_type: VoidType::Cancelled,
            loss_reason: None,
            loss_amount: None,
            note: None,
            authorizer_id: None,
            authorizer_name: None,
        },
    );
    let resp = manager.execute_command(void_cmd);
    assert!(resp.success);

    let void_cmd2 = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::VoidOrder {
            order_id: order_id.clone(),
            void_type: VoidType::Cancelled,
            loss_reason: None,
            loss_amount: None,
            note: None,
            authorizer_id: None,
            authorizer_name: None,
        },
    );
    let resp2 = manager.execute_command(void_cmd2);
    assert!(!resp2.success, "Voiding an already voided order should fail");
}


// ========================================================================
// 10. 移桌后结账完整流程
// ========================================================================

#[test]
fn test_move_order_then_complete() {
    let manager = create_test_manager();
    let order_id = open_table_with_items(
        &manager,
        207,
        vec![simple_item(1, "Coffee", 10.0, 1)],
    );

    let move_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::MoveOrder {
            order_id: order_id.clone(),
            target_table_id: 329,
            target_table_name: "Table 2".to_string(),
            target_zone_id: None,
            target_zone_name: None,
            authorizer_id: None,
            authorizer_name: None,
        },
    );
    let resp = manager.execute_command(move_cmd);
    assert!(resp.success);

    let pay_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::AddPayment {
            order_id: order_id.clone(),
            payment: PaymentInput {
                method: "CARD".to_string(),
                amount: 10.0,
                tendered: None,
                note: None,
            },
        },
    );
    manager.execute_command(pay_cmd);

    let complete_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::CompleteOrder {
            order_id: order_id.clone(),
            service_type: Some(ServiceType::DineIn),
        },
    );
    let resp = manager.execute_command(complete_cmd);
    assert!(resp.success);

    let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert_eq!(snapshot.status, OrderStatus::Completed);
    assert_eq!(snapshot.table_id, Some(329));
}


// ========================================================================
// 11. split by items → complete 流程
// ========================================================================

#[test]
fn test_split_by_items_then_complete() {
    let manager = create_test_manager();
    let order_id = open_table_with_items(
        &manager,
        208,
        vec![
            simple_item(1, "Coffee", 10.0, 2),
            simple_item(2, "Tea", 8.0, 1),
        ],
    );

    let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert_eq!(snapshot.total, 28.0);
    let coffee_instance = snapshot.items[0].instance_id.clone();

    // Split pay: 2x Coffee = 20.0
    let split_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::SplitByItems {
            order_id: order_id.clone(),
            payment_method: "CASH".to_string(),
            items: vec![shared::order::SplitItem {
                instance_id: coffee_instance,
                name: "Coffee".to_string(),
                quantity: 2,
                unit_price: 10.0,
            }],
            tendered: None,
        },
    );
    let resp = manager.execute_command(split_cmd);
    assert!(resp.success);

    let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert_eq!(snapshot.paid_amount, 20.0);

    // Pay remaining: Tea = 8.0
    let pay_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::AddPayment {
            order_id: order_id.clone(),
            payment: PaymentInput {
                method: "CARD".to_string(),
                amount: 8.0,
                tendered: None,
                note: None,
            },
        },
    );
    manager.execute_command(pay_cmd);

    let complete_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::CompleteOrder {
            order_id: order_id.clone(),
            service_type: Some(ServiceType::DineIn),
        },
    );
    let resp = manager.execute_command(complete_cmd);
    assert!(resp.success);

    let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert_eq!(snapshot.status, OrderStatus::Completed);
}


// ========================================================================
// 12. AA split 完整流程 → complete
// ========================================================================

#[test]
fn test_aa_split_full_flow_then_complete() {
    let manager = create_test_manager();
    let order_id = open_table_with_items(
        &manager,
        209,
        vec![simple_item(1, "Coffee", 30.0, 1)],
    );

    // Start AA: 3 shares, pay 1
    let start_aa_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::StartAaSplit {
            order_id: order_id.clone(),
            total_shares: 3,
            shares: 1,
            payment_method: "CASH".to_string(),
            tendered: None,
        },
    );
    let resp = manager.execute_command(start_aa_cmd);
    assert!(resp.success);

    let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert_eq!(snapshot.aa_total_shares, Some(3));
    assert_eq!(snapshot.aa_paid_shares, 1);
    assert_eq!(snapshot.paid_amount, 10.0);

    // Pay share 2
    let pay_aa_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::PayAaSplit {
            order_id: order_id.clone(),
            shares: 1,
            payment_method: "CARD".to_string(),
            tendered: None,
        },
    );
    let resp = manager.execute_command(pay_aa_cmd);
    assert!(resp.success);

    // Pay share 3 (last — should get exact remaining)
    let pay_aa_last = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::PayAaSplit {
            order_id: order_id.clone(),
            shares: 1,
            payment_method: "CASH".to_string(),
            tendered: None,
        },
    );
    let resp = manager.execute_command(pay_aa_last);
    assert!(resp.success);

    let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert_eq!(snapshot.aa_paid_shares, 3);
    assert_eq!(snapshot.paid_amount, 30.0);

    // Complete
    let complete_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::CompleteOrder {
            order_id: order_id.clone(),
            service_type: Some(ServiceType::DineIn),
        },
    );
    let resp = manager.execute_command(complete_cmd);
    assert!(resp.success);

    let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert_eq!(snapshot.status, OrderStatus::Completed);
}


// ========================================================================
// 13. 零售订单应生成 queue_number
// ========================================================================

#[test]
fn test_retail_order_gets_queue_number() {
    let manager = create_test_manager();

    let cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::OpenTable {
            table_id: None,
            table_name: None,
            zone_id: None,
            zone_name: None,
            guest_count: 1,
            is_retail: true,
        },
    );
    let resp = manager.execute_command(cmd);
    assert!(resp.success);

    let order_id = resp.order_id.unwrap();
    let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert!(snapshot.queue_number.is_some(), "Retail order should have queue number");
    assert!(snapshot.is_retail);
}


// ========================================================================
// 14. execute_command_with_events 返回 events
// ========================================================================

#[test]
fn test_execute_command_with_events_returns_events() {
    let manager = create_test_manager();

    let cmd = create_open_table_cmd(1);
    let (resp, events) = manager.execute_command_with_events(cmd);

    assert!(resp.success);
    assert!(!events.is_empty());
    assert_eq!(events[0].event_type, OrderEventType::TableOpened);
}


// ========================================================================
// 15. get_events_since 完整性
// ========================================================================

#[test]
fn test_get_events_since_completeness() {
    let manager = create_test_manager();

    let order_id = open_table_with_items(
        &manager,
        210,
        vec![simple_item(1, "Coffee", 10.0, 1)],
    );

    let seq_before = manager.get_current_sequence().unwrap();

    let pay_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::AddPayment {
            order_id: order_id.clone(),
            payment: PaymentInput {
                method: "CASH".to_string(),
                amount: 10.0,
                tendered: None,
                note: None,
            },
        },
    );
    manager.execute_command(pay_cmd);

    let events = manager.get_events_since(seq_before).unwrap();
    assert!(!events.is_empty());
    assert!(events.iter().any(|e| e.event_type == OrderEventType::PaymentAdded));
}


// ========================================================================
// 16. 合并后源订单变为 Merged 状态
// ========================================================================

#[test]
fn test_merge_source_becomes_merged_status() {
    let manager = create_test_manager();

    let source_id = open_table_with_items(
        &manager,
        211,
        vec![simple_item(1, "Coffee", 5.0, 1)],
    );
    let target_id = open_table_with_items(
        &manager,
        212,
        vec![simple_item(2, "Tea", 3.0, 1)],
    );

    let merge_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::MergeOrders {
            source_order_id: source_id.clone(),
            target_order_id: target_id.clone(),
            authorizer_id: None,
            authorizer_name: None,
        },
    );
    let resp = manager.execute_command(merge_cmd);
    assert!(resp.success);

    let source = manager.get_snapshot(&source_id).unwrap().unwrap();
    assert_eq!(source.status, OrderStatus::Merged);

    let active = manager.get_active_orders().unwrap();
    assert!(active.iter().all(|o| o.order_id != source_id));

    let target = manager.get_snapshot(&target_id).unwrap().unwrap();
    assert_eq!(target.status, OrderStatus::Active);
    assert_eq!(target.items.len(), 2);
}


// ========================================================================
// 17. 操作不存在的订单应返回错误
// ========================================================================

#[test]
fn test_operations_on_nonexistent_order_fail() {
    let manager = create_test_manager();

    let pay_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::AddPayment {
            order_id: "nonexistent".to_string(),
            payment: PaymentInput {
                method: "CASH".to_string(),
                amount: 10.0,
                tendered: None,
                note: None,
            },
        },
    );
    let resp = manager.execute_command(pay_cmd);
    assert!(!resp.success);

    let complete_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::CompleteOrder {
            order_id: "nonexistent".to_string(),
            service_type: Some(ServiceType::DineIn),
        },
    );
    let resp = manager.execute_command(complete_cmd);
    assert!(!resp.success);

    let void_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::VoidOrder {
            order_id: "nonexistent".to_string(),
            void_type: VoidType::Cancelled,
            loss_reason: None,
            loss_amount: None,
            note: None,
            authorizer_id: None,
            authorizer_name: None,
        },
    );
    let resp = manager.execute_command(void_cmd);
    assert!(!resp.success);
}

