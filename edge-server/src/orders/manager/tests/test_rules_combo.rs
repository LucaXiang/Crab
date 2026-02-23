use super::*;

// --- Test 31: 价格规则 + skip/unskip 循环 ---

#[tokio::test]
async fn test_combo_rule_skip_unskip_cycle() {
    let manager = create_test_manager();
    let order_id = open_table(&manager, 31).await;

    // 注入 10% 折扣规则
    manager.cache_rules(&order_id, vec![make_discount_rule(10, 10.0)]);

    // 添加商品: 100€ × 2
    let r = add_items(&manager, &order_id, vec![simple_item(1, "Steak", 100.0, 2)]).await;
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    // base=100, rule_discount=10% → unit_price=90, subtotal=180
    assert_close(s.subtotal, 180.0, "subtotal after rule");
    assert_close(s.total, 180.0, "total after rule");
    let item = &s.items[0];
    assert!(!item.applied_rules.is_empty(), "should have applied rules");
    assert_close(item.unit_price, 90.0, "unit_price with 10% discount");
    assert_close(item.price, 90.0, "item.price synced to unit_price");
    assert_eq!(item.original_price, 100.0, "original_price = catalog base");

    // Skip 规则 → 恢复原价
    let r = toggle_rule_skip(&manager, &order_id, 10, true).await;
    assert!(r.success, "skip failed: {:?}", r.error);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert_close(s.subtotal, 200.0, "subtotal after skip");
    assert_close(s.items[0].price, 100.0, "item.price after skip");

    // Unskip → 恢复折扣
    let r = toggle_rule_skip(&manager, &order_id, 10, false).await;
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert_close(s.subtotal, 180.0, "subtotal after unskip");
    assert_close(s.items[0].price, 90.0, "item.price after unskip");

    // 支付并结单
    let r = pay(&manager, &order_id, s.total, "CARD").await;
    assert!(r.success);
    let r = complete_order(&manager, &order_id).await;
    assert!(r.success);

    assert_snapshot_consistent(&manager, &order_id);
}

// --- Test 32: 价格规则 + 手动改价 ---

#[tokio::test]
async fn test_combo_rule_then_manual_reprice() {
    let manager = create_test_manager();
    let order_id = open_table(&manager, 32).await;

    manager.cache_rules(&order_id, vec![make_discount_rule(10, 10.0)]);

    let r = add_items(&manager, &order_id, vec![simple_item(1, "Wine", 100.0, 1)]).await;
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    let iid = s.items[0].instance_id.clone();
    assert_close(s.items[0].price, 90.0, "initial price with rule");

    // 手动改价到 80 → original_price=80, rule 10% on 80 → unit_price=72
    let r = modify_item(&manager, &order_id, &iid, price_changes(80.0)).await;
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    // 改价后 instance_id 可能变化，找到改价后的商品
    let item = &s.items[0];
    assert_eq!(
        item.original_price, 80.0,
        "original_price updated to manual price"
    );
    assert_close(item.unit_price, 72.0, "unit_price = 80 - 10% = 72");
    assert_close(item.price, 72.0, "item.price synced");
    assert_close(s.total, 72.0, "total");

    // Skip 规则 → 恢复到手动改价后的基础价
    let rule_id = item.applied_rules[0].rule_id.clone();
    let r = toggle_rule_skip(&manager, &order_id, rule_id, true).await;
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert_close(s.items[0].price, 80.0, "price after skip = manual price");
    assert_close(s.total, 80.0, "total after skip");

    // 支付完成
    let r = pay(&manager, &order_id, s.total, "CARD").await;
    assert!(r.success);
    let r = complete_order(&manager, &order_id).await;
    assert!(r.success);

    assert_snapshot_consistent(&manager, &order_id);
}

// --- Test 33: 选项 + 手动折扣 + 价格规则 ---

#[tokio::test]
async fn test_combo_options_manual_discount_rule() {
    let manager = create_test_manager();
    let order_id = open_table(&manager, 33).await;

    manager.cache_rules(&order_id, vec![make_discount_rule(10, 10.0)]);

    // 商品 50€, 选项 +5€ (加大), 数量 2
    let r = add_items(
        &manager,
        &order_id,
        vec![item_with_options(
            1,
            "Coffee",
            50.0,
            2,
            vec![make_option(1, "Size", 1, "Large", 5.0)],
        )],
    )
    .await;
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    let item = &s.items[0];
    // base=50, options=+5 → base_with_options=55
    // rule discount = 10% of 55 = 5.5 → unit_price = 55 - 5.5 = 49.5
    assert_close(item.unit_price, 49.5, "unit_price with option + rule");
    assert_close(s.subtotal, 99.0, "subtotal = 49.5 × 2");
    let iid = item.instance_id.clone();

    // 手动加 20% 折扣
    let r = modify_item(&manager, &order_id, &iid, discount_changes(20.0)).await;
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    let item = &s.items[0];
    // base_with_options=55, manual 20% = 11 → after_manual=44
    // rule discount = 10% of after_manual=44 → 4.4
    // unit_price = 55 - 11 - 4.4 = 39.6
    assert_close(
        item.unit_price,
        39.6,
        "unit_price with option + manual + rule",
    );
    assert_close(s.subtotal, 79.2, "subtotal = 39.6 × 2");

    // 支付完成
    let r = pay(&manager, &order_id, s.total, "CARD").await;
    assert!(r.success);
    let r = complete_order(&manager, &order_id).await;
    assert!(r.success);

    assert_snapshot_consistent(&manager, &order_id);
}

// --- Test 34: 规格变更 + 规则重算 ---

#[tokio::test]
async fn test_combo_spec_change_with_rule() {
    let manager = create_test_manager();
    let order_id = open_table(&manager, 34).await;

    manager.cache_rules(&order_id, vec![make_discount_rule(10, 10.0)]);

    // 商品: spec A 价格 100€
    let r = add_items(
        &manager,
        &order_id,
        vec![item_with_spec(
            1,
            "Pasta",
            100.0,
            1,
            make_spec(1, "Regular", Some(100.0)),
        )],
    )
    .await;
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    let iid = s.items[0].instance_id.clone();
    assert_close(s.items[0].price, 90.0, "initial with rule 10%");

    // 改规格到 spec B (价格 150€) — 通过 ModifyItem 改价格和规格
    let r = modify_item(
        &manager,
        &order_id,
        &iid,
        combo_changes(
            Some(150.0),
            None,
            None,
            None,
            Some(make_spec(2, "Premium", Some(150.0))),
        ),
    )
    .await;
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    let item = &s.items[0];
    // original_price=150, rule 10% → unit_price=135
    assert_eq!(
        item.original_price, 150.0,
        "original_price updated to new spec price"
    );
    assert_close(item.unit_price, 135.0, "unit_price after spec change");
    assert_close(s.total, 135.0, "total after spec change");

    assert_snapshot_consistent(&manager, &order_id);
}

