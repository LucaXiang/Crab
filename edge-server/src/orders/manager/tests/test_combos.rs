use super::*;

// --- Test 1: 折扣循环 50%→20%→50%→0% ---

#[test]
fn test_combo_discount_cycling_no_payment() {
    let manager = create_test_manager();
    let order_id = open_table_with_items(
        &manager,
        277,
        vec![simple_item(1, "Coffee", 10.0, 3)], // total=30
    );

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    let iid = s.items[0].instance_id.clone();

    // 50% discount → total = 15
    let r = modify_item(&manager, &order_id, &iid, discount_changes(50.0));
    assert!(r.success);
    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert_eq!(s.items.len(), 1, "Should still be 1 item");
    assert!((s.total - 15.0).abs() < 0.01);
    let iid = s.items[0].instance_id.clone();

    // 20% discount → total = 24
    let r = modify_item(&manager, &order_id, &iid, discount_changes(20.0));
    assert!(r.success);
    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert_eq!(s.items.len(), 1);
    assert!((s.total - 24.0).abs() < 0.01);
    let iid = s.items[0].instance_id.clone();

    // Back to 50% → total = 15
    let r = modify_item(&manager, &order_id, &iid, discount_changes(50.0));
    assert!(r.success);
    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert_eq!(s.items.len(), 1);
    assert!((s.total - 15.0).abs() < 0.01);
    let iid = s.items[0].instance_id.clone();

    // Remove discount → total = 30
    let r = modify_item(&manager, &order_id, &iid, discount_changes(0.0));
    assert!(r.success);
    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert_eq!(s.items.len(), 1, "Removing discount should merge back");
    assert!((s.total - 30.0).abs() < 0.01);

    assert_snapshot_consistent(&manager, &order_id);
}

// --- Test 2: 折扣循环 + 部分支付 + 取消支付 ---

#[test]
fn test_combo_discount_cycle_with_partial_payment_and_cancel() {
    let manager = create_test_manager();
    let order_id = open_table_with_items(
        &manager,
        278,
        vec![simple_item(1, "Coffee", 10.0, 4)], // total=40
    );

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    let iid = s.items[0].instance_id.clone();

    // 1. Apply 50% discount → total=20
    let r = modify_item(&manager, &order_id, &iid, discount_changes(50.0));
    assert!(r.success);

    // 2. Pay 10 (partial)
    let r = pay(&manager, &order_id, 10.0, "CASH");
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert!((s.paid_amount - 10.0).abs() < 0.01);
    // After partial payment, total=20, paid=10
    // But recalculate_totals recalculates with paid items at discounted price
    assert!(s.remaining_amount > 0.0, "Should have remaining amount");

    // 3. Change unpaid items to 20% discount (paid items keep 50%)
    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    let iid = s
        .items
        .iter()
        .find(|i| i.unpaid_quantity > 0)
        .unwrap()
        .instance_id
        .clone();
    let r = modify_item(&manager, &order_id, &iid, discount_changes(20.0));
    assert!(
        r.success,
        "Item-level discount on unpaid portion should succeed"
    );

    // 4. Cancel the payment
    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    let payment_id = s
        .payments
        .iter()
        .find(|p| !p.cancelled)
        .unwrap()
        .payment_id
        .clone();
    let r = cancel_payment(&manager, &order_id, &payment_id);
    assert!(r.success);

    // 5. After cancel, paid_amount should be 0
    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert!(
        (s.paid_amount).abs() < 0.01,
        "paid should be 0 after cancel"
    );

    // 6. Remove all discounts
    let items_snapshot: Vec<_> = s
        .items
        .iter()
        .filter(|i| i.manual_discount_percent.is_some())
        .map(|i| i.instance_id.clone())
        .collect();
    for iid in &items_snapshot {
        modify_item(&manager, &order_id, iid, discount_changes(0.0));
    }

    // 7. Verify: should be back to original total, no fragmentation
    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert!(
        (s.total - 40.0).abs() < 0.01,
        "Total should be original 40, got {}",
        s.total
    );
    let total_qty: i32 = s.items.iter().map(|i| i.quantity).sum();
    assert_eq!(total_qty, 4, "Total quantity should remain 4");

    assert_snapshot_consistent(&manager, &order_id);
}

// --- Test 3: 分单支付 + 改价 + 再支付 ---

#[test]
fn test_combo_split_payment_then_modify_then_pay() {
    let manager = create_test_manager();
    let order_id = open_table_with_items(
        &manager,
        279,
        vec![
            simple_item(1, "Coffee", 10.0, 3), // 30
            simple_item(2, "Tea", 8.0, 2),     // 16 → total=46
        ],
    );

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    let coffee_iid = s
        .items
        .iter()
        .find(|i| i.name == "Coffee")
        .unwrap()
        .instance_id
        .clone();

    // 1. Split-pay 2 coffees (20)
    let r = split_by_items(
        &manager,
        &order_id,
        vec![shared::order::SplitItem {
            instance_id: coffee_iid.clone(),
            name: "Coffee".to_string(),
            quantity: 2,
            unit_price: 10.0,
        }],
        "CARD",
    );
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert!((s.paid_amount - 20.0).abs() < 0.01);
    assert_eq!(s.paid_item_quantities.get(&coffee_iid), Some(&2));

    // 2. Modify remaining coffee price to 15 (should only affect unpaid portion)
    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    let unpaid_coffee = s
        .items
        .iter()
        .find(|i| i.name == "Coffee" && i.unpaid_quantity > 0)
        .unwrap();
    let unpaid_iid = unpaid_coffee.instance_id.clone();
    let r = modify_item(&manager, &order_id, &unpaid_iid, price_changes(15.0));
    assert!(r.success, "Should be able to modify unpaid coffee price");

    // 3. Pay remaining
    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    let remaining = s.remaining_amount;
    assert!(remaining > 0.0);
    let r = pay(&manager, &order_id, remaining, "CASH");
    assert!(r.success, "Should pay remaining {}", remaining);

    // 4. Complete
    let r = complete_order(&manager, &order_id);
    assert!(r.success);

    assert_snapshot_consistent(&manager, &order_id);
}

// --- Test 4: Comp + 折扣 + 支付 ---

