use super::*;


// ========================================================================
// ========================================================================
//  P0: 核心业务流程测试
// ========================================================================
// ========================================================================

// ------------------------------------------------------------------------
// P0.1: 完整堂食订单生命周期
// OpenTable → AddItems(3种) → ModifyItem → AddPayment → CompleteOrder
// ------------------------------------------------------------------------
#[test]
fn test_complete_dine_in_flow() {
    let manager = create_test_manager();

    // 1. 开台
    let open_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::OpenTable {
            table_id: Some(325),
            table_name: Some("Table Dine Flow".to_string()),
            zone_id: Some(1),
            zone_name: Some("Zone A".to_string()),
            guest_count: 4,
            is_retail: false,
        },
    );
    let resp = manager.execute_command(open_cmd);
    assert!(resp.success, "OpenTable should succeed");
    let order_id = resp.order_id.unwrap();

    // 验证开台状态
    let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert_eq!(snapshot.status, OrderStatus::Active);
    assert_eq!(snapshot.guest_count, 4);
    assert!(!snapshot.receipt_number.is_empty());

    // 2. 添加 3 种商品
    let add_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::AddItems {
            order_id: order_id.clone(),
            items: vec![
                simple_item(10, "Coffee", 5.0, 2),      // 10.0
                simple_item(11, "Tea", 3.0, 3),            // 9.0
                simple_item(12, "Cake", 12.50, 1),        // 12.50
            ],
        },
    );
    let resp = manager.execute_command(add_cmd);
    assert!(resp.success, "AddItems should succeed");

    let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert_eq!(snapshot.items.len(), 3);
    assert_eq!(snapshot.subtotal, 31.5); // 10 + 9 + 12.5

    // 3. ModifyItem: 减少 Tea 数量 3 → 2
    let tea_instance_id = snapshot.items.iter()
        .find(|i| i.name == "Tea")
        .unwrap()
        .instance_id.clone();

    let modify_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::ModifyItem {
            order_id: order_id.clone(),
            instance_id: tea_instance_id,
            affected_quantity: None,
            changes: shared::order::ItemChanges {
                quantity: Some(2),
                ..Default::default()
            },
            authorizer_id: None,
            authorizer_name: None,
        },
    );
    let resp = manager.execute_command(modify_cmd);
    assert!(resp.success, "ModifyItem should succeed");

    let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
    // 新总额: 10 + 6 + 12.5 = 28.5
    assert_eq!(snapshot.subtotal, 28.5);
    assert_eq!(snapshot.total, 28.5);

    // 4. 全额支付
    let pay_resp = pay(&manager, &order_id, 28.5, "CARD");
    assert!(pay_resp.success, "Payment should succeed");

    let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert_eq!(snapshot.paid_amount, 28.5);

    // 5. 完成订单
    let complete_resp = complete_order(&manager, &order_id);
    assert!(complete_resp.success, "CompleteOrder should succeed");

    let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert_eq!(snapshot.status, OrderStatus::Completed);
    assert!(snapshot.end_time.is_some());
}


// ------------------------------------------------------------------------
// P0.2: 完整零售订单流程 (带 queue_number)
// ------------------------------------------------------------------------
#[test]
fn test_complete_retail_flow_with_queue_number() {
    let manager = create_test_manager();

    // 1. 开零售订单
    let order_id = open_retail_order(&manager);

    let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert!(snapshot.is_retail);
    assert!(snapshot.queue_number.is_some(), "Retail order should have queue_number");
    assert!(snapshot.table_id.is_none(), "Retail order should have no table_id");

    // 2. 添加商品
    let add_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::AddItems {
            order_id: order_id.clone(),
            items: vec![simple_item(1, "Item", 15.0, 2)],
        },
    );
    manager.execute_command(add_cmd);

    // 3. 支付
    pay(&manager, &order_id, 30.0, "CASH");

    // 4. 完成 (指定服务类型为外带)
    let complete_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::CompleteOrder {
            order_id: order_id.clone(),
            service_type: Some(ServiceType::Takeout),
        },
    );
    let resp = manager.execute_command(complete_cmd);
    assert!(resp.success);

    let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert_eq!(snapshot.status, OrderStatus::Completed);
    assert_eq!(snapshot.service_type, Some(ServiceType::Takeout));
}


// ------------------------------------------------------------------------
// P0.3: VoidOrder 损失结算
// ------------------------------------------------------------------------
#[test]
fn test_void_order_loss_settlement() {
    let manager = create_test_manager();

    // 开台 + 添加商品
    let order_id = open_table_with_items(
        &manager,
        248,
        vec![simple_item(1, "Expensive Item", 100.0, 1)],
    );

    // 损失结算作废
    let void_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::VoidOrder {
            order_id: order_id.clone(),
            void_type: VoidType::LossSettled,
            loss_reason: Some(shared::order::LossReason::CustomerFled),
            loss_amount: Some(100.0),
            note: Some("Customer fled without paying".to_string()),
            authorizer_id: Some(1),
            authorizer_name: Some("Manager".to_string()),
        },
    );
    let resp = manager.execute_command(void_cmd);
    assert!(resp.success);

    let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert_eq!(snapshot.status, OrderStatus::Void);
    assert_eq!(snapshot.void_type, Some(VoidType::LossSettled));
    assert_eq!(snapshot.loss_reason, Some(shared::order::LossReason::CustomerFled));
    assert_eq!(snapshot.loss_amount, Some(100.0));
}