// --- Test 35: 多规则 skip 其中一个 ---

#[tokio::test]
async fn test_combo_multiple_rules_selective_skip() {
    let manager = create_test_manager();
    let order_id = open_table(&manager, 35).await;

    // 两个规则: 10% 折扣 + 5% 附加费
    manager.cache_rules(
        &order_id,
        vec![make_discount_rule(10, 10.0), make_surcharge_rule(5, 5.0)],
    );

    let r = add_items(&manager, &order_id, vec![simple_item(1, "Steak", 100.0, 1)]).await;
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    // base=100, manual=0 → after_manual=100
    // discount: 10% of 100 = 10
    // surcharge: 5% of (base_with_options=100) = 5
    // unit_price = 100 - 10 + 5 = 95
    assert_close(s.items[0].unit_price, 95.0, "both rules active");
    assert_close(s.total, 95.0, "total");

    // Skip 折扣 → 只剩附加费
    let r = toggle_rule_skip(&manager, &order_id, 10, true).await;
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    // unit_price = 100 + 5 = 105
    assert_close(s.items[0].price, 105.0, "only surcharge active");

    // Skip 附加费 → 无规则
    let r = toggle_rule_skip(&manager, &order_id, 5, true).await;
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert_close(s.items[0].price, 100.0, "no rules active");

    // Unskip 两个
    let r = toggle_rule_skip(&manager, &order_id, 10, false).await;
    assert!(r.success);
    let r = toggle_rule_skip(&manager, &order_id, 5, false).await;
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert_close(s.items[0].price, 95.0, "both rules restored");

    // 支付完成
    let r = pay(&manager, &order_id, s.total, "CARD").await;
    assert!(r.success);
    let r = complete_order(&manager, &order_id).await;
    assert!(r.success);

    assert_snapshot_consistent(&manager, &order_id);
}

// --- Test 36: 价格规则 + 整单折扣 + 整单附加费 ---

#[tokio::test]
async fn test_combo_item_rule_plus_order_discount_surcharge() {
    let manager = create_test_manager();
    let order_id = open_table(&manager, 36).await;

    manager.cache_rules(&order_id, vec![make_discount_rule(10, 10.0)]);

    // 两个商品
    let r = add_items(
        &manager,
        &order_id,
        vec![
            simple_item(1, "A", 100.0, 1), // rule: 90
            simple_item(2, "B", 50.0, 2),  // rule: 45 × 2 = 90
        ],
    )
    .await;
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert_close(s.subtotal, 180.0, "subtotal with rules");

    // 整单 20% 折扣 → discount = 180 * 20% = 36 → total = 144
    let r = apply_discount(&manager, &order_id, 20.0).await;
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert_close(s.total, 144.0, "total after order discount");

    // 整单 10% 附加费 → surcharge = 180 * 10% = 18 → total = 180 - 36 + 18 = 162
    let r = apply_surcharge(&manager, &order_id, 10.0).await;
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert_close(s.total, 162.0, "total after surcharge");
    assert_remaining_consistent(&s);

    // Skip 商品规则 → subtotal 变大 → 整单折扣/附加费重算
    let rule_id = s.items[0].applied_rules[0].rule_id.clone();
    let r = toggle_rule_skip(&manager, &order_id, rule_id, true).await;
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    // subtotal = 100 + 50*2 = 200
    // discount = 200 * 20% = 40
    // surcharge = 200 * 10% = 20
    // total = 200 - 40 + 20 = 180
    assert_close(s.subtotal, 200.0, "subtotal after skip");
    assert_close(s.total, 180.0, "total after skip with order adjustments");

    // 支付完成
    let r = pay(&manager, &order_id, s.total, "CARD").await;
    assert!(r.success);
    let r = complete_order(&manager, &order_id).await;
    assert!(r.success);

    assert_snapshot_consistent(&manager, &order_id);
}

// --- Test 37: 价格规则 + 选项 ± 金额 + 修改选项 ---

#[tokio::test]
async fn test_combo_options_modifier_change() {
    let manager = create_test_manager();
    let order_id = open_table(&manager, 37).await;

    manager.cache_rules(&order_id, vec![make_discount_rule(10, 10.0)]);

    // 商品 80€, 选项 +10€ (Extra Cheese) + -3€ (No Sauce)
    let r = add_items(
        &manager,
        &order_id,
        vec![item_with_options(
            1,
            "Burger",
            80.0,
            1,
            vec![
                make_option(2, "Topping", 0, "Extra Cheese", 10.0),
                make_option(3, "Sauce", 1, "No Sauce", -3.0),
            ],
        )],
    )
    .await;
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    let iid = s.items[0].instance_id.clone();
    // base=80, options=+10-3=+7 → base_with_options=87
    // rule discount = 10% of 87 = 8.7
    // unit_price = 87 - 8.7 = 78.3
    assert_close(s.items[0].unit_price, 78.3, "initial with options + rule");

    // 修改选项: 换成 Extra Meat +15€
    let r = modify_item(
        &manager,
        &order_id,
        &iid,
        combo_changes(
            None,
            None,
            None,
            Some(vec![make_option(2, "Topping", 2, "Extra Meat", 15.0)]),
            None,
        ),
    )
    .await;
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    // base=80, options=+15 → base_with_options=95
    // rule discount = 10% of 95 = 9.5
    // unit_price = 95 - 9.5 = 85.5
    assert_close(s.items[0].unit_price, 85.5, "after options change");
    assert_close(s.total, 85.5, "total");

    // 支付完成
    let r = pay(&manager, &order_id, s.total, "CARD").await;
    assert!(r.success);
    let r = complete_order(&manager, &order_id).await;
    assert!(r.success);

    assert_snapshot_consistent(&manager, &order_id);
}

// --- Test 38: 固定金额规则 + 百分比规则叠加 ---