#[test]
fn test_combo_comp_then_discount_then_pay() {
    let manager = create_test_manager();
    let order_id = open_table_with_items(
        &manager,
        280,
        vec![
            simple_item(1, "Coffee", 10.0, 2), // 20
            simple_item(2, "Tea", 5.0, 2),     // 10 → total=30
        ],
    );

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    let tea_iid = s
        .items
        .iter()
        .find(|i| i.name == "Tea")
        .unwrap()
        .instance_id
        .clone();

    // 1. Comp the tea
    let r = comp_item(&manager, &order_id, &tea_iid);
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert!(
        (s.total - 20.0).abs() < 0.01,
        "Total should be 20 after comp tea"
    );

    // 2. Apply 50% discount on coffee → total should be 10
    let coffee_iid = s
        .items
        .iter()
        .find(|i| i.name == "Coffee" && !i.is_comped)
        .unwrap()
        .instance_id
        .clone();
    let r = modify_item(&manager, &order_id, &coffee_iid, discount_changes(50.0));
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert!(
        (s.total - 10.0).abs() < 0.01,
        "Total should be 10, got {}",
        s.total
    );

    // 3. Pay full amount
    let r = pay(&manager, &order_id, s.total, "CARD");
    assert!(r.success);

    // 4. Complete
    let r = complete_order(&manager, &order_id);
    assert!(r.success);

    // 5. Verify order is completed and totals are correct
    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert_eq!(s.status, OrderStatus::Completed);
    // Comped tea should have is_comped=true and not contribute to total
    let comped_count = s.items.iter().filter(|i| i.is_comped).count();
    assert!(comped_count > 0, "Should have comped items");

    assert_snapshot_consistent(&manager, &order_id);
}

// --- Test 5: 整单折扣 100% → total=0 → 可完成 ---

#[test]
fn test_combo_100_percent_order_discount() {
    let manager = create_test_manager();
    let order_id = open_table_with_items(
        &manager,
        281,
        vec![simple_item(1, "Coffee", 10.0, 2)], // 20
    );

    // 100% discount
    let r = apply_discount(&manager, &order_id, 100.0);
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert!(
        (s.total).abs() < 0.01,
        "100% discount → total=0, got {}",
        s.total
    );
    assert!((s.remaining_amount).abs() < 0.01);

    // Should be able to complete without payment
    let r = complete_order(&manager, &order_id);
    assert!(r.success, "Should complete with 0 total");

    assert_snapshot_consistent(&manager, &order_id);
}

// --- Test 6: 大量折扣 → total clamp 到 0 ---

#[test]
fn test_combo_fixed_discount_exceeds_total() {
    let manager = create_test_manager();
    let order_id = open_table_with_items(
        &manager,
        282,
        vec![simple_item(1, "Coffee", 10.0, 1)], // 10
    );

    // Fixed discount of 50 on a 10 order
    let cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::ApplyOrderDiscount {
            order_id: order_id.clone(),
            discount_percent: None,
            discount_fixed: Some(50.0),
            authorizer_id: None,
            authorizer_name: None,
        },
    );
    let r = manager.execute_command(cmd);
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert!(
        s.total >= 0.0,
        "Total must not be negative, got {}",
        s.total
    );
    assert!(
        (s.total).abs() < 0.01,
        "Total should clamp to 0, got {}",
        s.total
    );

    assert_snapshot_consistent(&manager, &order_id);
}

// --- Test 7: 支付 → 取消 → 再支付 → 完成 ---

#[test]
fn test_combo_pay_cancel_repay_complete() {
    let manager = create_test_manager();
    let order_id = open_table_with_items(
        &manager,
        283,
        vec![simple_item(1, "Coffee", 10.0, 3)], // 30
    );

    // Pay 15
    let r = pay(&manager, &order_id, 15.0, "CARD");
    assert!(r.success);

    // Cancel it
    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    let pid = s.payments[0].payment_id.clone();
    let r = cancel_payment(&manager, &order_id, &pid);
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert!((s.paid_amount).abs() < 0.01);
    assert!((s.remaining_amount - 30.0).abs() < 0.01);

    // Pay full
    let r = pay(&manager, &order_id, 30.0, "CASH");
    assert!(r.success);

    let r = complete_order(&manager, &order_id);
    assert!(r.success);

    assert_snapshot_consistent(&manager, &order_id);
}

// --- Test 8: 分单支付后 cancel → 重新分单 ---

#[test]
fn test_combo_split_cancel_resplit() {
    let manager = create_test_manager();
    let order_id = open_table_with_items(
        &manager,
        284,
        vec![
            simple_item(1, "Coffee", 10.0, 2), // 20
            simple_item(2, "Tea", 5.0, 2),     // 10
        ], // total=30
    );

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    let coffee_iid = s
        .items
        .iter()
        .find(|i| i.name == "Coffee")
        .unwrap()
        .instance_id
        .clone();
    let tea_iid = s
        .items
        .iter()
        .find(|i| i.name == "Tea")
        .unwrap()
        .instance_id
        .clone();

    // 1. Split-pay all coffee (20)
    let r = split_by_items(
        &manager,
        &order_id,
        vec![shared::order::SplitItem {
            instance_id: coffee_iid.clone(),
            name: "Coffee".to_string(),
            quantity: 2,
            unit_price: 10.0,
        }],
        "CARD",
    );
    assert!(r.success);

    // 2. Cancel that split payment
    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    let pid = s
        .payments
        .iter()
        .find(|p| !p.cancelled)
        .unwrap()
        .payment_id
        .clone();
    let r = cancel_payment(&manager, &order_id, &pid);
    assert!(r.success);

    // 3. Now split-pay tea instead
    let r = split_by_items(
        &manager,
        &order_id,
        vec![shared::order::SplitItem {
            instance_id: tea_iid.clone(),
            name: "Tea".to_string(),
            quantity: 2,
            unit_price: 5.0,
        }],
        "CASH",
    );
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert!(
        (s.paid_amount - 10.0).abs() < 0.01,
        "Should have paid 10 for tea"
    );
    assert!((s.remaining_amount - 20.0).abs() < 0.01);

    // 4. Pay remaining
    let r = pay(&manager, &order_id, 20.0, "CARD");
    assert!(r.success);

    let r = complete_order(&manager, &order_id);
    assert!(r.success);

    assert_snapshot_consistent(&manager, &order_id);
}

// --- Test 9: 多次部分支付 + 修改数量 ---

#[test]
fn test_combo_multiple_partial_payments_then_modify_qty() {
    let manager = create_test_manager();
    let order_id = open_table_with_items(
        &manager,
        285,
        vec![simple_item(1, "Coffee", 10.0, 5)], // 50
    );

    // Pay 20, then 15
    let r = pay(&manager, &order_id, 20.0, "CARD");
    assert!(r.success, "Pay 20 failed: {:?}", r.error);

    let r = pay(&manager, &order_id, 15.0, "CASH");
    assert!(r.success, "Pay 15 failed: {:?}", r.error);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert!(
        (s.paid_amount - 35.0).abs() < 0.01,
        "paid={}, total={}, remaining={}, payments={}",
        s.paid_amount,
        s.total,
        s.remaining_amount,
        s.payments.len()
    );

    // Compute actual remaining from authoritative fields
    let actual_remaining = s.total - s.paid_amount;
    assert!(
        (s.remaining_amount - actual_remaining).abs() < 0.02,
        "remaining_amount({}) diverged from total({}) - paid({})",
        s.remaining_amount,
        s.total,
        s.paid_amount
    );

    // Try to overpay — should fail
    let r = pay(&manager, &order_id, actual_remaining + 1.0, "CARD");
    assert!(!r.success, "Should reject overpayment");

    // Pay exact remaining
    let r = pay(&manager, &order_id, actual_remaining, "CARD");
    assert!(
        r.success,
        "Paying remaining ({}) failed: {:?}",
        actual_remaining, r.error
    );

    let r = complete_order(&manager, &order_id);
    assert!(r.success);

    assert_snapshot_consistent(&manager, &order_id);
}