// ------------------------------------------------------------------------
// P0.4: 多次菜品分单后完成
// ------------------------------------------------------------------------
#[test]
fn test_split_by_items_multiple_then_complete() {
    let manager = create_test_manager();

    let order_id = open_table_with_items(
        &manager,
        249,
        vec![
            simple_item(1, "Item A", 20.0, 2),  // 40
            simple_item(2, "Item B", 15.0, 2),  // 30
            simple_item(3, "Item C", 10.0, 1),  // 10
        ],
    );

    let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert_eq!(snapshot.total, 80.0);
    let item_a_id = snapshot.items.iter().find(|i| i.name == "Item A").unwrap().instance_id.clone();
    let item_b_id = snapshot.items.iter().find(|i| i.name == "Item B").unwrap().instance_id.clone();

    // 第一次分单: 支付 Item A 的 1 个 (20.0)
    let split_cmd1 = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::SplitByItems {
            order_id: order_id.clone(),
            payment_method: "CASH".to_string(),
            items: vec![shared::order::SplitItem {
                instance_id: item_a_id.clone(),
                name: "Item A".to_string(),
                quantity: 1,
                unit_price: 20.0,
            }],
            tendered: Some(20.0),
        },
    );
    let resp = manager.execute_command(split_cmd1);
    assert!(resp.success, "First split should succeed");

    let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert_eq!(snapshot.paid_amount, 20.0);
    assert_eq!(snapshot.paid_item_quantities.get(&item_a_id), Some(&1));

    // 第二次分单: 支付 Item B 的 2 个 (30.0)
    let split_cmd2 = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::SplitByItems {
            order_id: order_id.clone(),
            payment_method: "CARD".to_string(),
            items: vec![shared::order::SplitItem {
                instance_id: item_b_id.clone(),
                name: "Item B".to_string(),
                quantity: 2,
                unit_price: 15.0,
            }],
            tendered: None,
        },
    );
    let resp = manager.execute_command(split_cmd2);
    assert!(resp.success, "Second split should succeed");

    let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert_eq!(snapshot.paid_amount, 50.0);

    // 支付剩余 (Item A 的 1 个 + Item C): 20 + 10 = 30
    pay(&manager, &order_id, 30.0, "CASH");

    let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert_eq!(snapshot.paid_amount, 80.0);

    // 完成
    let complete_resp = complete_order(&manager, &order_id);
    assert!(complete_resp.success);
    assert_order_status(&manager, &order_id, OrderStatus::Completed);
}


// ------------------------------------------------------------------------
// P0.5: 多次金额分单后完成
// ------------------------------------------------------------------------
#[test]
fn test_split_by_amount_multiple_then_complete() {
    let manager = create_test_manager();

    let order_id = open_table_with_items(
        &manager,
        250,
        vec![simple_item(1, "Total Item", 100.0, 1)],
    );

    // 第一次金额分单: 30%
    let split_cmd1 = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::SplitByAmount {
            order_id: order_id.clone(),
            split_amount: 30.0,
            payment_method: "CASH".to_string(),
            tendered: Some(30.0),
        },
    );
    let resp = manager.execute_command(split_cmd1);
    assert!(resp.success);

    let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert_eq!(snapshot.paid_amount, 30.0);
    assert!(snapshot.has_amount_split, "has_amount_split should be true");

    // 第二次金额分单: 30%
    let split_cmd2 = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::SplitByAmount {
            order_id: order_id.clone(),
            split_amount: 30.0,
            payment_method: "CARD".to_string(),
            tendered: None,
        },
    );
    let resp = manager.execute_command(split_cmd2);
    assert!(resp.success);

    let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert_eq!(snapshot.paid_amount, 60.0);

    // 支付剩余 40%
    pay(&manager, &order_id, 40.0, "CASH");

    // 完成
    let complete_resp = complete_order(&manager, &order_id);
    assert!(complete_resp.success);
    assert_order_status(&manager, &order_id, OrderStatus::Completed);
}


// ------------------------------------------------------------------------
// P0.6: AA 分单 3 人不能整除场景 (精度测试)
// ------------------------------------------------------------------------
#[test]
fn test_aa_split_three_payers_indivisible() {
    let manager = create_test_manager();

    // 100 元订单，3 人 AA
    let order_id = open_table_with_items(
        &manager,
        251,
        vec![simple_item(1, "Shared Meal", 100.0, 1)],
    );

    // StartAaSplit: 3 人，先付 1 份
    let start_aa = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::StartAaSplit {
            order_id: order_id.clone(),
            total_shares: 3,
            shares: 1,
            payment_method: "CASH".to_string(),
            tendered: Some(34.0), // 给 34 块
        },
    );
    let resp = manager.execute_command(start_aa);
    assert!(resp.success, "StartAaSplit should succeed");

    let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert_eq!(snapshot.aa_total_shares, Some(3));
    assert_eq!(snapshot.aa_paid_shares, 1);
    // 100 / 3 ≈ 33.33，实际支付第一份
    let first_share = snapshot.paid_amount;
    assert!(first_share > 33.0 && first_share < 34.0, "First share should be ~33.33");

    // PayAaSplit: 第二份
    let pay_aa_2 = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::PayAaSplit {
            order_id: order_id.clone(),
            shares: 1,
            payment_method: "CARD".to_string(),
            tendered: None,
        },
    );
    let resp = manager.execute_command(pay_aa_2);
    assert!(resp.success);

    let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert_eq!(snapshot.aa_paid_shares, 2);

    // PayAaSplit: 第三份 (最后一份应该拿剩余金额)
    let pay_aa_3 = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::PayAaSplit {
            order_id: order_id.clone(),
            shares: 1,
            payment_method: "CASH".to_string(),
            tendered: Some(34.0),
        },
    );
    let resp = manager.execute_command(pay_aa_3);
    assert!(resp.success);

    let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert_eq!(snapshot.aa_paid_shares, 3);
    // 精度验证: 总支付应该恰好等于 100.0
    let diff = (snapshot.paid_amount - 100.0).abs();
    assert!(diff < 0.01, "Total paid should be exactly 100.0, got {}", snapshot.paid_amount);

    // 完成
    let complete_resp = complete_order(&manager, &order_id);
    assert!(complete_resp.success);
}