#[tokio::test]
async fn test_combo_fixed_and_percent_rules_stacking() {
    let manager = create_test_manager();
    let order_id = open_table(&manager, 38).await;

    // 固定 5€ 折扣 + 15% 附加费
    manager.cache_rules(
        &order_id,
        vec![
            make_fixed_discount_rule(50, 5.0),
            make_surcharge_rule(15, 15.0),
        ],
    );

    let r = add_items(&manager, &order_id, vec![simple_item(1, "Salmon", 60.0, 2)]).await;
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    // Per item: base=60, after_manual=60
    // discount: fixed 5
    // surcharge: 15% of base_with_options(60) = 9
    // unit_price = 60 - 5 + 9 = 64
    assert_close(s.items[0].unit_price, 64.0, "fixed disc + % surcharge");
    assert_close(s.subtotal, 128.0, "subtotal = 64 × 2");

    // Skip 折扣 → unit_price = 60 + 9 = 69
    let r = toggle_rule_skip(&manager, &order_id, 50, true).await;
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert_close(s.items[0].price, 69.0, "after skip discount");

    // Skip 附加费 → unit_price = 60
    let r = toggle_rule_skip(&manager, &order_id, 15, true).await;
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert_close(s.items[0].price, 60.0, "all rules skipped");

    // 支付完成
    let r = pay(&manager, &order_id, s.total, "CARD").await;
    assert!(r.success);
    let r = complete_order(&manager, &order_id).await;
    assert!(r.success);

    assert_snapshot_consistent(&manager, &order_id);
}

// --- Test 39: 全组合: 规则+手动折扣+选项+规格+整单折扣+整单附加费 ---

#[tokio::test]
async fn test_combo_kitchen_sink() {
    let manager = create_test_manager();
    let order_id = open_table(&manager, 39).await;

    // 10% 折扣规则
    manager.cache_rules(&order_id, vec![make_discount_rule(10, 10.0)]);

    // 商品1: 100€, spec A (100€), option +5€, 手动折扣 20%, qty 2
    let r = add_items(
        &manager,
        &order_id,
        vec![CartItemInput {
            product_id: 1,
            name: "Deluxe Plate".to_string(),
            price: 100.0,
            original_price: None,
            quantity: 2,
            selected_options: Some(vec![make_option(4, "Side", 0, "Truffle Fries", 5.0)]),
            selected_specification: Some(make_spec(1, "Regular", Some(100.0))),
            manual_discount_percent: Some(20.0),
            note: None,
            authorizer_id: None,
            authorizer_name: None,
        }],
    )
    .await;
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    let item = &s.items[0];
    // base=100, options=+5 → base_with_options=105
    // manual 20% = 21 → after_manual=84
    // rule 10% of after_manual(84) = 8.4
    // unit_price = 105 - 21 - 8.4 = 75.6
    assert_close(item.unit_price, 75.6, "item1 unit_price");
    assert_close(s.subtotal, 151.2, "subtotal = 75.6 × 2");

    // 加第二个商品: 简单 30€ × 3
    let r = add_items(&manager, &order_id, vec![simple_item(2, "Bread", 30.0, 3)]).await;
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    // Bread: 30 - 10% = 27, line = 27×3 = 81
    // subtotal = 151.2 + 81 = 232.2
    assert_close(s.subtotal, 232.2, "subtotal with both items");

    // 整单 5% 折扣
    let r = apply_discount(&manager, &order_id, 5.0).await;
    assert!(r.success);

    // 整单 8% 附加费
    let r = apply_surcharge(&manager, &order_id, 8.0).await;
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    // discount = 232.2 * 5% = 11.61
    // surcharge = 232.2 * 8% = 18.576 → 18.58
    // total = 232.2 - 11.61 + 18.58 = 239.17
    let expected_total = 232.2 - (232.2 * 0.05) + (232.2 * 0.08);
    assert!(
        (s.total - expected_total).abs() < 0.1,
        "total = {:.2}, expected {:.2}",
        s.total,
        expected_total
    );
    assert_remaining_consistent(&s);

    // 支付完成
    let r = pay(&manager, &order_id, s.total, "CARD").await;
    assert!(r.success);
    let r = complete_order(&manager, &order_id).await;
    assert!(r.success);

    assert_snapshot_consistent(&manager, &order_id);
}

// --- Test 40: 规则 + 部分支付 + skip 规则 + 支付剩余 ---

#[tokio::test]
async fn test_combo_rule_partial_pay_skip_pay_remaining() {
    let manager = create_test_manager();
    let order_id = open_table(&manager, 40).await;

    manager.cache_rules(&order_id, vec![make_discount_rule(10, 10.0)]);

    // 100€ × 2 → subtotal = 180 (after 10% discount)
    let r = add_items(&manager, &order_id, vec![simple_item(1, "A", 100.0, 2)]).await;
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert_close(s.total, 180.0, "initial total");

    // 部分支付 50
    let r = pay(&manager, &order_id, 50.0, "CARD").await;
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert_close(s.paid_amount, 50.0, "paid");
    assert_close(s.remaining_amount, 130.0, "remaining");

    // Skip 规则 → total 变为 200, remaining 变为 150
    let rule_id = s.items[0].applied_rules[0].rule_id.clone();
    let r = toggle_rule_skip(&manager, &order_id, rule_id, true).await;
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert_close(s.total, 200.0, "total after skip");
    assert_close(s.paid_amount, 50.0, "paid unchanged");
    assert_remaining_consistent(&s);
    assert_close(s.remaining_amount, 150.0, "remaining after skip");

    // 支付剩余并完成
    let r = pay(&manager, &order_id, s.remaining_amount, "CASH").await;
    assert!(r.success);
    let r = complete_order(&manager, &order_id).await;
    assert!(r.success);

    assert_snapshot_consistent(&manager, &order_id);
}

// --- Test 41: 选项 quantity > 1 + 规则 ---

#[tokio::test]
async fn test_combo_option_quantity_with_rule() {
    let manager = create_test_manager();
    let order_id = open_table(&manager, 41).await;

    manager.cache_rules(&order_id, vec![make_surcharge_rule(10, 10.0)]);

    // 商品 20€, 选项 +2€ × qty 3 (e.g., 3 eggs)
    let r = add_items(
        &manager,
        &order_id,
        vec![item_with_options(
            1,
            "Ramen",
            20.0,
            1,
            vec![shared::order::ItemOption {
                attribute_id: 7,
                attribute_name: "Eggs".to_string(),
                option_id: 0,
                option_name: "Extra Egg".to_string(),
                price_modifier: Some(2.0),
                quantity: 3,
            }],
        )],
    )
    .await;
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    // base=20, options=2*3=6 → base_with_options=26
    // surcharge=10% of 26=2.6 → unit_price=26+2.6=28.6
    assert_close(s.items[0].unit_price, 28.6, "option qty 3 + surcharge");
    assert_close(s.total, 28.6, "total");

    // 支付完成
    let r = pay(&manager, &order_id, s.total, "CARD").await;
    assert!(r.success);
    let r = complete_order(&manager, &order_id).await;
    assert!(r.success);

    assert_snapshot_consistent(&manager, &order_id);
}

// --- Test 42: 改价 + 改选项 + 改规格 + 改折扣 一次性修改 ---