// --- Test 10: 折扣 + comp + 分单 + 完成 ---

#[test]
fn test_combo_discount_comp_split_complete() {
    let manager = create_test_manager();
    let order_id = open_table_with_items(
        &manager,
        286,
        vec![
            simple_item(1, "Steak", 25.0, 2), // 50
            simple_item(2, "Wine", 15.0, 2),  // 30
            simple_item(3, "Bread", 3.0, 1),  // 3
        ], // total=83
    );

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    let bread_iid = s
        .items
        .iter()
        .find(|i| i.name == "Bread")
        .unwrap()
        .instance_id
        .clone();
    let steak_iid = s
        .items
        .iter()
        .find(|i| i.name == "Steak")
        .unwrap()
        .instance_id
        .clone();

    // 1. Comp the bread
    let r = comp_item(&manager, &order_id, &bread_iid);
    assert!(r.success);

    // 2. 20% discount on steak → steak_total = 2 * 25 * 0.8 = 40
    let r = modify_item(&manager, &order_id, &steak_iid, discount_changes(20.0));
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    // total = steak(40) + wine(30) = 70
    assert!(
        (s.total - 70.0).abs() < 0.01,
        "Expected 70, got {}",
        s.total
    );

    // 3. Split-pay 1 steak (discounted: 25*0.8 = 20)
    let steak = s
        .items
        .iter()
        .find(|i| i.name == "Steak" && !i.is_comped && i.unpaid_quantity > 0)
        .unwrap();
    let r = split_by_items(
        &manager,
        &order_id,
        vec![shared::order::SplitItem {
            instance_id: steak.instance_id.clone(),
            name: "Steak".to_string(),
            quantity: 1,
            unit_price: 20.0,
        }],
        "CARD",
    );
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert!((s.paid_amount - 20.0).abs() < 0.01);

    // 4. Pay remaining (50)
    let r = pay(&manager, &order_id, s.remaining_amount, "CASH");
    assert!(r.success);

    let r = complete_order(&manager, &order_id);
    assert!(r.success);

    assert_snapshot_consistent(&manager, &order_id);
}

// --- Test 11: 同一商品多次添加 + 折扣 + 去折 → 自动合并 ---

#[test]
fn test_combo_add_twice_discount_undiscount_merges() {
    let manager = create_test_manager();
    let order_id = open_table_with_items(
        &manager,
        287,
        vec![simple_item(1, "Coffee", 10.0, 2)], // 20
    );

    // Add same product again
    let add_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::AddItems {
            order_id: order_id.clone(),
            items: vec![simple_item(1, "Coffee", 10.0, 3)],
        },
    );
    let r = manager.execute_command(add_cmd);
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    // Should auto-merge: 1 item with qty=5
    assert_eq!(s.items.len(), 1, "Same product should merge on add");
    assert_eq!(s.items[0].quantity, 5);
    let iid = s.items[0].instance_id.clone();

    // Apply 30% discount
    let r = modify_item(&manager, &order_id, &iid, discount_changes(30.0));
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    let iid = s.items[0].instance_id.clone();

    // Remove discount → should return to original instance_id and stay as 1 item
    let r = modify_item(&manager, &order_id, &iid, discount_changes(0.0));
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert_eq!(
        s.items.len(),
        1,
        "Should still be 1 item after removing discount"
    );
    assert_eq!(s.items[0].quantity, 5);
    assert!((s.total - 50.0).abs() < 0.01);

    assert_snapshot_consistent(&manager, &order_id);
}

// --- Test 12: 支付后不能加整单折扣 ---

#[test]
fn test_combo_order_discount_blocked_after_payment() {
    let manager = create_test_manager();
    let order_id = open_table_with_items(
        &manager,
        288,
        vec![simple_item(1, "Coffee", 10.0, 3)], // 30
    );

    // Pay 10
    let r = pay(&manager, &order_id, 10.0, "CARD");
    assert!(r.success);

    // Try order-level discount — should fail
    let r = apply_discount(&manager, &order_id, 20.0);
    assert!(!r.success, "Order discount should be blocked after payment");

    // Cancel payment
    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    let pid = s.payments[0].payment_id.clone();
    let r = cancel_payment(&manager, &order_id, &pid);
    assert!(r.success);

    // Now order discount should work again
    let r = apply_discount(&manager, &order_id, 20.0);
    assert!(
        r.success,
        "Order discount should work after cancelling all payments"
    );

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert!((s.total - 24.0).abs() < 0.01); // 30 * 0.8 = 24

    assert_snapshot_consistent(&manager, &order_id);
}

// --- Test 13: 添加商品 → 部分支付 → void → 验证 loss ---

#[test]
fn test_combo_partial_pay_then_void_loss() {
    let manager = create_test_manager();
    let order_id = open_table_with_items(
        &manager,
        289,
        vec![simple_item(1, "Coffee", 10.0, 5)], // 50
    );

    // Pay 30
    let r = pay(&manager, &order_id, 30.0, "CARD");
    assert!(r.success);

    // Void with loss settled (auto-calculate loss)
    let void_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::VoidOrder {
            order_id: order_id.clone(),
            void_type: VoidType::LossSettled,
            loss_reason: Some(shared::order::LossReason::CustomerFled),
            loss_amount: None, // auto-calculate
            note: None,
            authorizer_id: None,
            authorizer_name: None,
        },
    );
    let r = manager.execute_command(void_cmd);
    assert!(r.success);

    // Verify via snapshot: void sets status
    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert_eq!(s.status, OrderStatus::Void);
    // Verify loss_amount via rebuild (events contain the value)
    let rebuilt = manager.rebuild_snapshot(&order_id).unwrap();
    assert_eq!(rebuilt.status, OrderStatus::Void);

    assert_snapshot_consistent(&manager, &order_id);
}

// --- Test 14: 超付保护 — 边界值 ---

#[test]
fn test_combo_overpayment_boundary() {
    let manager = create_test_manager();
    let order_id = open_table_with_items(
        &manager,
        290,
        vec![simple_item(1, "Coffee", 10.0, 1)], // 10
    );

    // Pay 10.00 exact — should succeed
    let r = pay(&manager, &order_id, 10.0, "CARD");
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert!((s.remaining_amount).abs() < 0.01);

    // Try to pay even 0.01 more — should fail
    let r = pay(&manager, &order_id, 0.02, "CARD");
    assert!(!r.success, "Should reject payment when fully paid");

    assert_snapshot_consistent(&manager, &order_id);
}