// ------------------------------------------------------------------------
// P0.7: 合并订单后修改并完成
// ------------------------------------------------------------------------
#[test]
fn test_merge_orders_then_modify_then_complete() {
    let manager = create_test_manager();

    // 源订单
    let source_id = open_table_with_items(
        &manager,
        252,
        vec![simple_item(1, "Coffee", 5.0, 2)], // 10
    );

    // 目标订单
    let target_id = open_table_with_items(
        &manager,
        253,
        vec![simple_item(2, "Tea", 4.0, 1)], // 4
    );

    // 合并
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
    assert!(resp.success, "Merge should succeed");

    // 验证源订单状态
    assert_order_status(&manager, &source_id, OrderStatus::Merged);

    // 验证目标订单
    let target = manager.get_snapshot(&target_id).unwrap().unwrap();
    assert_eq!(target.items.len(), 2, "Target should have 2 items after merge");
    assert_eq!(target.total, 14.0); // 10 + 4

    // 继续在目标订单添加商品
    let add_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::AddItems {
            order_id: target_id.clone(),
            items: vec![simple_item(3, "Cake", 6.0, 1)],
        },
    );
    let resp = manager.execute_command(add_cmd);
    assert!(resp.success, "AddItems to merged target should succeed");

    let target = manager.get_snapshot(&target_id).unwrap().unwrap();
    assert_eq!(target.items.len(), 3);
    assert_eq!(target.total, 20.0);

    // 支付并完成
    pay(&manager, &target_id, 20.0, "CARD");
    let complete_resp = complete_order(&manager, &target_id);
    assert!(complete_resp.success);
}


// ------------------------------------------------------------------------
// P0.8: 移桌后合并再完成
// ------------------------------------------------------------------------
#[test]
fn test_move_then_merge_then_complete() {
    let manager = create_test_manager();

    // 订单 1: T1
    let order1 = open_table_with_items(
        &manager,
        401,
        vec![simple_item(1, "Item 1", 10.0, 1)],
    );

    // 移桌: T1 → T2
    let move_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::MoveOrder {
            order_id: order1.clone(),
            target_table_id: 400,
            target_table_name: "Table 2".to_string(),
            target_zone_id: Some(2),
            target_zone_name: Some("Zone B".to_string()),
            authorizer_id: None,
            authorizer_name: None,
        },
    );
    let resp = manager.execute_command(move_cmd);
    assert!(resp.success);

    let snapshot = manager.get_snapshot(&order1).unwrap().unwrap();
    assert_eq!(snapshot.table_id, Some(400));
    assert_eq!(snapshot.zone_id, Some(2));

    // 订单 2: T3
    let order2 = open_table_with_items(
        &manager,
        403,
        vec![simple_item(2, "Item 2", 20.0, 1)],
    );

    // 合并: T2 (order1) → T3 (order2)
    let merge_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::MergeOrders {
            source_order_id: order1.clone(),
            target_order_id: order2.clone(),
            authorizer_id: None,
            authorizer_name: None,
        },
    );
    let resp = manager.execute_command(merge_cmd);
    assert!(resp.success);

    // order1 应该是 Merged 状态
    assert_order_status(&manager, &order1, OrderStatus::Merged);

    // order2 应该有所有商品
    let target = manager.get_snapshot(&order2).unwrap().unwrap();
    assert_eq!(target.items.len(), 2);
    assert_eq!(target.total, 30.0);

    // 完成
    pay(&manager, &order2, 30.0, "CASH");
    let complete_resp = complete_order(&manager, &order2);
    assert!(complete_resp.success);
}


// ------------------------------------------------------------------------
// P0.9: 商品添加→修改→移除链条 (金额重算验证)
// ------------------------------------------------------------------------
#[test]
fn test_items_add_modify_remove_chain() {
    let manager = create_test_manager();

    let order_id = open_table_with_items(
        &manager,
        254,
        vec![simple_item(1, "Test Item", 10.0, 5)], // 50
    );

    let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
    let instance_id = snapshot.items[0].instance_id.clone();
    assert_eq!(snapshot.subtotal, 50.0);

    // ModifyItem: 数量 5 → 3
    let modify_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::ModifyItem {
            order_id: order_id.clone(),
            instance_id: instance_id.clone(),
            affected_quantity: None,
            changes: shared::order::ItemChanges {
                quantity: Some(3),
                ..Default::default()
            },
            authorizer_id: None,
            authorizer_name: None,
        },
    );
    let resp = manager.execute_command(modify_cmd);
    assert!(resp.success);

    let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert_eq!(snapshot.items[0].quantity, 3);
    assert_eq!(snapshot.subtotal, 30.0);

    // RemoveItem: 移除 2 个
    let remove_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::RemoveItem {
            order_id: order_id.clone(),
            instance_id: instance_id.clone(),
            quantity: Some(2),
            reason: Some("Customer changed mind".to_string()),
            authorizer_id: None,
            authorizer_name: None,
        },
    );
    let resp = manager.execute_command(remove_cmd);
    assert!(resp.success);

    let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert_eq!(snapshot.items[0].quantity, 1);
    assert_eq!(snapshot.subtotal, 10.0);

    // 再移除剩下的 1 个
    let remove_cmd2 = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::RemoveItem {
            order_id: order_id.clone(),
            instance_id: instance_id.clone(),
            quantity: None, // 移除全部
            reason: None,
            authorizer_id: None,
            authorizer_name: None,
        },
    );
    let resp = manager.execute_command(remove_cmd2);
    assert!(resp.success);

    let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert!(snapshot.items.is_empty() || snapshot.items.iter().all(|i| i.quantity == 0));
    assert_eq!(snapshot.subtotal, 0.0);
}


// ========================================================================
// ========================================================================
//  P1: 金额计算准确性测试
// ========================================================================
// ========================================================================