#[tokio::test]
async fn test_combo_modify_everything_at_once() {
    let manager = create_test_manager();
    let order_id = open_table(&manager, 42).await;

    manager.cache_rules(&order_id, vec![make_discount_rule(5, 5.0)]);

    // 初始: 50€, spec A, option +3€
    let r = add_items(
        &manager,
        &order_id,
        vec![CartItemInput {
            product_id: 1,
            name: "Salad".to_string(),
            price: 50.0,
            original_price: None,
            quantity: 1,
            selected_options: Some(vec![make_option(5, "Dressing", 0, "Vinaigrette", 3.0)]),
            selected_specification: Some(make_spec(1, "Small", Some(50.0))),
            manual_discount_percent: None,
            note: None,
            authorizer_id: None,
            authorizer_name: None,
        }],
    )
    .await;
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    let iid = s.items[0].instance_id.clone();
    // base=50, option=+3, base_with_options=53
    // rule 5% of 53=2.65 → unit_price=53-2.65=50.35
    assert_close(s.items[0].unit_price, 50.35, "initial");

    // 一次性: 改价 60, 改规格 spec B, 改选项 +8€, 加 10% 手动折扣
    let r = modify_item(
        &manager,
        &order_id,
        &iid,
        combo_changes(
            Some(60.0),
            None,
            Some(10.0),
            Some(vec![make_option(5, "Dressing", 1, "Caesar", 8.0)]),
            Some(make_spec(2, "Large", Some(60.0))),
        ),
    )
    .await;
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    let item = &s.items[0];
    // original_price=60, option=+8, base_with_options=68
    // manual 10% = 6.8 → after_manual=61.2
    // rule 5% of 61.2=3.06 → unit_price=68-6.8-3.06=58.14
    assert_eq!(item.original_price, 60.0);
    assert_close(item.unit_price, 58.14, "after combo modify");
    assert_close(s.total, 58.14, "total");

    // 支付完成
    let r = pay(&manager, &order_id, s.total, "CARD").await;
    assert!(r.success);
    let r = complete_order(&manager, &order_id).await;
    assert!(r.success);

    assert_snapshot_consistent(&manager, &order_id);
}

// --- Test 43: 分单支付(按商品) + 价格规则 ---

#[tokio::test]
async fn test_combo_split_by_items_with_rule() {
    let manager = create_test_manager();
    let order_id = open_table(&manager, 43).await;

    manager.cache_rules(&order_id, vec![make_discount_rule(10, 10.0)]);

    // A: 100€ × 2 → 90×2=180, B: 50€ × 3 → 45×3=135
    let r = add_items(
        &manager,
        &order_id,
        vec![simple_item(1, "A", 100.0, 2), simple_item(2, "B", 50.0, 3)],
    )
    .await;
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert_close(s.subtotal, 315.0, "subtotal = 180 + 135");

    let a_iid = s
        .items
        .iter()
        .find(|i| i.name == "A")
        .unwrap()
        .instance_id
        .clone();
    let a_unit_price = s.items.iter().find(|i| i.name == "A").unwrap().unit_price;

    // 分单支付: 1 个 A
    let r = split_by_items(
        &manager,
        &order_id,
        vec![shared::order::SplitItem {
            instance_id: a_iid.clone(),
            name: "A".to_string(),
            quantity: 1,
            unit_price: a_unit_price,
        }],
        "CARD",
    )
    .await;
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert_close(s.paid_amount, 90.0, "paid 1 A = 90");
    assert_remaining_consistent(&s);

    // 支付剩余并完成
    let r = pay(&manager, &order_id, s.remaining_amount, "CASH").await;
    assert!(r.success);
    let r = complete_order(&manager, &order_id).await;
    assert!(r.success);

    assert_snapshot_consistent(&manager, &order_id);
}

// --- Test 44: 规则 + comp + uncomp ---

#[tokio::test]
async fn test_combo_rule_comp_uncomp() {
    let manager = create_test_manager();
    let order_id = open_table(&manager, 44).await;

    manager.cache_rules(&order_id, vec![make_discount_rule(10, 10.0)]);

    // 100€ × 2 → 90×2=180
    let r = add_items(&manager, &order_id, vec![simple_item(1, "A", 100.0, 2)]).await;
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    let iid = s.items[0].instance_id.clone();
    assert_close(s.total, 180.0, "initial");

    // Comp 1 个（不是全部）
    let comp_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::CompItem {
            order_id: order_id.clone(),
            instance_id: iid.clone(),
            quantity: 1,
            reason: "test comp".to_string(),
            authorizer_id: 1,
            authorizer_name: "Test".to_string(),
        },
    );
    let r = manager.execute_command(comp_cmd).await;
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    // 1 comped (price=0) + 1 normal (90) → subtotal=90
    assert_close(s.subtotal, 90.0, "after comp 1");
    assert!(s.comp_total_amount > 0.0, "comp_total tracked");
    // comp_total should be based on original_price (100) not discounted
    assert_close(s.comp_total_amount, 100.0, "comp_total = original value");

    // Uncomp
    let comped_iid = s
        .items
        .iter()
        .find(|i| i.is_comped)
        .unwrap()
        .instance_id
        .clone();
    let r = uncomp_item(&manager, &order_id, &comped_iid).await;
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert_close(s.subtotal, 180.0, "after uncomp, restored");
    assert_close(s.comp_total_amount, 0.0, "comp_total cleared");

    // 支付完成
    let r = pay(&manager, &order_id, s.total, "CARD").await;
    assert!(r.success);
    let r = complete_order(&manager, &order_id).await;
    assert!(r.success);

    assert_snapshot_consistent(&manager, &order_id);
}

// --- Test 45: 负选项金额 + 固定折扣规则 ---

#[tokio::test]
async fn test_combo_negative_option_with_fixed_discount() {
    let manager = create_test_manager();
    let order_id = open_table(&manager, 45).await;

    // 固定 3€ 折扣
    manager.cache_rules(&order_id, vec![make_fixed_discount_rule(30, 3.0)]);

    // 30€, 选项 -5€ (No Premium Ingredient), qty 2
    let r = add_items(
        &manager,
        &order_id,
        vec![item_with_options(
            1,
            "Soup",
            30.0,
            2,
            vec![make_option(6, "Ingredient", 0, "No Truffle", -5.0)],
        )],
    )
    .await;
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    // base=30, option=-5 → base_with_options=25
    // after_manual=25, fixed discount=3
    // unit_price = 25 - 3 = 22
    assert_close(
        s.items[0].unit_price,
        22.0,
        "negative option + fixed discount",
    );
    assert_close(s.subtotal, 44.0, "subtotal = 22 × 2");

    // 支付完成
    let r = pay(&manager, &order_id, s.total, "CARD").await;
    assert!(r.success);
    let r = complete_order(&manager, &order_id).await;
    assert!(r.success);

    assert_snapshot_consistent(&manager, &order_id);
}