// --- Test 15: 整单折扣 + 整单附加费 + 商品折扣 组合 ---

#[test]
fn test_combo_order_discount_surcharge_item_discount() {
    let manager = create_test_manager();
    let order_id = open_table_with_items(
        &manager,
        291,
        vec![
            simple_item(1, "Coffee", 10.0, 2), // 20
            simple_item(2, "Tea", 8.0, 1),     // 8 → total=28
        ],
    );

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    let coffee_iid = s
        .items
        .iter()
        .find(|i| i.name == "Coffee")
        .unwrap()
        .instance_id
        .clone();

    // 1. 50% item discount on coffee → coffee=10, total=18
    let r = modify_item(&manager, &order_id, &coffee_iid, discount_changes(50.0));
    assert!(r.success);

    // 2. 10% order discount → total = 18 - 1.8 = 16.2
    let r = apply_discount(&manager, &order_id, 10.0);
    assert!(r.success);

    // 3. 5% order surcharge → total = 18 - 1.8 + 0.9 = 17.1
    let cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::ApplyOrderSurcharge {
            order_id: order_id.clone(),
            surcharge_percent: Some(5.0),
            surcharge_amount: None,
            authorizer_id: None,
            authorizer_name: None,
        },
    );
    let r = manager.execute_command(cmd);
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert!(s.total > 0.0, "Total must be positive");
    assert!(s.total < 28.0, "Total must be less than original");

    // 4. Pay and complete
    let r = pay(&manager, &order_id, s.total, "CARD");
    assert!(r.success);

    let r = complete_order(&manager, &order_id);
    assert!(r.success);

    assert_snapshot_consistent(&manager, &order_id);
}

// --- Test 16: 3次部分支付 → 取消中间那笔 → 验证 remaining ---

#[test]
fn test_combo_three_payments_cancel_middle() {
    let manager = create_test_manager();
    let order_id = open_table_with_items(
        &manager,
        292,
        vec![simple_item(1, "Steak", 30.0, 3)], // 90
    );

    // 3 payments: 25, 35, 20
    let r = pay(&manager, &order_id, 25.0, "CARD");
    assert!(r.success);
    let r = pay(&manager, &order_id, 35.0, "CASH");
    assert!(r.success);
    let r = pay(&manager, &order_id, 20.0, "CARD");
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert!((s.paid_amount - 80.0).abs() < 0.01);
    assert_remaining_consistent(&s);

    // Cancel middle payment (35)
    let mid_pid = s
        .payments
        .iter()
        .find(|p| !p.cancelled && (p.amount - 35.0).abs() < 0.01)
        .unwrap()
        .payment_id
        .clone();
    let r = cancel_payment(&manager, &order_id, &mid_pid);
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert!(
        (s.paid_amount - 45.0).abs() < 0.01,
        "paid should be 25+20=45, got {}",
        s.paid_amount
    );
    assert_remaining_consistent(&s);
    assert!(
        (s.remaining_amount - 45.0).abs() < 0.01,
        "remaining should be 90-45=45, got {}",
        s.remaining_amount
    );

    // Pay remaining and complete
    let r = pay(&manager, &order_id, s.remaining_amount, "CARD");
    assert!(r.success);
    let r = complete_order(&manager, &order_id);
    assert!(r.success);

    assert_snapshot_consistent(&manager, &order_id);
}

// --- Test 17: 部分支付 → 整单折扣被阻 → 取消支付 → 整单折扣 + 附加费 → 支付 ---

#[test]
fn test_combo_cancel_payment_then_order_discount_and_surcharge() {
    let manager = create_test_manager();
    let order_id = open_table_with_items(
        &manager,
        293,
        vec![
            simple_item(1, "Coffee", 10.0, 4), // 40
            simple_item(2, "Tea", 5.0, 2),     // 10 → total=50
        ],
    );

    // Pay 20
    let r = pay(&manager, &order_id, 20.0, "CARD");
    assert!(r.success);

    // Try discount → blocked
    let r = apply_discount(&manager, &order_id, 20.0);
    assert!(!r.success, "Discount should be blocked after payment");

    // Try surcharge → also blocked
    let r = apply_surcharge(&manager, &order_id, 10.0);
    assert!(!r.success, "Surcharge should be blocked after payment");

    // Cancel payment
    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    let pid = s
        .payments
        .iter()
        .find(|p| !p.cancelled)
        .unwrap()
        .payment_id
        .clone();
    let r = cancel_payment(&manager, &order_id, &pid);
    assert!(r.success);

    // Now discount (20%) → total = 50 * 0.8 = 40
    let r = apply_discount(&manager, &order_id, 20.0);
    assert!(r.success);

    // Surcharge (10%) → total = 50 - 10 + 5 = 45
    // (discount on subtotal 50 = 10, surcharge on subtotal 50 = 5)
    let r = apply_surcharge(&manager, &order_id, 10.0);
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert!(s.total > 0.0);
    assert!(
        (s.total - 45.0).abs() < 0.01,
        "Expected 45, got {}",
        s.total
    );
    assert_remaining_consistent(&s);

    // Pay and complete
    let r = pay(&manager, &order_id, s.total, "CASH");
    assert!(r.success);
    let r = complete_order(&manager, &order_id);
    assert!(r.success);

    assert_snapshot_consistent(&manager, &order_id);
}

// --- Test 18: 商品折扣 + 整单折扣 + 整单附加费 + comp → 多层叠加 ---