// ------------------------------------------------------------------------
// P1.1: 部分移除后金额重算
// ------------------------------------------------------------------------
#[test]
fn test_remove_item_partial_recalculates() {
    let manager = create_test_manager();

    let order_id = open_table_with_items(
        &manager,
        255,
        vec![simple_item(1, "Item", 10.0, 5)], // 50
    );

    let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
    let instance_id = snapshot.items[0].instance_id.clone();

    // 移除 2 个
    let remove_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::RemoveItem {
            order_id: order_id.clone(),
            instance_id,
            quantity: Some(2),
            reason: None,
            authorizer_id: None,
            authorizer_name: None,
        },
    );
    let resp = manager.execute_command(remove_cmd);
    assert!(resp.success);

    let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert_eq!(snapshot.items[0].quantity, 3);
    assert_eq!(snapshot.subtotal, 30.0);
    assert_eq!(snapshot.total, 30.0);
}


// ------------------------------------------------------------------------
// P1.2: 折扣 + 附加费叠加
// ------------------------------------------------------------------------
#[test]
fn test_discount_plus_surcharge_calculation() {
    let manager = create_test_manager();

    let order_id = open_table_with_items(
        &manager,
        256,
        vec![simple_item(1, "Item", 100.0, 1)],
    );

    // 应用 10% 折扣
    let discount_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::ApplyOrderDiscount {
            order_id: order_id.clone(),
            discount_percent: Some(10.0),
            discount_fixed: None,
            authorizer_id: None,
            authorizer_name: None,
        },
    );
    let resp = manager.execute_command(discount_cmd);
    assert!(resp.success);

    let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert_eq!(snapshot.order_manual_discount_percent, Some(10.0));
    assert_eq!(snapshot.order_manual_discount_amount, 10.0);

    // 应用 15 元附加费
    let surcharge_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::ApplyOrderSurcharge {
            order_id: order_id.clone(),
            surcharge_percent: None,
            surcharge_amount: Some(15.0),
            authorizer_id: None,
            authorizer_name: None,
        },
    );
    let resp = manager.execute_command(surcharge_cmd);
    assert!(resp.success);

    let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert_eq!(snapshot.order_manual_surcharge_fixed, Some(15.0));
    // total = 100 - 10 + 15 = 105
    assert_eq!(snapshot.total, 105.0);
}


// ------------------------------------------------------------------------
// P1.3: 商品级折扣 + 订单级折扣叠加
// ------------------------------------------------------------------------
#[test]
fn test_item_level_plus_order_level_discount() {
    let manager = create_test_manager();

    // 添加带 10% 手动折扣的商品
    let open_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::OpenTable {
            table_id: Some(326),
            table_name: Some("Table".to_string()),
            zone_id: None,
            zone_name: None,
            guest_count: 1,
            is_retail: false,
        },
    );
    let resp = manager.execute_command(open_cmd);
    let order_id = resp.order_id.unwrap();

    let add_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::AddItems {
            order_id: order_id.clone(),
            items: vec![item_with_discount(1, "Item", 100.0, 1, 10.0)], // 90 after item discount
        },
    );
    manager.execute_command(add_cmd);

    let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
    // 商品级折扣后 subtotal = 90
    assert_eq!(snapshot.subtotal, 90.0);

    // 应用 5% 订单级折扣
    let discount_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::ApplyOrderDiscount {
            order_id: order_id.clone(),
            discount_percent: Some(5.0),
            discount_fixed: None,
            authorizer_id: None,
            authorizer_name: None,
        },
    );
    let resp = manager.execute_command(discount_cmd);
    assert!(resp.success);

    let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
    // total = 90 - (90 * 5%) = 90 - 4.5 = 85.5
    assert_eq!(snapshot.total, 85.5);
}


// ------------------------------------------------------------------------
// P1.4: Comp 后 Uncomp 恢复价格
// ------------------------------------------------------------------------
#[test]
fn test_comp_then_uncomp_restores_price() {
    let manager = create_test_manager();

    let order_id = open_table_with_items(
        &manager,
        257,
        vec![simple_item(1, "Item", 25.0, 2)], // 50
    );

    let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
    let instance_id = snapshot.items[0].instance_id.clone();
    assert_eq!(snapshot.total, 50.0);

    // Comp 2 个
    let comp_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::CompItem {
            order_id: order_id.clone(),
            instance_id: instance_id.clone(),
            quantity: 2,
            reason: "Birthday gift".to_string(),
            authorizer_id: 1,
            authorizer_name: "Manager".to_string(),
        },
    );
    let resp = manager.execute_command(comp_cmd);
    assert!(resp.success);

    let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
    // Comp 后，原商品应该变成 comped，价格为 0
    let comped_item = snapshot.items.iter().find(|i| i.is_comped).unwrap();
    assert_eq!(comped_item.quantity, 2);
    assert_eq!(snapshot.total, 0.0);
    assert_eq!(snapshot.comp_total_amount, 50.0);

    // Uncomp
    let uncomp_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::UncompItem {
            order_id: order_id.clone(),
            instance_id: comped_item.instance_id.clone(),
            authorizer_id: 1,
            authorizer_name: "Manager".to_string(),
        },
    );
    let resp = manager.execute_command(uncomp_cmd);
    assert!(resp.success);

    let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
    // Uncomp 后价格恢复
    assert_eq!(snapshot.total, 50.0);
    assert_eq!(snapshot.comp_total_amount, 0.0);
}