// --- Test 46: [FIXED] Full comp + uncomp 保留 applied_rules ---
// Comp 保留 applied_rules, uncomp 后规则正确恢复
#[tokio::test]
async fn test_full_comp_uncomp_preserves_applied_rules() {
    let manager = create_test_manager();
    let order_id = open_table(&manager, 46).await;

    // 10% 折扣规则
    manager.cache_rules(&order_id, vec![make_discount_rule(10, 10.0)]);

    // 100€ × 1 → 规则后 90€
    let r = add_items(&manager, &order_id, vec![simple_item(1, "A", 100.0, 1)]).await;
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    let iid = s.items[0].instance_id.clone();
    assert_close(s.items[0].unit_price, 90.0, "before comp: 100*0.9=90");
    assert!(!s.items[0].applied_rules.is_empty(), "should have rules");

    // Full comp → rules preserved
    let r = comp_item(&manager, &order_id, &iid).await;
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert!(s.items[0].is_comped);
    assert_close(s.total, 0.0, "comped = free");
    assert!(
        !s.items[0].applied_rules.is_empty(),
        "rules preserved on comped item"
    );

    // Uncomp → rules correctly restored
    let r = uncomp_item(&manager, &order_id, &iid).await;
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert!(!s.items[0].is_comped);
    assert!(
        !s.items[0].applied_rules.is_empty(),
        "rules restored after uncomp"
    );
    assert_close(s.items[0].unit_price, 90.0, "100*0.9=90 correctly restored");
    assert_close(s.total, 90.0, "total correct after uncomp");

    assert_snapshot_consistent(&manager, &order_id);
}

// --- Test 47: Partial comp + uncomp (merge back) 保留源商品规则 ---
#[tokio::test]
async fn test_partial_comp_uncomp_merge_preserves_rules() {
    let manager = create_test_manager();
    let order_id = open_table(&manager, 47).await;

    manager.cache_rules(&order_id, vec![make_discount_rule(20, 20.0)]);

    // 50€ × 3 → 40×3=120
    let r = add_items(&manager, &order_id, vec![simple_item(1, "A", 50.0, 3)]).await;
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    let iid = s.items[0].instance_id.clone();
    assert_close(s.total, 120.0, "initial: 50*0.8*3=120");

    // Comp 1 个（partial）
    let r = comp_item_qty(&manager, &order_id, &iid, 1).await;
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert_eq!(s.items.len(), 2, "should split into 2 items");
    let source = s.items.iter().find(|i| !i.is_comped).unwrap();
    let comped = s.items.iter().find(|i| i.is_comped).unwrap();
    assert_eq!(source.quantity, 2);
    assert_eq!(comped.quantity, 1);
    // 源商品保留规则
    assert!(!source.applied_rules.is_empty(), "source keeps rules");
    assert_close(source.unit_price, 40.0, "source still 50*0.8=40");
    assert_close(s.subtotal, 80.0, "subtotal: 40*2=80 (comped=0)");

    // Uncomp → 合并回源
    let comped_iid = comped.instance_id.clone();
    let r = uncomp_item(&manager, &order_id, &comped_iid).await;
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert_eq!(s.items.len(), 1, "merged back");
    assert_eq!(s.items[0].quantity, 3);
    // 源商品规则保留 → 价格正确
    assert!(
        !s.items[0].applied_rules.is_empty(),
        "rules preserved after merge"
    );
    assert_close(s.total, 120.0, "restored: 40*3=120");

    assert_snapshot_consistent(&manager, &order_id);
}

// --- Test 48: 规则 + skip + comp + toggle(comped时仍可操作) + uncomp → 验证规则状态 ---
#[tokio::test]
async fn test_rule_skip_comp_toggle_uncomp_rules_preserved() {
    let manager = create_test_manager();
    let order_id = open_table(&manager, 48).await;

    manager.cache_rules(&order_id, vec![make_discount_rule(15, 15.0)]);

    // 200€ × 1 → 170
    let r = add_items(&manager, &order_id, vec![simple_item(1, "A", 200.0, 1)]).await;
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    let iid = s.items[0].instance_id.clone();
    assert_close(s.total, 170.0, "200*0.85=170");

    // Skip 规则 → total=200
    let rule_id: i64 = 15;
    let r = toggle_rule_skip(&manager, &order_id, rule_id, true).await;
    assert!(r.success);
    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert_close(s.total, 200.0, "rule skipped → 200");

    // Full comp → total=0, applied_rules preserved (fixed!)
    let r = comp_item(&manager, &order_id, &iid).await;
    assert!(r.success);
    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert_close(s.total, 0.0, "comped");
    assert!(
        !s.items[0].applied_rules.is_empty(),
        "rules preserved on comp"
    );

    // Toggle 规则应该成功 — rules 保留在 comped item 上
    let r = toggle_rule_skip(&manager, &order_id, rule_id, false).await;
    assert!(r.success, "toggle succeeds: rules preserved on comped item");
    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert_close(s.total, 0.0, "still comped, total stays 0");

    // Uncomp → 规则已 unskip, 应该恢复为 170
    let r = uncomp_item(&manager, &order_id, &iid).await;
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert!(!s.items[0].is_comped);
    assert!(
        !s.items[0].applied_rules.is_empty(),
        "rules restored after uncomp"
    );
    assert_close(s.items[0].unit_price, 170.0, "200*0.85=170 restored");
    assert_close(s.total, 170.0, "total restored with rules");
    assert_remaining_consistent(&s);
    assert_snapshot_consistent(&manager, &order_id);
}

// --- Test 49: 手动折扣 + comp + uncomp → 验证手动折扣恢复 ---
#[tokio::test]
async fn test_manual_discount_comp_uncomp_restores_discount() {
    let manager = create_test_manager();
    let order_id = open_table_with_items(&manager, 49, vec![simple_item(1, "A", 100.0, 1)]).await;

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    let iid = s.items[0].instance_id.clone();

    // 30% 手动折扣 → 70
    let r = modify_item(&manager, &order_id, &iid, discount_changes(30.0)).await;
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    // ModifyItem 可能改变 instance_id
    let iid = s.items[0].instance_id.clone();
    assert_close(s.total, 70.0, "100*0.7=70");
    assert_eq!(s.items[0].manual_discount_percent, Some(30.0));

    // Comp
    let r = comp_item(&manager, &order_id, &iid).await;
    assert!(r.success);
    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert_close(s.total, 0.0, "comped");
    // manual_discount_percent preserved on comped item (fixed!)
    assert_eq!(s.items[0].manual_discount_percent, Some(30.0));

    // Uncomp
    let r = uncomp_item(&manager, &order_id, &iid).await;
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert!(!s.items[0].is_comped);
    // manual_discount_percent 保留 → uncomp 后正确恢复折扣
    assert_eq!(s.items[0].manual_discount_percent, Some(30.0));
    assert_close(s.total, 70.0, "100*0.7=70 restored correctly");
    assert_remaining_consistent(&s);
    assert_snapshot_consistent(&manager, &order_id);
}