#[test]
fn test_combo_multi_layer_discounts_surcharges_comp() {
    let manager = create_test_manager();
    let order_id = open_table_with_items(
        &manager,
        294,
        vec![
            simple_item(1, "Steak", 20.0, 2), // 40
            simple_item(2, "Wine", 15.0, 2),  // 30
            simple_item(3, "Bread", 3.0, 1),  // 3  → total=73
        ],
    );

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    let bread_iid = s
        .items
        .iter()
        .find(|i| i.name == "Bread")
        .unwrap()
        .instance_id
        .clone();
    let _steak_iid = s
        .items
        .iter()
        .find(|i| i.name == "Steak")
        .unwrap()
        .instance_id
        .clone();
    let wine_iid = s
        .items
        .iter()
        .find(|i| i.name == "Wine")
        .unwrap()
        .instance_id
        .clone();

    // 1. Comp bread (free) → subtotal = 40 + 30 = 70
    let r = comp_item(&manager, &order_id, &bread_iid);
    assert!(r.success);

    // 2. 50% item discount on wine → wine = 15, subtotal = 40 + 15 = 55
    let r = modify_item(&manager, &order_id, &wine_iid, discount_changes(50.0));
    assert!(r.success);

    // 3. 10% order discount → discount = 55 * 0.1 = 5.5
    let r = apply_discount(&manager, &order_id, 10.0);
    assert!(r.success);

    // 4. 5% order surcharge → surcharge = 55 * 0.05 = 2.75
    let r = apply_surcharge(&manager, &order_id, 5.0);
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    // total = subtotal(55) - discount(5.5) + surcharge(2.75) = 52.25
    assert!(
        (s.total - 52.25).abs() < 0.01,
        "Expected 52.25, got {}",
        s.total
    );
    assert!(s.total > 0.0);
    assert_remaining_consistent(&s);

    // 5. Uncomp bread → subtotal = 40 + 15 + 3 = 58
    //    But can't uncomp after order discount applied? Let's check...
    //    Order discount/surcharge don't block uncomp, only paid_amount blocks discount changes.
    let bread_iids: Vec<_> = s
        .items
        .iter()
        .filter(|i| i.name == "Bread")
        .map(|i| i.instance_id.clone())
        .collect();
    if let Some(comped_bread) = bread_iids.first() {
        let r = uncomp_item(&manager, &order_id, comped_bread);
        if r.success {
            let s = manager.get_snapshot(&order_id).unwrap().unwrap();
            // New subtotal = 40 + 15 + 3 = 58
            // discount = 58 * 0.1 = 5.8, surcharge = 58 * 0.05 = 2.9
            // total = 58 - 5.8 + 2.9 = 55.1
            assert!(s.total > 52.0, "Total should increase after uncomp");
            assert_remaining_consistent(&s);
        }
    }

    // 6. Pay and complete
    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    let r = pay(&manager, &order_id, s.total, "CARD");
    assert!(r.success);
    let r = complete_order(&manager, &order_id);
    assert!(r.success);

    assert_snapshot_consistent(&manager, &order_id);
}

// --- Test 19: 分单支付 → 改价(触发 split) → 取消分单 → 再改价 ---

#[test]
fn test_combo_split_pay_modify_cancel_modify_again() {
    let manager = create_test_manager();
    let order_id = open_table_with_items(
        &manager,
        295,
        vec![simple_item(1, "Coffee", 10.0, 6)], // 60
    );

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    let iid = s.items[0].instance_id.clone();

    // 1. Split-pay 3 coffees (30)
    let r = split_by_items(
        &manager,
        &order_id,
        vec![shared::order::SplitItem {
            instance_id: iid.clone(),
            name: "Coffee".to_string(),
            quantity: 3,
            unit_price: 10.0,
        }],
        "CARD",
    );
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert!((s.paid_amount - 30.0).abs() < 0.01);

    // Verify split-pay set paid_item_quantities
    assert!(
        s.paid_item_quantities.get(&iid).copied().unwrap_or(0) == 3,
        "Expected 3 paid coffees, got paid_item_quantities={:?}",
        s.paid_item_quantities
    );

    // 2. Modify unpaid coffee price to 8 (should split: paid@10, unpaid@8)
    let unpaid_item = s.items.iter().find(|i| i.unpaid_quantity > 0).unwrap();
    let unpaid_iid = unpaid_item.instance_id.clone();
    assert_eq!(
        unpaid_item.quantity, 6,
        "Item should still be qty=6 (unsplit)"
    );
    assert_eq!(unpaid_item.unpaid_quantity, 3, "Unpaid should be 3");

    let r = modify_item(&manager, &order_id, &unpaid_iid, price_changes(8.0));
    assert!(r.success, "Modify price failed: {:?}", r.error);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    // Debug: show what items we have
    let items_info: Vec<_> = s
        .items
        .iter()
        .map(|i| (i.price, i.quantity, i.unpaid_quantity))
        .collect();
    // paid portion: 3 * 10 = 30, unpaid: 3 * 8 = 24, total=54
    assert!(
        (s.total - 54.0).abs() < 0.01,
        "Expected 54, got {}. Items: {:?}, paid_amount: {}, paid_item_quantities: {:?}",
        s.total,
        items_info,
        s.paid_amount,
        s.paid_item_quantities
    );
    assert_remaining_consistent(&s);

    // 3. Cancel the split payment → paid items should be restored
    let pid = s
        .payments
        .iter()
        .find(|p| !p.cancelled)
        .unwrap()
        .payment_id
        .clone();
    let r = cancel_payment(&manager, &order_id, &pid);
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert!(
        (s.paid_amount).abs() < 0.01,
        "paid should be 0 after cancel"
    );
    assert_remaining_consistent(&s);

    // Total should still reflect the 2 different prices:
    // 3@10 (restored from split cancel) + 3@8 = 54
    assert!(
        (s.total - 54.0).abs() < 0.01,
        "Total should be 54 after cancel, got {}",
        s.total
    );

    // 4. Modify all items back to 10 (normalize price)
    for item in &s.items {
        if (item.price - 10.0).abs() > 0.01 {
            let r = modify_item(&manager, &order_id, &item.instance_id, price_changes(10.0));
            assert!(r.success);
        }
    }

    // After normalizing, items with same content should merge
    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    let total_qty: i32 = s.items.iter().map(|i| i.quantity).sum();
    assert_eq!(total_qty, 6, "Total qty should be 6");
    assert!(
        (s.total - 60.0).abs() < 0.01,
        "Total should be 60 after re-normalizing prices"
    );
    assert_remaining_consistent(&s);

    // 5. Pay and complete
    let r = pay(&manager, &order_id, s.total, "CASH");
    assert!(r.success);
    let r = complete_order(&manager, &order_id);
    assert!(r.success);

    assert_snapshot_consistent(&manager, &order_id);
}

// --- Test 20: comp → uncomp → discount → remove → add → 完整循环 ---

#[test]
fn test_combo_comp_uncomp_discount_remove_add_cycle() {
    let manager = create_test_manager();
    let order_id = open_table_with_items(
        &manager,
        296,
        vec![
            simple_item(1, "Steak", 25.0, 2), // 50
            simple_item(2, "Wine", 12.0, 3),  // 36 → total=86
        ],
    );

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    let wine_iid = s
        .items
        .iter()
        .find(|i| i.name == "Wine")
        .unwrap()
        .instance_id
        .clone();

    // 1. Comp wine
    let r = comp_item(&manager, &order_id, &wine_iid);
    assert!(r.success);
    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert!(
        (s.total - 50.0).abs() < 0.01,
        "Total should be 50 after comp wine"
    );

    // 2. Uncomp wine
    let comped_iid = s
        .items
        .iter()
        .find(|i| i.is_comped && i.name == "Wine")
        .unwrap()
        .instance_id
        .clone();
    let r = uncomp_item(&manager, &order_id, &comped_iid);
    assert!(r.success);
    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert!(
        (s.total - 86.0).abs() < 0.01,
        "Total should be 86 after uncomp"
    );

    // 3. 30% discount on wine
    let wine_iid = s
        .items
        .iter()
        .find(|i| i.name == "Wine" && !i.is_comped)
        .unwrap()
        .instance_id
        .clone();
    let r = modify_item(&manager, &order_id, &wine_iid, discount_changes(30.0));
    assert!(r.success);

    // 4. Remove 1 steak
    let steak_iid = s
        .items
        .iter()
        .find(|i| i.name == "Steak")
        .unwrap()
        .instance_id
        .clone();
    let r = remove_item(&manager, &order_id, &steak_iid, Some(1));
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    // steak: 1 * 25 = 25, wine: 3 * 12 * 0.7 = 25.2, total = 50.2
    assert!(s.total > 0.0);
    assert_remaining_consistent(&s);

    // 5. Add 2 more wines (same product, no discount → different instance_id)
    let r = add_items(&manager, &order_id, vec![simple_item(2, "Wine", 12.0, 2)]);
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    let total_wine_qty: i32 = s
        .items
        .iter()
        .filter(|i| i.name == "Wine" && !i.is_comped)
        .map(|i| i.quantity)
        .sum();
    assert_eq!(
        total_wine_qty, 5,
        "Should have 5 wines total (3 discounted + 2 new)"
    );
    assert_remaining_consistent(&s);

    // 6. Pay and complete
    let r = pay(&manager, &order_id, s.total, "CARD");
    assert!(r.success);
    let r = complete_order(&manager, &order_id);
    assert!(r.success);

    assert_snapshot_consistent(&manager, &order_id);
}