// ------------------------------------------------------------------------
// P1.5: 部分 Comp 创建拆分商品
// ------------------------------------------------------------------------
#[test]
fn test_comp_partial_creates_split_item() {
    let manager = create_test_manager();

    let order_id = open_table_with_items(
        &manager,
        258,
        vec![simple_item(1, "Item", 10.0, 5)], // 50
    );

    let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
    let instance_id = snapshot.items[0].instance_id.clone();

    // Comp 2 个 (部分)
    let comp_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::CompItem {
            order_id: order_id.clone(),
            instance_id: instance_id.clone(),
            quantity: 2,
            reason: "Promotion".to_string(),
            authorizer_id: 1,
            authorizer_name: "Manager".to_string(),
        },
    );
    let resp = manager.execute_command(comp_cmd);
    assert!(resp.success);

    let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
    // 应该有 2 个商品项: 原始 3 个 + comped 2 个
    assert_eq!(snapshot.items.len(), 2);

    let normal_item = snapshot.items.iter().find(|i| !i.is_comped).unwrap();
    let comped_item = snapshot.items.iter().find(|i| i.is_comped).unwrap();

    assert_eq!(normal_item.quantity, 3);
    assert_eq!(comped_item.quantity, 2);
    assert!(comped_item.instance_id.contains("::comp::"));

    // total = 3 * 10 = 30 (comped 部分不计)
    assert_eq!(snapshot.total, 30.0);
    assert_eq!(snapshot.comp_total_amount, 20.0);
}


// ------------------------------------------------------------------------
// P1.6: 大金额精度测试
// ------------------------------------------------------------------------
#[test]
fn test_large_order_precision() {
    let manager = create_test_manager();

    let order_id = open_table_with_items(
        &manager,
        259,
        vec![simple_item(1, "Expensive", 99999.99, 100)],
    );

    let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
    // 99999.99 * 100 = 9999999.0
    assert_eq!(snapshot.subtotal, 9999999.0);
    assert_eq!(snapshot.total, 9999999.0);

    // 验证无精度丢失
    let expected = 99999.99 * 100.0;
    let diff = (snapshot.total - expected).abs();
    assert!(diff < 0.01, "Precision loss detected: expected {}, got {}", expected, snapshot.total);
}


// ------------------------------------------------------------------------
// P1.7: 选项价格修改器累加
// ------------------------------------------------------------------------
#[test]
fn test_option_price_modifiers_accumulate() {
    let manager = create_test_manager();

    let open_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::OpenTable {
            table_id: Some(327),
            table_name: Some("Table".to_string()),
            zone_id: None,
            zone_name: None,
            guest_count: 1,
            is_retail: false,
        },
    );
    let resp = manager.execute_command(open_cmd);
    let order_id = resp.order_id.unwrap();

    let add_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::AddItems {
            order_id: order_id.clone(),
            items: vec![item_with_options(
                1,
                "Pizza",
                15.0,
                1,
                vec![
                    shared::order::ItemOption {
                        attribute_id: 1,
                        attribute_name: "Size".to_string(),
                        option_id: 1,
                        option_name: "Large".to_string(),
                        price_modifier: Some(5.0),
                        quantity: 1,
                    },
                    shared::order::ItemOption {
                        attribute_id: 2,
                        attribute_name: "Topping".to_string(),
                        option_id: 0,
                        option_name: "Extra Cheese".to_string(),
                        price_modifier: Some(2.5),
                        quantity: 1,
                    },
                ],
            )],
        },
    );
    let resp = manager.execute_command(add_cmd);
    assert!(resp.success);

    let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
    // 15 + 5 + 2.5 = 22.5
    assert_eq!(snapshot.subtotal, 22.5);
}


// ========================================================================
// ========================================================================
//  P2: 状态转换边界测试
// ========================================================================
// ========================================================================

// ------------------------------------------------------------------------
// P2.1: 已完成订单拒绝所有修改命令
// ------------------------------------------------------------------------
#[test]
fn test_all_commands_reject_completed_order() {
    let manager = create_test_manager();

    let order_id = open_table_with_items(
        &manager,
        260,
        vec![simple_item(1, "Item", 10.0, 1)],
    );

    // 支付并完成
    pay(&manager, &order_id, 10.0, "CASH");
    complete_order(&manager, &order_id);
    assert_order_status(&manager, &order_id, OrderStatus::Completed);

    // 测试 AddItems
    let add_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::AddItems {
            order_id: order_id.clone(),
            items: vec![simple_item(2, "New Item", 5.0, 1)],
        },
    );
    let resp = manager.execute_command(add_cmd);
    assert!(!resp.success, "AddItems should fail on completed order");

    // 测试 AddPayment
    let pay_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::AddPayment {
            order_id: order_id.clone(),
            payment: PaymentInput {
                method: "CASH".to_string(),
                amount: 5.0,
                tendered: None,
                note: None,
            },
        },
    );
    let resp = manager.execute_command(pay_cmd);
    assert!(!resp.success, "AddPayment should fail on completed order");

    // 测试 VoidOrder
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
    assert!(!resp.success, "VoidOrder should fail on completed order");

    // 测试 MoveOrder
    let move_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::MoveOrder {
            order_id: order_id.clone(),
            target_table_id: 332,
            target_table_name: "New Table".to_string(),
            target_zone_id: None,
            target_zone_name: None,
            authorizer_id: None,
            authorizer_name: None,
        },
    );
    let resp = manager.execute_command(move_cmd);
    assert!(!resp.success, "MoveOrder should fail on completed order");
}