// --- Test 50: 部分支付 + comp + uncomp + 支付完成 ---
#[tokio::test]
async fn test_partial_pay_comp_uncomp_complete() {
    let manager = create_test_manager();
    let order_id = open_table_with_items(
        &manager,
        50,
        vec![
            simple_item(1, "A", 50.0, 2), // 100
            simple_item(2, "B", 30.0, 1), // 30 → total=130
        ],
    )
    .await;

    // 部分支付 60
    let r = pay(&manager, &order_id, 60.0, "CARD").await;
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert_close(s.paid_amount, 60.0, "paid 60");
    let b_iid = s
        .items
        .iter()
        .find(|i| i.name == "B")
        .unwrap()
        .instance_id
        .clone();

    // Comp B (30€)
    let r = comp_item(&manager, &order_id, &b_iid).await;
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert_close(s.total, 100.0, "A=100, B comped=0");
    assert_remaining_consistent(&s);

    // Uncomp B
    let r = uncomp_item(&manager, &order_id, &b_iid).await;
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert_close(s.total, 130.0, "B restored → 130");
    assert_remaining_consistent(&s);

    // 支付剩余 70 并完成
    let r = pay(&manager, &order_id, s.remaining_amount, "CASH").await;
    assert!(r.success);
    let r = complete_order(&manager, &order_id).await;
    assert!(r.success);

    assert_snapshot_consistent(&manager, &order_id);
}

// --- Test 51: 规则 + 部分支付 + comp 部分 + toggle rule ---
#[tokio::test]
async fn test_rule_partial_pay_partial_comp_toggle() {
    let manager = create_test_manager();
    let order_id = open_table(&manager, 51).await;

    manager.cache_rules(&order_id, vec![make_discount_rule(10, 10.0)]);

    // 100€ × 4 → 90×4=360
    let r = add_items(&manager, &order_id, vec![simple_item(1, "A", 100.0, 4)]).await;
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    let iid = s.items[0].instance_id.clone();
    assert_close(s.total, 360.0, "100*0.9*4=360");

    // 支付 180 (一半)
    let r = pay(&manager, &order_id, 180.0, "CARD").await;
    assert!(r.success);

    // Comp 1 个 (partial comp)
    let r = comp_item_qty(&manager, &order_id, &iid, 1).await;
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    // 源: qty=3 (2paid+1unpaid), comped: qty=1
    // subtotal = 90*3 = 270
    assert_close(s.subtotal, 270.0, "3 remaining @ 90");
    assert_close(s.paid_amount, 180.0, "paid unchanged");
    assert_remaining_consistent(&s);

    // Skip 规则 → 源商品变为 100/个, 3个=300
    let rule_id: i64 = 10;
    let r = toggle_rule_skip(&manager, &order_id, rule_id, true).await;
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    let source = s.items.iter().find(|i| !i.is_comped).unwrap();
    assert_close(source.unit_price, 100.0, "rule skipped → 100/unit");
    assert_close(s.subtotal, 300.0, "100*3=300");
    assert_remaining_consistent(&s);

    // Unskip → 回到 90/个
    let r = toggle_rule_skip(&manager, &order_id, rule_id, false).await;
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert_close(s.subtotal, 270.0, "restored 90*3=270");
    assert_remaining_consistent(&s);

    assert_snapshot_consistent(&manager, &order_id);
}

// --- Test 52: 两种商品 + 规则 + comp 其中一种 + 整单折扣 ---
#[tokio::test]
async fn test_two_items_rule_comp_order_discount() {
    let manager = create_test_manager();
    let order_id = open_table(&manager, 52).await;

    manager.cache_rules(&order_id, vec![make_discount_rule(10, 10.0)]);

    // A: 100€×2=200→180, B: 50€×1=50→45 → total=225
    let r = add_items(
        &manager,
        &order_id,
        vec![simple_item(1, "A", 100.0, 2), simple_item(2, "B", 50.0, 1)],
    )
    .await;
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert_close(s.subtotal, 225.0, "initial subtotal");
    let b_iid = s
        .items
        .iter()
        .find(|i| i.name == "B")
        .unwrap()
        .instance_id
        .clone();

    // Comp B
    let r = comp_item(&manager, &order_id, &b_iid).await;
    assert!(r.success);
    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert_close(s.subtotal, 180.0, "A=180, B comped");

    // 整单折扣 20% → subtotal=180, discount=36, total=144
    let r = apply_discount(&manager, &order_id, 20.0).await;
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert_close(s.total, 144.0, "180-36=144");
    assert_remaining_consistent(&s);

    // 支付完成
    let r = pay(&manager, &order_id, s.total, "CARD").await;
    assert!(r.success);
    let r = complete_order(&manager, &order_id).await;
    assert!(r.success);

    assert_snapshot_consistent(&manager, &order_id);
}

// --- Test 53: 选项 + 规则 + comp + uncomp + 修改选项 ---
#[tokio::test]
async fn test_options_rule_comp_uncomp_modify_options() {
    let manager = create_test_manager();
    let order_id = open_table(&manager, 53).await;

    manager.cache_rules(&order_id, vec![make_discount_rule(10, 10.0)]);

    // 80€ + 选项(+10€) = 90 → 规则后 81
    let r = add_items(
        &manager,
        &order_id,
        vec![item_with_options(
            1,
            "Steak",
            80.0,
            1,
            vec![make_option(4, "Side", 0, "Premium Fries", 10.0)],
        )],
    )
    .await;
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    let iid = s.items[0].instance_id.clone();
    assert_close(s.items[0].unit_price, 81.0, "(80+10)*0.9=81");

    // Full comp
    let r = comp_item(&manager, &order_id, &iid).await;
    assert!(r.success);

    // Uncomp
    let r = uncomp_item(&manager, &order_id, &iid).await;
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    // uncomp 恢复 price, 但 applied_rules 可能丢失 (Test 46 BUG)
    // 选项应该还在
    assert!(s.items[0].selected_options.is_some());
    assert_remaining_consistent(&s);
    assert_snapshot_consistent(&manager, &order_id);
}