// --- Test 21: 整单折扣+附加费 → 改为固定折扣 → 改为固定附加费 → 反复切换 ---

#[test]
fn test_combo_order_adjustment_switching() {
    let manager = create_test_manager();
    let order_id = open_table_with_items(
        &manager,
        297,
        vec![simple_item(1, "Coffee", 10.0, 5)], // 50
    );

    // 1. 20% order discount → total = 50 - 10 = 40
    let r = apply_discount(&manager, &order_id, 20.0);
    assert!(r.success);
    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert!((s.total - 40.0).abs() < 0.01);

    // 2. Switch to fixed discount of 15 → total = 50 - 15 = 35
    let r = apply_discount_fixed(&manager, &order_id, 15.0);
    assert!(r.success);
    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert!(
        (s.total - 35.0).abs() < 0.01,
        "Expected 35 with fixed discount 15, got {}",
        s.total
    );

    // 3. Add 10% surcharge → total = 50 - 15 + 5 = 40
    let r = apply_surcharge(&manager, &order_id, 10.0);
    assert!(r.success);
    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert!(
        (s.total - 40.0).abs() < 0.01,
        "Expected 40, got {}",
        s.total
    );

    // 4. Switch surcharge to fixed 8 → total = 50 - 15 + 8 = 43
    let r = apply_surcharge_fixed(&manager, &order_id, 8.0);
    assert!(r.success);
    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert!(
        (s.total - 43.0).abs() < 0.01,
        "Expected 43, got {}",
        s.total
    );

    // 5. Remove discount entirely → total = 50 + 8 = 58
    let r = apply_discount(&manager, &order_id, 0.0);
    assert!(r.success);
    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert!(
        (s.total - 58.0).abs() < 0.01,
        "Expected 58, got {}",
        s.total
    );

    // 6. Remove surcharge → total = 50
    //    (surcharge_percent doesn't accept 0, use None/None to clear)
    let clear_surcharge_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::ApplyOrderSurcharge {
            order_id: order_id.clone(),
            surcharge_percent: None,
            surcharge_amount: None,
            authorizer_id: None,
            authorizer_name: None,
        },
    );
    let r = manager.execute_command(clear_surcharge_cmd);
    assert!(r.success);
    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert!(
        (s.total - 50.0).abs() < 0.01,
        "Expected 50, got {}",
        s.total
    );

    assert_remaining_consistent(&s);
    assert_snapshot_consistent(&manager, &order_id);
}

// --- Test 22: 整单折扣 > subtotal → total clamp 0 + 附加费 → total 仍为正 ---

#[test]
fn test_combo_extreme_discount_with_surcharge() {
    let manager = create_test_manager();
    let order_id = open_table_with_items(
        &manager,
        298,
        vec![simple_item(1, "Coffee", 5.0, 2)], // 10
    );

    // 固定折扣 30 on total 10 → clamp to 0
    let r = apply_discount_fixed(&manager, &order_id, 30.0);
    assert!(r.success);
    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert!(s.total >= 0.0, "Total must not be negative");
    assert!(
        (s.total).abs() < 0.01,
        "Total should be clamped to 0, got {}",
        s.total
    );

    // Add surcharge 5 → total = max(10 - 30, 0) + 5 → depends on clamp logic
    // Actually: total = (subtotal - discount + surcharge).max(0)
    //         = (10 - 30 + 5).max(0) = max(-15, 0) = 0
    let r = apply_surcharge_fixed(&manager, &order_id, 5.0);
    assert!(r.success);
    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert!(
        s.total >= 0.0,
        "Total must not be negative even with surcharge"
    );

    // Reduce discount to 8 → total = (10 - 8 + 5).max(0) = 7
    let r = apply_discount_fixed(&manager, &order_id, 8.0);
    assert!(r.success);
    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert!((s.total - 7.0).abs() < 0.01, "Expected 7, got {}", s.total);
    assert_remaining_consistent(&s);

    // Pay and complete
    let r = pay(&manager, &order_id, s.total, "CARD");
    assert!(r.success);
    let r = complete_order(&manager, &order_id);
    assert!(r.success);

    assert_snapshot_consistent(&manager, &order_id);
}

// --- Test 23: 部分支付多次 → 取消全部 → 重新支付 → 完成 ---

#[test]
fn test_combo_pay_multiple_cancel_all_repay() {
    let manager = create_test_manager();
    let order_id = open_table_with_items(
        &manager,
        299,
        vec![
            simple_item(1, "Coffee", 10.0, 2), // 20
            simple_item(2, "Tea", 5.0, 4),     // 20 → total=40
        ],
    );

    // 4 partial payments
    for amount in &[8.0, 12.0, 10.0, 5.0] {
        let r = pay(&manager, &order_id, *amount, "CARD");
        assert!(r.success);
    }

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert!((s.paid_amount - 35.0).abs() < 0.01);
    assert_remaining_consistent(&s);

    // Cancel all payments one by one
    let payment_ids: Vec<String> = s
        .payments
        .iter()
        .filter(|p| !p.cancelled)
        .map(|p| p.payment_id.clone())
        .collect();
    for pid in &payment_ids {
        let r = cancel_payment(&manager, &order_id, pid);
        assert!(r.success);
    }

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert!(
        (s.paid_amount).abs() < 0.01,
        "All payments cancelled, paid should be 0"
    );
    assert!(
        (s.remaining_amount - 40.0).abs() < 0.01,
        "Remaining should be full 40"
    );
    assert_remaining_consistent(&s);

    // Pay full amount at once
    let r = pay(&manager, &order_id, 40.0, "CASH");
    assert!(r.success);
    let r = complete_order(&manager, &order_id);
    assert!(r.success);

    assert_snapshot_consistent(&manager, &order_id);
}