// ------------------------------------------------------------------------
// P2.2: 已作废订单拒绝所有修改命令
// ------------------------------------------------------------------------
#[test]
fn test_all_commands_reject_voided_order() {
    let manager = create_test_manager();

    let order_id = open_table_with_items(
        &manager,
        261,
        vec![simple_item(1, "Item", 10.0, 1)],
    );

    // 作废订单
    void_order_helper(&manager, &order_id, VoidType::Cancelled);
    assert_order_status(&manager, &order_id, OrderStatus::Void);

    // 测试 AddItems
    let add_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::AddItems {
            order_id: order_id.clone(),
            items: vec![simple_item(2, "New Item", 5.0, 1)],
        },
    );
    let resp = manager.execute_command(add_cmd);
    assert!(!resp.success, "AddItems should fail on voided order");

    // 测试 AddPayment
    let pay_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::AddPayment {
            order_id: order_id.clone(),
            payment: PaymentInput {
                method: "CASH".to_string(),
                amount: 5.0,
                tendered: None,
                note: None,
            },
        },
    );
    let resp = manager.execute_command(pay_cmd);
    assert!(!resp.success, "AddPayment should fail on voided order");

    // 测试 CompleteOrder
    let complete_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::CompleteOrder {
            order_id: order_id.clone(),
            service_type: Some(ServiceType::DineIn),
        },
    );
    let resp = manager.execute_command(complete_cmd);
    assert!(!resp.success, "CompleteOrder should fail on voided order");
}


// ------------------------------------------------------------------------
// P2.3: 已合并订单 - 验证合并后源订单状态
// 注意: 当前实现对 Merged 状态订单不检查 AddItems，但订单已不在活跃列表
// ------------------------------------------------------------------------
#[test]
fn test_merged_order_not_in_active_list() {
    let manager = create_test_manager();

    let source_id = open_table_with_items(
        &manager,
        262,
        vec![simple_item(1, "Item", 10.0, 1)],
    );
    let target_id = open_table_with_items(
        &manager,
        263,
        vec![simple_item(2, "Item 2", 10.0, 1)],
    );

    // 合并前两个订单都在活跃列表
    let active = manager.get_active_orders().unwrap();
    assert_eq!(active.len(), 2);

    // 合并
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
    manager.execute_command(merge_cmd);
    assert_order_status(&manager, &source_id, OrderStatus::Merged);

    // 合并后源订单不在活跃列表
    let active = manager.get_active_orders().unwrap();
    assert_eq!(active.len(), 1);
    assert!(active.iter().all(|o| o.order_id != source_id), "Merged order should not be in active list");
    assert!(active.iter().any(|o| o.order_id == target_id), "Target order should be in active list");
}


// ------------------------------------------------------------------------
// P2.4: UpdateOrderInfo 不影响金额
// ------------------------------------------------------------------------
#[test]
fn test_update_guest_count_does_not_affect_totals() {
    let manager = create_test_manager();

    let order_id = open_table_with_items(
        &manager,
        264,
        vec![simple_item(1, "Item", 100.0, 1)],
    );

    let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert_eq!(snapshot.guest_count, 2); // 默认值
    assert_eq!(snapshot.total, 100.0);

    // 更新 guest_count
    let update_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::UpdateOrderInfo {
            order_id: order_id.clone(),
            guest_count: Some(8),
            table_name: None,
            is_pre_payment: None,
        },
    );
    let resp = manager.execute_command(update_cmd);
    assert!(resp.success);

    let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert_eq!(snapshot.guest_count, 8);
    assert_eq!(snapshot.total, 100.0, "Total should not change after updating guest_count");
}


// ------------------------------------------------------------------------
// P2.5: AddOrderNote 覆盖之前的备注
// ------------------------------------------------------------------------
#[test]
fn test_add_note_overwrites_previous() {
    let manager = create_test_manager();

    let order_id = open_table_with_items(&manager, 109, vec![]);

    // 添加第一个备注
    let note_cmd1 = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::AddOrderNote {
            order_id: order_id.clone(),
            note: "First note".to_string(),
        },
    );
    manager.execute_command(note_cmd1);

    let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert_eq!(snapshot.note, Some("First note".to_string()));

    // 添加第二个备注 (应覆盖)
    let note_cmd2 = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::AddOrderNote {
            order_id: order_id.clone(),
            note: "Second note".to_string(),
        },
    );
    manager.execute_command(note_cmd2);

    let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert_eq!(snapshot.note, Some("Second note".to_string()));
}


// ------------------------------------------------------------------------
// P2.6: 空字符串清除备注
// ------------------------------------------------------------------------
#[test]
fn test_clear_note_with_empty_string() {
    let manager = create_test_manager();

    let order_id = open_table_with_items(&manager, 110, vec![]);

    // 添加备注
    let note_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::AddOrderNote {
            order_id: order_id.clone(),
            note: "Some note".to_string(),
        },
    );
    manager.execute_command(note_cmd);

    let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert_eq!(snapshot.note, Some("Some note".to_string()));

    // 用空字符串清除
    let clear_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::AddOrderNote {
            order_id: order_id.clone(),
            note: String::new(),
        },
    );
    manager.execute_command(clear_cmd);

    let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert!(snapshot.note.is_none() || snapshot.note.as_deref() == Some(""));
}


// ========================================================================
// ========================================================================
//  P3: 边界条件与错误处理测试
// ========================================================================
// ========================================================================

// ------------------------------------------------------------------------
// P3.1: 支付→取消→支付循环
// ------------------------------------------------------------------------
#[test]
fn test_add_cancel_add_payment_cycle() {
    let manager = create_test_manager();

    let order_id = open_table_with_items(
        &manager,
        265,
        vec![simple_item(1, "Item", 30.0, 1)],
    );

    // 第一次支付
    pay(&manager, &order_id, 30.0, "CARD");
    let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
    let payment1_id = snapshot.payments[0].payment_id.clone();
    assert_eq!(snapshot.paid_amount, 30.0);

    // 取消第一次支付
    let cancel_cmd1 = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::CancelPayment {
            order_id: order_id.clone(),
            payment_id: payment1_id,
            reason: Some("Wrong card".to_string()),
            authorizer_id: None,
            authorizer_name: None,
        },
    );
    let resp = manager.execute_command(cancel_cmd1);
    assert!(resp.success);

    let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert_eq!(snapshot.paid_amount, 0.0);

    // 第二次支付
    pay(&manager, &order_id, 30.0, "CASH");
    let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
    let payment2_id = snapshot.payments.iter().find(|p| !p.cancelled).unwrap().payment_id.clone();
    assert_eq!(snapshot.paid_amount, 30.0);

    // 取消第二次支付
    let cancel_cmd2 = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::CancelPayment {
            order_id: order_id.clone(),
            payment_id: payment2_id,
            reason: Some("Customer changed mind".to_string()),
            authorizer_id: None,
            authorizer_name: None,
        },
    );
    let resp = manager.execute_command(cancel_cmd2);
    assert!(resp.success);

    // 第三次支付
    pay(&manager, &order_id, 30.0, "CARD");

    let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert_eq!(snapshot.payments.len(), 3);
    assert_eq!(
        snapshot.payments.iter().filter(|p| p.cancelled).count(),
        2,
        "Should have 2 cancelled payments"
    );
    assert_eq!(snapshot.paid_amount, 30.0);

    // 完成订单
    let complete_resp = complete_order(&manager, &order_id);
    assert!(complete_resp.success);
}