// --- Test 54: 规则 + 分单支付 + 取消支付 + toggle rule ---
#[tokio::test]
async fn test_rule_split_pay_cancel_toggle() {
    let manager = create_test_manager();
    let order_id = open_table(&manager, 54).await;

    manager.cache_rules(&order_id, vec![make_discount_rule(10, 10.0)]);

    // 100€ × 3 → 90*3=270
    let r = add_items(&manager, &order_id, vec![simple_item(1, "A", 100.0, 3)]).await;
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    let iid = s.items[0].instance_id.clone();
    let a_unit_price = s.items[0].unit_price;
    assert_close(a_unit_price, 90.0, "100*0.9=90");

    // 分单支付 1 个 A
    let r = split_by_items(
        &manager,
        &order_id,
        vec![shared::order::SplitItem {
            instance_id: iid.clone(),
            name: "A".to_string(),
            quantity: 1,
            unit_price: a_unit_price,
        }],
        "CARD",
    )
    .await;
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert_close(s.paid_amount, 90.0, "paid 1×90=90");
    assert_remaining_consistent(&s);

    // 取消这笔支付
    let payment_id = s
        .payments
        .iter()
        .find(|p| !p.cancelled)
        .unwrap()
        .payment_id
        .clone();
    let r = cancel_payment(&manager, &order_id, &payment_id).await;
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert_close(s.paid_amount, 0.0, "payment cancelled");
    assert_remaining_consistent(&s);

    // Toggle rule skip → 100/个
    let rule_id: i64 = 10;
    let r = toggle_rule_skip(&manager, &order_id, rule_id, true).await;
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert_close(s.subtotal, 300.0, "3×100=300 (rule skipped)");
    assert_remaining_consistent(&s);

    // Unskip → 90/个
    let r = toggle_rule_skip(&manager, &order_id, rule_id, false).await;
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert_close(s.subtotal, 270.0, "3×90=270");
    assert_remaining_consistent(&s);

    assert_snapshot_consistent(&manager, &order_id);
}

// --- Test 55: 多商品 + 规则 + 选择性 comp + 分单支付 ---
#[tokio::test]
async fn test_multi_items_rule_selective_comp_split_pay() {
    let manager = create_test_manager();
    let order_id = open_table(&manager, 55).await;

    manager.cache_rules(&order_id, vec![make_discount_rule(10, 10.0)]);

    // A: 80€×2 → 72×2=144, B: 40€×1 → 36
    let r = add_items(
        &manager,
        &order_id,
        vec![simple_item(1, "A", 80.0, 2), simple_item(2, "B", 40.0, 1)],
    )
    .await;
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    let a_iid = s
        .items
        .iter()
        .find(|i| i.name == "A")
        .unwrap()
        .instance_id
        .clone();
    let b_iid = s
        .items
        .iter()
        .find(|i| i.name == "B")
        .unwrap()
        .instance_id
        .clone();
    assert_close(s.subtotal, 180.0, "144+36=180");

    // Comp B
    let r = comp_item(&manager, &order_id, &b_iid).await;
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert_close(s.subtotal, 144.0, "A only");

    // 分单支付 1 个 A
    let a_unit = s.items.iter().find(|i| i.name == "A").unwrap().unit_price;
    let r = split_by_items(
        &manager,
        &order_id,
        vec![shared::order::SplitItem {
            instance_id: a_iid.clone(),
            name: "A".to_string(),
            quantity: 1,
            unit_price: a_unit,
        }],
        "CARD",
    )
    .await;
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert_close(s.paid_amount, 72.0, "paid 1A=72");
    assert_remaining_consistent(&s);

    // 支付剩余并完成
    let r = pay(&manager, &order_id, s.remaining_amount, "CASH").await;
    assert!(r.success);
    let r = complete_order(&manager, &order_id).await;
    assert!(r.success);

    assert_snapshot_consistent(&manager, &order_id);
}

// --- Test 56: 修改数量 + 规则 + comp 部分 + 修改价格 ---
#[tokio::test]
async fn test_modify_qty_rule_partial_comp_modify_price() {
    let manager = create_test_manager();
    let order_id = open_table(&manager, 56).await;

    manager.cache_rules(&order_id, vec![make_discount_rule(10, 10.0)]);

    // 60€ × 2 → 54×2=108
    let r = add_items(&manager, &order_id, vec![simple_item(1, "A", 60.0, 2)]).await;
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    let iid = s.items[0].instance_id.clone();
    assert_close(s.total, 108.0, "60*0.9*2=108");

    // 增加到 4 个 → 54×4=216
    let r = modify_item(&manager, &order_id, &iid, qty_changes(4)).await;
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    let iid = s.items[0].instance_id.clone();
    assert_close(s.total, 216.0, "54*4=216");

    // Comp 1 个
    let r = comp_item_qty(&manager, &order_id, &iid, 1).await;
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert_close(s.subtotal, 162.0, "54*3=162");

    // 修改源商品价格 → 80€
    let source = s.items.iter().find(|i| !i.is_comped).unwrap();
    let source_iid = source.instance_id.clone();
    let r = modify_item(&manager, &order_id, &source_iid, price_changes(80.0)).await;
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    // 源商品: 80*0.9=72 (如果规则还在), qty=3 → 216
    // comped 商品: price=0
    let source = s.items.iter().find(|i| !i.is_comped).unwrap();
    assert_close(source.unit_price, 72.0, "80*0.9=72");
    assert_remaining_consistent(&s);

    assert_snapshot_consistent(&manager, &order_id);
}

// --- Test 57: 规则 + 整单附加费 + comp + 取消附加费 ---
#[tokio::test]
async fn test_rule_surcharge_comp_cancel_surcharge() {
    let manager = create_test_manager();
    let order_id = open_table(&manager, 57).await;

    manager.cache_rules(&order_id, vec![make_discount_rule(10, 10.0)]);

    // 100€ × 2 → 90×2=180
    let r = add_items(&manager, &order_id, vec![simple_item(1, "A", 100.0, 2)]).await;
    assert!(r.success);

    // 10% 整单附加费 → subtotal=180, surcharge=18, total=198
    let r = apply_surcharge(&manager, &order_id, 10.0).await;
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    let iid = s
        .items
        .iter()
        .find(|i| i.name == "A")
        .unwrap()
        .instance_id
        .clone();
    assert_close(s.total, 198.0, "180+18=198");

    // Comp 全部 A
    let r = comp_item(&manager, &order_id, &iid).await;
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    // subtotal=0 (全部 comped), surcharge=0 (基于 subtotal), total=0
    assert_close(s.total, 0.0, "all comped → total=0");

    // 取消附加费
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
    let r = manager.execute_command(clear_surcharge_cmd).await;
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert_close(s.total, 0.0, "still 0 (all comped, no surcharge)");

    // Uncomp
    let comped_iid = s
        .items
        .iter()
        .find(|i| i.is_comped)
        .unwrap()
        .instance_id
        .clone();
    let r = uncomp_item(&manager, &order_id, &comped_iid).await;
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    // uncomp 后: subtotal 应恢复, surcharge 已清除
    assert!(s.total > 0.0, "uncomped, should have value");
    assert_remaining_consistent(&s);
    assert_snapshot_consistent(&manager, &order_id);
}