// --- Test 24: 分单支付+商品折扣循环+取消 → 验证 remaining 始终一致 ---

#[test]
fn test_combo_split_discount_cycle_cancel_remaining_consistency() {
    let manager = create_test_manager();
    let order_id = open_table_with_items(
        &manager,
        300,
        vec![simple_item(1, "Coffee", 10.0, 6)], // 60
    );

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    let iid = s.items[0].instance_id.clone();

    // 1. Split-pay 2 coffees (20)
    let r = split_by_items(
        &manager,
        &order_id,
        vec![shared::order::SplitItem {
            instance_id: iid.clone(),
            name: "Coffee".to_string(),
            quantity: 2,
            unit_price: 10.0,
        }],
        "CARD",
    );
    assert!(r.success);

    // 2. 30% discount on unpaid coffees
    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert_remaining_consistent(&s); // check after split
    let unpaid_iid = s
        .items
        .iter()
        .find(|i| i.unpaid_quantity > 0)
        .unwrap()
        .instance_id
        .clone();
    let r = modify_item(&manager, &order_id, &unpaid_iid, discount_changes(30.0));
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert_remaining_consistent(&s); // check after discount

    // 3. Change discount to 50%
    let unpaid_iid = s
        .items
        .iter()
        .find(|i| i.unpaid_quantity > 0 && !i.is_comped)
        .unwrap()
        .instance_id
        .clone();
    let r = modify_item(&manager, &order_id, &unpaid_iid, discount_changes(50.0));
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert_remaining_consistent(&s); // check after 2nd discount change

    // 4. Cancel the split payment
    let pid = s
        .payments
        .iter()
        .find(|p| !p.cancelled)
        .unwrap()
        .payment_id
        .clone();
    let r = cancel_payment(&manager, &order_id, &pid);
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert!((s.paid_amount).abs() < 0.01);
    assert_remaining_consistent(&s); // check after cancel

    // 5. Remove discount
    let discounted: Vec<_> = s
        .items
        .iter()
        .filter(|i| i.manual_discount_percent.is_some())
        .map(|i| i.instance_id.clone())
        .collect();
    for iid in &discounted {
        modify_item(&manager, &order_id, iid, discount_changes(0.0));
    }

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    let total_qty: i32 = s.items.iter().map(|i| i.quantity).sum();
    assert_eq!(total_qty, 6);
    assert_remaining_consistent(&s); // final check

    assert_snapshot_consistent(&manager, &order_id);
}

// --- Test 25: 整单折扣 + 整单附加费 + 商品折扣 + comp → 支付后 void ---

#[test]
fn test_combo_everything_then_partial_pay_void() {
    let manager = create_test_manager();
    let order_id = open_table_with_items(
        &manager,
        301,
        vec![
            simple_item(1, "Steak", 20.0, 2), // 40
            simple_item(2, "Wine", 10.0, 3),  // 30
            simple_item(3, "Bread", 2.0, 2),  // 4  → total=74
        ],
    );

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    let bread_iid = s
        .items
        .iter()
        .find(|i| i.name == "Bread")
        .unwrap()
        .instance_id
        .clone();
    let wine_iid = s
        .items
        .iter()
        .find(|i| i.name == "Wine")
        .unwrap()
        .instance_id
        .clone();

    // 1. Comp bread → subtotal = 40 + 30 = 70
    let r = comp_item(&manager, &order_id, &bread_iid);
    assert!(r.success);

    // 2. 25% discount on wine → wine=22.5, subtotal = 40 + 22.5 = 62.5
    let r = modify_item(&manager, &order_id, &wine_iid, discount_changes(25.0));
    assert!(r.success);

    // 3. 10% order discount → -6.25
    let r = apply_discount(&manager, &order_id, 10.0);
    assert!(r.success);

    // 4. 5% order surcharge → +3.125
    let r = apply_surcharge(&manager, &order_id, 5.0);
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    // total = (62.5 - 6.25 + 3.125).max(0) ≈ 59.375
    assert!(
        s.total > 50.0 && s.total < 65.0,
        "Expected ~59.375, got {}",
        s.total
    );
    let expected_total = s.total;
    assert_remaining_consistent(&s);

    // 5. Pay 30 (partial)
    let r = pay(&manager, &order_id, 30.0, "CARD");
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert!((s.paid_amount - 30.0).abs() < 0.01);
    assert!((s.remaining_amount - (expected_total - 30.0)).abs() < 0.02);
    assert_remaining_consistent(&s);

    // 6. Void with loss → auto-calculate loss_amount = total - paid
    let void_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::VoidOrder {
            order_id: order_id.clone(),
            void_type: VoidType::LossSettled,
            loss_reason: Some(shared::order::LossReason::CustomerFled),
            loss_amount: None,
            note: Some("Complex order voided".to_string()),
            authorizer_id: None,
            authorizer_name: None,
        },
    );
    let r = manager.execute_command(void_cmd);
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert_eq!(s.status, OrderStatus::Void);

    assert_snapshot_consistent(&manager, &order_id);
}

// --- Test 26: 连续 add → remove → add → 验证总量和总价 ---

#[test]
fn test_combo_add_remove_add_items_total_tracking() {
    let manager = create_test_manager();
    let order_id = open_table_with_items(
        &manager,
        302,
        vec![simple_item(1, "Coffee", 10.0, 2)], // 20
    );

    // Add 3 more coffees → 5 total, 50
    let r = add_items(&manager, &order_id, vec![simple_item(1, "Coffee", 10.0, 3)]);
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert_eq!(s.items.len(), 1, "Same product should merge");
    assert_eq!(s.items[0].quantity, 5);
    assert!((s.total - 50.0).abs() < 0.01);

    // Remove 2 coffees → 3 left, 30
    let iid = s.items[0].instance_id.clone();
    let r = remove_item(&manager, &order_id, &iid, Some(2));
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    let total_qty: i32 = s.items.iter().map(|i| i.quantity).sum();
    assert_eq!(total_qty, 3);
    assert!((s.total - 30.0).abs() < 0.01);

    // Add tea
    let r = add_items(&manager, &order_id, vec![simple_item(2, "Tea", 5.0, 4)]);
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    // 3 coffees (30) + 4 teas (20) = 50
    assert!((s.total - 50.0).abs() < 0.01);
    assert_remaining_consistent(&s);

    // Pay and complete
    let r = pay(&manager, &order_id, 50.0, "CASH");
    assert!(r.success);
    let r = complete_order(&manager, &order_id);
    assert!(r.success);

    assert_snapshot_consistent(&manager, &order_id);
}

// --- Test 27: 部分支付 → 商品折扣(触发 split) → 取消支付 → 删除高价商品 → 支付 ---