// ------------------------------------------------------------------------
// P3.2: AA 分单不能与菜品分单混用
// ------------------------------------------------------------------------
#[test]
fn test_aa_split_cannot_mix_with_item_split() {
    let manager = create_test_manager();

    let order_id = open_table_with_items(
        &manager,
        266,
        vec![simple_item(1, "Item", 100.0, 2)],
    );

    let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
    let instance_id = snapshot.items[0].instance_id.clone();

    // 开始 AA 分单
    let start_aa = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::StartAaSplit {
            order_id: order_id.clone(),
            total_shares: 2,
            shares: 1,
            payment_method: "CASH".to_string(),
            tendered: Some(100.0),
        },
    );
    let resp = manager.execute_command(start_aa);
    assert!(resp.success);

    // 尝试菜品分单应该失败
    let item_split_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::SplitByItems {
            order_id: order_id.clone(),
            payment_method: "CARD".to_string(),
            items: vec![shared::order::SplitItem {
                instance_id,
                name: "Item".to_string(),
                quantity: 1,
                unit_price: 100.0,
            }],
            tendered: None,
        },
    );
    let resp = manager.execute_command(item_split_cmd);
    assert!(!resp.success, "Item split should fail when AA split is active");
}


// ------------------------------------------------------------------------
// P3.3: Comp 后支付再完成
// ------------------------------------------------------------------------
#[test]
fn test_comp_then_pay_then_complete() {
    let manager = create_test_manager();

    let order_id = open_table_with_items(
        &manager,
        267,
        vec![
            simple_item(1, "Item A", 10.0, 1),
            simple_item(2, "Item B", 10.0, 1),
        ],
    );

    let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
    let item_a_id = snapshot.items.iter().find(|i| i.name == "Item A").unwrap().instance_id.clone();
    assert_eq!(snapshot.total, 20.0);

    // Comp Item A
    let comp_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::CompItem {
            order_id: order_id.clone(),
            instance_id: item_a_id,
            quantity: 1,
            reason: "Gift".to_string(),
            authorizer_id: 1,
            authorizer_name: "Manager".to_string(),
        },
    );
    let resp = manager.execute_command(comp_cmd);
    assert!(resp.success);

    let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
    // total 应该只包含 Item B: 10.0
    assert_eq!(snapshot.total, 10.0);

    // 支付 10.0
    pay(&manager, &order_id, 10.0, "CASH");

    // 完成
    let complete_resp = complete_order(&manager, &order_id);
    assert!(complete_resp.success);

    let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert_eq!(snapshot.status, OrderStatus::Completed);
}


// ------------------------------------------------------------------------
// P3.4: 金额分单后菜品分单被禁用
// ------------------------------------------------------------------------
#[test]
fn test_amount_split_blocks_item_split() {
    let manager = create_test_manager();

    let order_id = open_table_with_items(
        &manager,
        268,
        vec![simple_item(1, "Item", 100.0, 2)],
    );

    let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
    let instance_id = snapshot.items[0].instance_id.clone();

    // 金额分单
    let amount_split = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::SplitByAmount {
            order_id: order_id.clone(),
            split_amount: 50.0,
            payment_method: "CASH".to_string(),
            tendered: Some(50.0),
        },
    );
    let resp = manager.execute_command(amount_split);
    assert!(resp.success);

    let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert!(snapshot.has_amount_split);

    // 尝试菜品分单应该失败
    let item_split = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::SplitByItems {
            order_id: order_id.clone(),
            payment_method: "CARD".to_string(),
            items: vec![shared::order::SplitItem {
                instance_id,
                name: "Item".to_string(),
                quantity: 1,
                unit_price: 100.0,
            }],
            tendered: None,
        },
    );
    let resp = manager.execute_command(item_split);
    assert!(!resp.success, "Item split should be blocked when amount split is active");
}


// ------------------------------------------------------------------------
// P3.6: 取消不存在的支付应失败
// ------------------------------------------------------------------------
#[test]
fn test_cancel_nonexistent_payment_fails() {
    let manager = create_test_manager();

    let order_id = open_table_with_items(
        &manager,
        269,
        vec![simple_item(1, "Item", 10.0, 1)],
    );

    let cancel_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::CancelPayment {
            order_id: order_id.clone(),
            payment_id: "nonexistent-payment-id".to_string(),
            reason: None,
            authorizer_id: None,
            authorizer_name: None,
        },
    );
    let resp = manager.execute_command(cancel_cmd);
    assert!(!resp.success, "CancelPayment should fail for nonexistent payment");
}