// --- Test 58: 固定+百分比规则 + comp + uncomp + skip 交叉 ---
#[tokio::test]
async fn test_fixed_percent_rules_comp_uncomp_skip_cross() {
    let manager = create_test_manager();
    let order_id = open_table(&manager, 58).await;

    // 百分比折扣 10% + 固定折扣 5€
    manager.cache_rules(
        &order_id,
        vec![
            make_discount_rule(100, 10.0),
            make_fixed_discount_rule(200, 5.0),
        ],
    );

    // 100€ × 2
    // base=100, percent disc=10 → 90, fixed disc=5 → 85 per unit
    let r = add_items(&manager, &order_id, vec![simple_item(1, "A", 100.0, 2)]).await;
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    let iid = s.items[0].instance_id.clone();
    assert_close(s.items[0].unit_price, 85.0, "100*0.9 - 5 = 85");
    assert_close(s.subtotal, 170.0, "85*2=170");

    // Skip 固定折扣 → 90/unit
    let r = toggle_rule_skip(&manager, &order_id, 200, true).await;
    assert!(r.success);
    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert_close(s.items[0].unit_price, 90.0, "100*0.9=90 (fixed skipped)");

    // Comp 全部
    let r = comp_item(&manager, &order_id, &iid).await;
    assert!(r.success);

    // Uncomp
    let r = uncomp_item(&manager, &order_id, &iid).await;
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    // uncomp 后, 如果 applied_rules 丢失 (BUG), 则无折扣 → 100
    // 如果保留, 考虑 fdisc 仍然 skipped → 90
    assert_remaining_consistent(&s);
    assert_snapshot_consistent(&manager, &order_id);
}

// --- Test 59: 加菜 → 规则 → 再加菜 → comp 第一批 → 验证第二批不受影响 ---
#[tokio::test]
async fn test_add_items_twice_rule_comp_first_batch() {
    let manager = create_test_manager();
    let order_id = open_table(&manager, 59).await;

    manager.cache_rules(&order_id, vec![make_discount_rule(10, 10.0)]);

    // 第一批: A 100€ × 1 → 90
    let r = add_items(&manager, &order_id, vec![simple_item(1, "A", 100.0, 1)]).await;
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    let a_iid = s.items[0].instance_id.clone();
    assert_close(s.total, 90.0, "A=90");

    // 第二批: B 50€ × 2 → 45×2=90
    let r = add_items(&manager, &order_id, vec![simple_item(2, "B", 50.0, 2)]).await;
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert_close(s.total, 180.0, "A=90 + B=90 → 180");

    // Comp A
    let r = comp_item(&manager, &order_id, &a_iid).await;
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert_close(s.subtotal, 90.0, "B only: 45*2=90");

    // B 的规则不应受 comp A 影响
    let b_item = s.items.iter().find(|i| i.name == "B").unwrap();
    assert!(!b_item.applied_rules.is_empty(), "B keeps its rules");
    assert_close(b_item.unit_price, 45.0, "B=50*0.9=45");
    assert_remaining_consistent(&s);

    // 支付完成
    let r = pay(&manager, &order_id, s.total, "CARD").await;
    assert!(r.success);
    let r = complete_order(&manager, &order_id).await;
    assert!(r.success);

    assert_snapshot_consistent(&manager, &order_id);
}

// --- Test 60: Kitchen sink — 规则+选项+折扣+comp+uncomp+支付+取消+toggle+完成 ---
#[tokio::test]
async fn test_kitchen_sink_all_interactions() {
    let manager = create_test_manager();
    let order_id = open_table(&manager, 60).await;

    // 15% 折扣规则
    manager.cache_rules(&order_id, vec![make_discount_rule(15, 15.0)]);

    // A: 200€, 选项+20€, qty=2 → base=220, *0.85=187 → 374
    // B: 80€, qty=1 → 80*0.85=68
    // total = 374+68 = 442
    let r = add_items(
        &manager,
        &order_id,
        vec![
            item_with_options(
                1,
                "A",
                200.0,
                2,
                vec![make_option(7, "Add-on", 0, "Truffle", 20.0)],
            ),
            simple_item(2, "B", 80.0, 1),
        ],
    )
    .await;
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    let b_iid = s
        .items
        .iter()
        .find(|i| i.name == "B")
        .unwrap()
        .instance_id
        .clone();
    assert_close(s.subtotal, 442.0, "initial subtotal");

    // 1. 整单折扣 10% → discount=44.2, total=397.8
    let r = apply_discount(&manager, &order_id, 10.0).await;
    assert!(r.success);
    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert_close(s.total, 397.8, "442-44.2=397.8");

    // 2. 部分支付 100
    let r = pay(&manager, &order_id, 100.0, "CARD").await;
    assert!(r.success);

    // 3. Comp B
    let r = comp_item(&manager, &order_id, &b_iid).await;
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    // subtotal=374 (A only), discount=37.4, total=336.6
    assert_close(s.subtotal, 374.0, "A=374, B comped");
    assert_remaining_consistent(&s);

    // 4. Skip 规则 → A base=220, qty=2 → subtotal=440
    let rule_id: i64 = 15;
    let r = toggle_rule_skip(&manager, &order_id, rule_id, true).await;
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert_close(s.subtotal, 440.0, "A=220*2=440 (rule skipped)");

    // 5. 取消第一笔支付
    let payment_id = s
        .payments
        .iter()
        .find(|p| !p.cancelled)
        .unwrap()
        .payment_id
        .clone();
    let r = cancel_payment(&manager, &order_id, &payment_id).await;
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert_close(s.paid_amount, 0.0, "payment cancelled");

    // 6. Unskip 规则 → A=187*2=374
    let r = toggle_rule_skip(&manager, &order_id, rule_id, false).await;
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert_close(s.subtotal, 374.0, "rules restored");

    // 7. Uncomp B
    let b_comped_iid = s
        .items
        .iter()
        .find(|i| i.is_comped)
        .unwrap()
        .instance_id
        .clone();
    let r = uncomp_item(&manager, &order_id, &b_comped_iid).await;
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    // B uncomped: 看 Test 46 BUG 是否影响, B 的 applied_rules 可能丢失
    assert_remaining_consistent(&s);

    // 8. 清除整单折扣
    let r = clear_discount(&manager, &order_id).await;
    assert!(r.success);

    // 9. 支付全额并完成
    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    let r = pay(&manager, &order_id, s.total, "CASH").await;
    assert!(r.success);
    let r = complete_order(&manager, &order_id).await;
    assert!(r.success);

    assert_snapshot_consistent(&manager, &order_id);
}