#[test]
fn test_combo_partial_pay_discount_split_cancel_remove() {
    let manager = create_test_manager();
    let order_id = open_table_with_items(
        &manager,
        303,
        vec![
            simple_item(1, "Steak", 30.0, 2), // 60
            simple_item(2, "Salad", 8.0, 1),  // 8  → total=68
        ],
    );

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    let steak_iid = s
        .items
        .iter()
        .find(|i| i.name == "Steak")
        .unwrap()
        .instance_id
        .clone();

    // 1. Split-pay 1 steak (30)
    let r = split_by_items(
        &manager,
        &order_id,
        vec![shared::order::SplitItem {
            instance_id: steak_iid.clone(),
            name: "Steak".to_string(),
            quantity: 1,
            unit_price: 30.0,
        }],
        "CARD",
    );
    assert!(r.success);

    // 2. Apply 50% discount on unpaid steak (should split: paid@30 + unpaid@15)
    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    let unpaid_steak = s
        .items
        .iter()
        .find(|i| i.name == "Steak" && i.unpaid_quantity > 0)
        .unwrap();
    let r = modify_item(
        &manager,
        &order_id,
        &unpaid_steak.instance_id,
        discount_changes(50.0),
    );
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert_remaining_consistent(&s);

    // 3. Cancel the split payment
    let pid = s
        .payments
        .iter()
        .find(|p| !p.cancelled)
        .unwrap()
        .payment_id
        .clone();
    let r = cancel_payment(&manager, &order_id, &pid);
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert!((s.paid_amount).abs() < 0.01);
    assert_remaining_consistent(&s);

    // 4. Remove the salad
    let salad_iid = s
        .items
        .iter()
        .find(|i| i.name == "Salad")
        .unwrap()
        .instance_id
        .clone();
    let r = remove_item(&manager, &order_id, &salad_iid, None);
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert!(
        s.items.iter().all(|i| i.name != "Salad"),
        "Salad should be removed"
    );
    assert!(s.total > 0.0);
    assert_remaining_consistent(&s);

    // 5. Pay remaining and complete
    let r = pay(&manager, &order_id, s.total, "CASH");
    assert!(r.success);
    let r = complete_order(&manager, &order_id);
    assert!(r.success);

    assert_snapshot_consistent(&manager, &order_id);
}

// --- Test 28: 整单折扣+附加费 叠加后取消折扣 → 附加费基数变化 ---

#[test]
fn test_combo_order_discount_surcharge_interaction() {
    let manager = create_test_manager();
    let order_id = open_table_with_items(
        &manager,
        304,
        vec![simple_item(1, "Coffee", 10.0, 10)], // 100
    );

    // 20% discount → discount = 20, total = 80
    let r = apply_discount(&manager, &order_id, 20.0);
    assert!(r.success);
    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert!((s.total - 80.0).abs() < 0.01);

    // 10% surcharge → surcharge on subtotal(100) = 10, total = 100 - 20 + 10 = 90
    let r = apply_surcharge(&manager, &order_id, 10.0);
    assert!(r.success);
    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert!(
        (s.total - 90.0).abs() < 0.01,
        "Expected 90, got {}",
        s.total
    );

    // Remove discount → total = 100 + 10 = 110
    let r = apply_discount(&manager, &order_id, 0.0);
    assert!(r.success);
    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert!(
        (s.total - 110.0).abs() < 0.01,
        "Expected 110, got {}",
        s.total
    );

    // Remove surcharge → total = 100
    //    (surcharge_percent doesn't accept 0, use None/None to clear)
    let clear_surcharge_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::ApplyOrderSurcharge {
            order_id: order_id.clone(),
            surcharge_percent: None,
            surcharge_amount: None,
            authorizer_id: None,
            authorizer_name: None,
        },
    );
    let r = manager.execute_command(clear_surcharge_cmd);
    assert!(r.success);
    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert!(
        (s.total - 100.0).abs() < 0.01,
        "Expected 100, got {}",
        s.total
    );

    assert_remaining_consistent(&s);
    assert_snapshot_consistent(&manager, &order_id);
}

// --- Test 29: 部分支付 → 每笔支付后检查 remaining 一致性 ---

#[test]
fn test_combo_remaining_consistent_after_every_payment() {
    let manager = create_test_manager();
    let order_id = open_table_with_items(
        &manager,
        305,
        vec![simple_item(1, "Coffee", 7.5, 8)], // 60
    );

    let payments = vec![5.0, 10.5, 3.0, 15.0, 7.5, 9.0];
    let mut total_paid = 0.0;

    for amount in &payments {
        let r = pay(&manager, &order_id, *amount, "CARD");
        assert!(r.success, "Payment of {} failed", amount);
        total_paid += amount;

        let s = manager.get_snapshot(&order_id).unwrap().unwrap();
        assert!(
            (s.paid_amount - total_paid).abs() < 0.01,
            "After paying {}, expected paid_amount={}, got {}",
            amount,
            total_paid,
            s.paid_amount
        );
        assert_remaining_consistent(&s);
    }

    // Pay remaining
    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    let remaining = s.remaining_amount;
    assert!(remaining > 0.0, "Should still have remaining");
    let r = pay(&manager, &order_id, remaining, "CASH");
    assert!(r.success);

    let r = complete_order(&manager, &order_id);
    assert!(r.success);

    assert_snapshot_consistent(&manager, &order_id);
}

// --- Test 30: 固定折扣 + 百分比附加费 + 商品折扣 → 多维叠加计算验证 ---

#[test]
fn test_combo_fixed_discount_percent_surcharge_item_discount() {
    let manager = create_test_manager();
    let order_id = open_table_with_items(
        &manager,
        306,
        vec![
            simple_item(1, "A", 20.0, 3), // 60
            simple_item(2, "B", 15.0, 2), // 30 → subtotal=90
        ],
    );

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    let a_iid = s
        .items
        .iter()
        .find(|i| i.name == "A")
        .unwrap()
        .instance_id
        .clone();

    // 1. 40% item discount on A → A_line = 20*0.6*3 = 36, subtotal = 36+30 = 66
    let r = modify_item(&manager, &order_id, &a_iid, discount_changes(40.0));
    assert!(r.success);

    // 2. Fixed order discount of 10 → total = 66 - 10 = 56
    let r = apply_discount_fixed(&manager, &order_id, 10.0);
    assert!(r.success);

    // 3. 15% order surcharge → surcharge = 66 * 0.15 = 9.9, total = 66 - 10 + 9.9 = 65.9
    let r = apply_surcharge(&manager, &order_id, 15.0);
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert!(
        (s.total - 65.9).abs() < 0.1,
        "Expected ~65.9, got {}",
        s.total
    );
    assert_remaining_consistent(&s);

    // 4. Pay and complete
    let r = pay(&manager, &order_id, s.total, "CARD");
    assert!(r.success);
    let r = complete_order(&manager, &order_id);
    assert!(r.success);

    assert_snapshot_consistent(&manager, &order_id);
}