// ------------------------------------------------------------------------
// P3.7: 超额菜品分单应失败
// ------------------------------------------------------------------------
#[test]
fn test_split_by_items_overpay_fails() {
    let manager = create_test_manager();

    let order_id = open_table_with_items(
        &manager,
        270,
        vec![simple_item(1, "Item", 10.0, 1)],
    );

    let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
    let instance_id = snapshot.items[0].instance_id.clone();

    // 尝试支付超过可用数量
    let split_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::SplitByItems {
            order_id: order_id.clone(),
            payment_method: "CASH".to_string(),
            items: vec![shared::order::SplitItem {
                instance_id,
                name: "Item".to_string(),
                quantity: 5, // 订单只有 1 个
                unit_price: 10.0,
            }],
            tendered: Some(50.0),
        },
    );
    let resp = manager.execute_command(split_cmd);
    assert!(!resp.success, "Split with excessive quantity should fail");
}


// ------------------------------------------------------------------------
// P3.8: 移除超过现有数量应失败
// ------------------------------------------------------------------------
#[test]
fn test_remove_item_excessive_quantity_fails() {
    let manager = create_test_manager();

    let order_id = open_table_with_items(
        &manager,
        271,
        vec![simple_item(1, "Item", 10.0, 2)],
    );

    let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
    let instance_id = snapshot.items[0].instance_id.clone();

    // 尝试移除 5 个，但只有 2 个
    let remove_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::RemoveItem {
            order_id: order_id.clone(),
            instance_id,
            quantity: Some(5),
            reason: None,
            authorizer_id: None,
            authorizer_name: None,
        },
    );
    let resp = manager.execute_command(remove_cmd);
    assert!(!resp.success, "Remove with excessive quantity should fail");
}


// ------------------------------------------------------------------------
// P3.9: Comp 超过现有数量应失败
// ------------------------------------------------------------------------
#[test]
fn test_comp_item_excessive_quantity_fails() {
    let manager = create_test_manager();

    let order_id = open_table_with_items(
        &manager,
        272,
        vec![simple_item(1, "Item", 10.0, 2)],
    );

    let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
    let instance_id = snapshot.items[0].instance_id.clone();

    // 尝试 comp 5 个，但只有 2 个
    let comp_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::CompItem {
            order_id: order_id.clone(),
            instance_id,
            quantity: 5,
            reason: "Test".to_string(),
            authorizer_id: 1,
            authorizer_name: "Manager".to_string(),
        },
    );
    let resp = manager.execute_command(comp_cmd);
    assert!(!resp.success, "Comp with excessive quantity should fail");
}


// ------------------------------------------------------------------------
// P3.10: 清除整单折扣
// ------------------------------------------------------------------------
#[test]
fn test_clear_order_discount() {
    let manager = create_test_manager();

    let order_id = open_table_with_items(
        &manager,
        273,
        vec![simple_item(1, "Item", 100.0, 1)],
    );

    // 应用折扣
    let discount_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::ApplyOrderDiscount {
            order_id: order_id.clone(),
            discount_percent: Some(20.0),
            discount_fixed: None,
            authorizer_id: None,
            authorizer_name: None,
        },
    );
    manager.execute_command(discount_cmd);

    let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert_eq!(snapshot.total, 80.0);

    // 清除折扣 (两个参数都为 None)
    let clear_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::ApplyOrderDiscount {
            order_id: order_id.clone(),
            discount_percent: None,
            discount_fixed: None,
            authorizer_id: None,
            authorizer_name: None,
        },
    );
    let resp = manager.execute_command(clear_cmd);
    assert!(resp.success);

    let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert_eq!(snapshot.total, 100.0);
    assert!(snapshot.order_manual_discount_percent.is_none());
}


// ------------------------------------------------------------------------
// P3.11: ToggleRuleSkip - 规则不存在时应失败
// 注意: ToggleRuleSkip 需要订单中有 applied_rules，否则会失败
// ------------------------------------------------------------------------
#[test]
fn test_toggle_rule_skip_nonexistent_rule_fails() {
    let manager = create_test_manager();

    let order_id = open_table_with_items(
        &manager,
        274,
        vec![simple_item(1, "Item", 100.0, 1)],
    );

    // 尝试 toggle 不存在的规则应失败
    let toggle_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::ToggleRuleSkip {
            order_id: order_id.clone(),
            rule_id: 99999,
            skipped: true,
        },
    );
    let resp = manager.execute_command(toggle_cmd);
    assert!(!resp.success, "ToggleRuleSkip should fail when rule not found");
}


// ------------------------------------------------------------------------
// P3.12: 固定金额折扣
// ------------------------------------------------------------------------
#[test]
fn test_fixed_amount_discount() {
    let manager = create_test_manager();

    let order_id = open_table_with_items(
        &manager,
        275,
        vec![simple_item(1, "Item", 100.0, 1)],
    );

    // 应用 25 元固定折扣
    let discount_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::ApplyOrderDiscount {
            order_id: order_id.clone(),
            discount_percent: None,
            discount_fixed: Some(25.0),
            authorizer_id: None,
            authorizer_name: None,
        },
    );
    let resp = manager.execute_command(discount_cmd);
    assert!(resp.success);

    let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert_eq!(snapshot.order_manual_discount_fixed, Some(25.0));
    assert_eq!(snapshot.total, 75.0);
}


// ------------------------------------------------------------------------
// P3.13: 百分比附加费
// ------------------------------------------------------------------------
#[test]
fn test_percentage_surcharge() {
    let manager = create_test_manager();

    let order_id = open_table_with_items(
        &manager,
        276,
        vec![simple_item(1, "Item", 100.0, 1)],
    );

    // 应用 10% 附加费
    let surcharge_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::ApplyOrderSurcharge {
            order_id: order_id.clone(),
            surcharge_percent: Some(10.0),
            surcharge_amount: None,
            authorizer_id: None,
            authorizer_name: None,
        },
    );
    let resp = manager.execute_command(surcharge_cmd);
    assert!(resp.success);

    let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert_eq!(snapshot.order_manual_surcharge_percent, Some(10.0));
    assert_eq!(snapshot.total, 110.0);
}

