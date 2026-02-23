use super::*;

// ========================================================================
// 18. 完成订单后不能添加商品
// ========================================================================

#[tokio::test]
async fn test_cannot_add_items_to_completed_order() {
    let manager = create_test_manager();
    let order_id =
        open_table_with_items(&manager, 213, vec![simple_item(1, "Coffee", 10.0, 1)]).await;

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
    manager.execute_command(pay_cmd).await;

    let complete_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::CompleteOrder {
            order_id: order_id.clone(),
            service_type: Some(ServiceType::DineIn),
        },
    );
    manager.execute_command(complete_cmd).await;

    let add_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::AddItems {
            order_id: order_id.clone(),
            items: vec![simple_item(2, "Tea", 5.0, 1)],
        },
    );
    let resp = manager.execute_command(add_cmd).await;
    assert!(
        !resp.success,
        "Should not allow adding items to completed order"
    );
}

// ========================================================================
// ========================================================================
//  边界测试: 价格/数量/折扣/支付的极端值
// ========================================================================
// ========================================================================

// ========================================================================
// 19. 零价格商品可以正常添加和完成
// ========================================================================

#[tokio::test]
async fn test_add_items_with_zero_price() {
    let manager = create_test_manager();
    let order_id =
        open_table_with_items(&manager, 214, vec![simple_item(1, "Free Sample", 0.0, 1)]).await;

    let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert_eq!(snapshot.items.len(), 1);
    assert_eq!(snapshot.items[0].price, 0.0);
    assert_eq!(snapshot.subtotal, 0.0);
    assert_eq!(snapshot.total, 0.0);

    // 零总额可以直接完成
    let complete_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::CompleteOrder {
            order_id: order_id.clone(),
            service_type: Some(ServiceType::DineIn),
        },
    );
    let resp = manager.execute_command(complete_cmd).await;
    assert!(resp.success, "Zero-price order should complete");
}

// ========================================================================
// 20. NaN 价格 — 静默变成 0 (当前行为记录)
// ========================================================================

#[tokio::test]
async fn test_add_items_with_nan_price_rejected() {
    let manager = create_test_manager();

    let open_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::OpenTable {
            table_id: Some(308),
            table_name: Some("Table NaN".to_string()),
            zone_id: None,
            zone_name: None,
            guest_count: 1,
            is_retail: false,
        },
    );
    let resp = manager.execute_command(open_cmd).await;
    let order_id = resp.order_id.unwrap();

    let add_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::AddItems {
            order_id: order_id.clone(),
            items: vec![simple_item(1, "NaN Item", f64::NAN, 2)],
        },
    );
    let resp = manager.execute_command(add_cmd).await;
    assert!(!resp.success, "NaN price should be rejected by validation");
}

// ========================================================================
// 21. Infinity 价格 — 静默变成 0
// ========================================================================

#[tokio::test]
async fn test_add_items_with_infinity_price_rejected() {
    let manager = create_test_manager();

    let open_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::OpenTable {
            table_id: Some(309),
            table_name: Some("Table Inf".to_string()),
            zone_id: None,
            zone_name: None,
            guest_count: 1,
            is_retail: false,
        },
    );
    let resp = manager.execute_command(open_cmd).await;
    let order_id = resp.order_id.unwrap();

    let add_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::AddItems {
            order_id: order_id.clone(),
            items: vec![simple_item(1, "Infinity Item", f64::INFINITY, 1)],
        },
    );
    let resp = manager.execute_command(add_cmd).await;
    assert!(
        !resp.success,
        "Infinity price should be rejected by validation"
    );
}

// ========================================================================
// 22. 负价格 — 当前被 clamp 到 0
// ========================================================================

#[tokio::test]
async fn test_add_items_with_negative_price_rejected() {
    let manager = create_test_manager();

    let open_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::OpenTable {
            table_id: Some(310),
            table_name: Some("Table Neg".to_string()),
            zone_id: None,
            zone_name: None,
            guest_count: 1,
            is_retail: false,
        },
    );
    let resp = manager.execute_command(open_cmd).await;
    let order_id = resp.order_id.unwrap();

    let add_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::AddItems {
            order_id: order_id.clone(),
            items: vec![simple_item(1, "Negative Item", -10.0, 1)],
        },
    );
    let resp = manager.execute_command(add_cmd).await;
    assert!(
        !resp.success,
        "Negative price should be rejected by validation"
    );
}

// ========================================================================
// 23. 极大价格 × 数量仍正确计算
// ========================================================================

#[tokio::test]
async fn test_add_items_large_price_and_quantity() {
    let manager = create_test_manager();
    let order_id = open_table_with_items(
        &manager,
        215,
        vec![simple_item(1, "Expensive Item", 99999.99, 100)],
    )
    .await;

    let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
    // 99999.99 * 100 = 9_999_999.0
    assert_eq!(snapshot.subtotal, 9_999_999.0);
    assert_eq!(snapshot.total, 9_999_999.0);
}

// ========================================================================
// 24. f64::MAX 价格 — 转为 0 (Decimal 转换失败)
// ========================================================================

#[tokio::test]
async fn test_add_items_with_f64_max_price_rejected() {
    let manager = create_test_manager();

    let open_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::OpenTable {
            table_id: Some(311),
            table_name: Some("Table Max".to_string()),
            zone_id: None,
            zone_name: None,
            guest_count: 1,
            is_retail: false,
        },
    );
    let resp = manager.execute_command(open_cmd).await;
    let order_id = resp.order_id.unwrap();

    let add_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::AddItems {
            order_id: order_id.clone(),
            items: vec![simple_item(1, "Max Item", f64::MAX, 1)],
        },
    );
    let resp = manager.execute_command(add_cmd).await;
    assert!(
        !resp.success,
        "f64::MAX price should be rejected (exceeds max)"
    );
}

// ========================================================================
// 25. 数量为 0 — 当前被接受（应添加商品但金额为 0）
// ========================================================================

#[tokio::test]
async fn test_add_items_with_zero_quantity_rejected() {
    let manager = create_test_manager();

    let open_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::OpenTable {
            table_id: Some(312),
            table_name: Some("Table Zero Qty".to_string()),
            zone_id: None,
            zone_name: None,
            guest_count: 1,
            is_retail: false,
        },
    );
    let resp = manager.execute_command(open_cmd).await;
    let order_id = resp.order_id.unwrap();

    let add_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::AddItems {
            order_id: order_id.clone(),
            items: vec![simple_item(1, "Zero Qty", 10.0, 0)],
        },
    );
    let resp = manager.execute_command(add_cmd).await;
    assert!(!resp.success, "Zero quantity should be rejected");
}

// ========================================================================
// 26. 负数量 — 当前被接受 (导致负总额)
// ========================================================================

#[tokio::test]
async fn test_add_items_with_negative_quantity_rejected() {
    let manager = create_test_manager();

    let open_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::OpenTable {
            table_id: Some(313),
            table_name: Some("Table Neg Qty".to_string()),
            zone_id: None,
            zone_name: None,
            guest_count: 1,
            is_retail: false,
        },
    );
    let resp = manager.execute_command(open_cmd).await;
    let order_id = resp.order_id.unwrap();

    let add_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::AddItems {
            order_id: order_id.clone(),
            items: vec![simple_item(1, "Negative Qty", 10.0, -3)],
        },
    );
    let resp = manager.execute_command(add_cmd).await;
    assert!(!resp.success, "Negative quantity should be rejected");
}

// ========================================================================
// 27. i32::MAX 数量 — Decimal 可以处理
// ========================================================================

#[tokio::test]
async fn test_add_items_with_i32_max_quantity_rejected() {
    let manager = create_test_manager();

    let open_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::OpenTable {
            table_id: Some(314),
            table_name: Some("Table Max Qty".to_string()),
            zone_id: None,
            zone_name: None,
            guest_count: 1,
            is_retail: false,
        },
    );
    let resp = manager.execute_command(open_cmd).await;
    let order_id = resp.order_id.unwrap();

    let add_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::AddItems {
            order_id: order_id.clone(),
            items: vec![simple_item(1, "Max Qty", 0.01, i32::MAX)],
        },
    );
    let resp = manager.execute_command(add_cmd).await;
    assert!(
        !resp.success,
        "i32::MAX quantity exceeds max (9999), should be rejected"
    );
}

#[tokio::test]
async fn test_add_items_with_max_allowed_quantity() {
    let manager = create_test_manager();
    let order_id = open_table_with_items(
        &manager,
        216,
        vec![simple_item(1, "Max Allowed", 0.01, 9999)],
    )
    .await;

    let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert_eq!(snapshot.items[0].quantity, 9999);
    // 0.01 * 9999 = 99.99
    assert_eq!(snapshot.subtotal, 99.99);
}

// ========================================================================
// 28. 折扣超过 100% — unit_price clamp 到 0
// ========================================================================

#[tokio::test]
async fn test_add_items_with_discount_over_100_percent() {
    let manager = create_test_manager();

    // Open table
    let open_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::OpenTable {
            table_id: Some(315),
            table_name: Some("Table Over Discount".to_string()),
            zone_id: None,
            zone_name: None,
            guest_count: 1,
            is_retail: false,
        },
    );
    let resp = manager.execute_command(open_cmd).await;
    let order_id = resp.order_id.unwrap();

    // Add item with 200% discount
    let add_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::AddItems {
            order_id: order_id.clone(),
            items: vec![CartItemInput {
                product_id: 1,
                name: "Over Discounted".to_string(),
                price: 100.0,
                original_price: None,
                quantity: 1,
                selected_options: None,
                selected_specification: None,
                manual_discount_percent: Some(200.0),
                note: None,
                authorizer_id: None,
                authorizer_name: None,
            }],
        },
    );
    let resp = manager.execute_command(add_cmd).await;
    assert!(!resp.success, "200% discount should be rejected (max 100%)");
}

// ========================================================================
// 29. 负折扣 — 当前被接受 (相当于加价)
// ========================================================================

#[tokio::test]
async fn test_add_items_with_negative_discount_acts_as_markup() {
    let manager = create_test_manager();

    let open_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::OpenTable {
            table_id: Some(316),
            table_name: Some("Table Neg Discount".to_string()),
            zone_id: None,
            zone_name: None,
            guest_count: 1,
            is_retail: false,
        },
    );
    let resp = manager.execute_command(open_cmd).await;
    let order_id = resp.order_id.unwrap();

    let add_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::AddItems {
            order_id: order_id.clone(),
            items: vec![CartItemInput {
                product_id: 1,
                name: "Neg Discount Item".to_string(),
                price: 100.0,
                original_price: None,
                quantity: 1,
                selected_options: None,
                selected_specification: None,
                manual_discount_percent: Some(-50.0), // -50% = +50%
                note: None,
                authorizer_id: None,
                authorizer_name: None,
            }],
        },
    );
    let resp = manager.execute_command(add_cmd).await;
    assert!(
        !resp.success,
        "Negative discount should be rejected (min 0%)"
    );
}

// ========================================================================
// 30. 支付 NaN 金额 — 当前被 <= 0.0 检查通过 (NaN 比较特殊)
// ========================================================================

#[tokio::test]
async fn test_add_payment_with_nan_amount_rejected() {
    let manager = create_test_manager();
    let order_id =
        open_table_with_items(&manager, 217, vec![simple_item(1, "Coffee", 10.0, 1)]).await;

    let pay_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::AddPayment {
            order_id: order_id.clone(),
            payment: PaymentInput {
                method: "CASH".to_string(),
                amount: f64::NAN,
                tendered: None,
                note: None,
            },
        },
    );
    let resp = manager.execute_command(pay_cmd).await;
    assert!(!resp.success, "NaN payment amount should be rejected");
}

// ========================================================================
// 31. 支付 Infinity 金额 — 同样绕过 <= 0.0 检查
// ========================================================================

#[tokio::test]
async fn test_add_payment_with_infinity_amount_rejected() {
    let manager = create_test_manager();
    let order_id =
        open_table_with_items(&manager, 218, vec![simple_item(1, "Coffee", 10.0, 1)]).await;

    let pay_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::AddPayment {
            order_id: order_id.clone(),
            payment: PaymentInput {
                method: "CARD".to_string(),
                amount: f64::INFINITY,
                tendered: None,
                note: None,
            },
        },
    );
    let resp = manager.execute_command(pay_cmd).await;
    assert!(!resp.success, "Infinity payment amount should be rejected");
}

// ========================================================================
// 32. 支付 f64::MAX — 绕过检查，但 Decimal 转换为 0
// ========================================================================

#[tokio::test]
async fn test_add_payment_with_f64_max_amount_rejected() {
    let manager = create_test_manager();
    let order_id =
        open_table_with_items(&manager, 219, vec![simple_item(1, "Coffee", 10.0, 1)]).await;

    let pay_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::AddPayment {
            order_id: order_id.clone(),
            payment: PaymentInput {
                method: "CARD".to_string(),
                amount: f64::MAX,
                tendered: None,
                note: None,
            },
        },
    );
    let resp = manager.execute_command(pay_cmd).await;
    assert!(
        !resp.success,
        "f64::MAX payment should be rejected (exceeds max)"
    );
}

// ========================================================================
// 33. 多个极端商品叠加后完成订单
// ========================================================================

#[tokio::test]
async fn test_multiple_edge_items_then_complete() {
    let manager = create_test_manager();

    let open_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::OpenTable {
            table_id: Some(317),
            table_name: Some("Table Multi Edge".to_string()),
            zone_id: None,
            zone_name: None,
            guest_count: 1,
            is_retail: false,
        },
    );
    let resp = manager.execute_command(open_cmd).await;
    let order_id = resp.order_id.unwrap();

    // 正常商品 + 零价格商品
    let add_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::AddItems {
            order_id: order_id.clone(),
            items: vec![
                simple_item(1, "Normal", 25.50, 2),
                simple_item(2, "Free", 0.0, 1),
                simple_item(3, "Cheap", 0.01, 100),
            ],
        },
    );
    let resp = manager.execute_command(add_cmd).await;
    assert!(resp.success);

    let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
    // 25.50*2 + 0*1 + 0.01*100 = 51.0 + 0 + 1.0 = 52.0
    assert_eq!(snapshot.subtotal, 52.0);

    // 支付并完成
    let pay_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::AddPayment {
            order_id: order_id.clone(),
            payment: PaymentInput {
                method: "CASH".to_string(),
                amount: 52.0,
                tendered: Some(60.0),
                note: None,
            },
        },
    );
    manager.execute_command(pay_cmd).await;

    let complete_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::CompleteOrder {
            order_id: order_id.clone(),
            service_type: Some(ServiceType::DineIn),
        },
    );
    let resp = manager.execute_command(complete_cmd).await;
    assert!(resp.success);

    let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert_eq!(snapshot.status, OrderStatus::Completed);
    assert_eq!(snapshot.payments[0].change, Some(8.0)); // 60 - 52 = 8
}

// ========================================================================
// 34. 带选项价格修改器的边界测试
// ========================================================================

#[tokio::test]
async fn test_add_items_with_option_price_modifiers() {
    let manager = create_test_manager();

    let open_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::OpenTable {
            table_id: Some(318),
            table_name: Some("Table Opts".to_string()),
            zone_id: None,
            zone_name: None,
            guest_count: 1,
            is_retail: false,
        },
    );
    let resp = manager.execute_command(open_cmd).await;
    let order_id = resp.order_id.unwrap();

    let add_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::AddItems {
            order_id: order_id.clone(),
            items: vec![CartItemInput {
                product_id: 1,
                name: "Pizza".to_string(),
                price: 12.0,
                original_price: None,
                quantity: 1,
                selected_options: Some(vec![
                    shared::order::ItemOption {
                        attribute_id: 1,
                        attribute_name: "Size".to_string(),
                        option_id: 2,
                        option_name: "Large".to_string(),
                        price_modifier: Some(3.0), // +3
                        quantity: 1,
                    },
                    shared::order::ItemOption {
                        attribute_id: 2,
                        attribute_name: "Topping".to_string(),
                        option_id: 0,
                        option_name: "Extra Cheese".to_string(),
                        price_modifier: Some(1.50), // +1.50
                        quantity: 1,
                    },
                ]),
                selected_specification: None,
                manual_discount_percent: None,
                note: None,
                authorizer_id: None,
                authorizer_name: None,
            }],
        },
    );
    let resp = manager.execute_command(add_cmd).await;
    assert!(resp.success);

    let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
    // reducer: item_final = base(12) + options(3+1.50) = 16.50
    // money: original_price=12.0, options=4.50, base_with_options=16.50
    //   unit_price=16.50, line_total=16.50
    assert_eq!(snapshot.subtotal, 16.5);
}

// ========================================================================
// 35. 选项修改器为负值 — 当前被接受
// ========================================================================

#[tokio::test]
async fn test_add_items_with_negative_option_modifier() {
    let manager = create_test_manager();

    let open_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::OpenTable {
            table_id: Some(319),
            table_name: Some("Table Neg Opt".to_string()),
            zone_id: None,
            zone_name: None,
            guest_count: 1,
            is_retail: false,
        },
    );
    let resp = manager.execute_command(open_cmd).await;
    let order_id = resp.order_id.unwrap();

    let add_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::AddItems {
            order_id: order_id.clone(),
            items: vec![CartItemInput {
                product_id: 1,
                name: "Special".to_string(),
                price: 10.0,
                original_price: None,
                quantity: 1,
                selected_options: Some(vec![shared::order::ItemOption {
                    attribute_id: 3,
                    attribute_name: "Mod".to_string(),
                    option_id: 0,
                    option_name: "Smaller".to_string(),
                    price_modifier: Some(-15.0), // -15 使总价变负
                    quantity: 1,
                }]),
                selected_specification: None,
                manual_discount_percent: None,
                note: None,
                authorizer_id: None,
                authorizer_name: None,
            }],
        },
    );
    let resp = manager.execute_command(add_cmd).await;
    // 负的 price_modifier 是被允许的 (比如更小的规格减价)
    // 但不能超过 MAX_PRICE 的绝对值
    assert!(
        resp.success,
        "Negative option modifier within bounds is allowed"
    );

    let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
    // reducer: base=10+(-15)=-5, item_final=max(0,-5)=0
    // money: base_price=0, options=-15, base_with_options=-15 → clamped to 0
    assert_eq!(
        snapshot.subtotal, 0.0,
        "Negative modifier can reduce price to 0"
    );
}

// ========================================================================
// 37. 现金支付 tendered < amount 应被拒绝
// ========================================================================

#[tokio::test]
async fn test_add_cash_payment_tendered_less_than_amount_fails() {
    let manager = create_test_manager();
    let order_id =
        open_table_with_items(&manager, 220, vec![simple_item(1, "Coffee", 10.0, 1)]).await;

    let pay_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::AddPayment {
            order_id: order_id.clone(),
            payment: PaymentInput {
                method: "CASH".to_string(),
                amount: 10.0,
                tendered: Some(5.0), // 给了 5 块，要付 10 块
                note: None,
            },
        },
    );
    let resp = manager.execute_command(pay_cmd).await;
    assert!(
        !resp.success,
        "Tendered less than amount should be rejected"
    );
}

// ========================================================================
// 38. 折扣 + 附加费 + 选项叠加后精度测试
// ========================================================================

#[tokio::test]
async fn test_discount_surcharge_options_combined_precision() {
    let manager = create_test_manager();

    let open_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::OpenTable {
            table_id: Some(320),
            table_name: Some("Table Combo".to_string()),
            zone_id: None,
            zone_name: None,
            guest_count: 1,
            is_retail: false,
        },
    );
    let resp = manager.execute_command(open_cmd).await;
    let order_id = resp.order_id.unwrap();

    let add_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::AddItems {
            order_id: order_id.clone(),
            items: vec![CartItemInput {
                product_id: 1,
                name: "Combo Item".to_string(),
                price: 33.33,
                original_price: None,
                quantity: 3,
                selected_options: Some(vec![shared::order::ItemOption {
                    attribute_id: 1,
                    attribute_name: "Size".to_string(),
                    option_id: 1,
                    option_name: "Large".to_string(),
                    price_modifier: Some(1.67),
                    quantity: 1,
                }]),
                selected_specification: None,
                manual_discount_percent: Some(10.0), // 10% off
                note: None,
                authorizer_id: None,
                authorizer_name: None,
            }],
        },
    );
    let resp = manager.execute_command(add_cmd).await;
    assert!(resp.success);

    let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
    // reducer: base=33.33+1.67=35.0, discount=3.5, item_final=31.5
    // money: original_price=33.33, options=1.67, base_with_options=35.0
    //   manual_discount=35.0*10/100=3.5
    //   unit_price=35.0-3.5=31.5
    //   line_total=31.5*3=94.5
    assert_eq!(snapshot.subtotal, 94.5);
}

// ========================================================================
// 39. 支付 NaN 后尝试完成订单 — 应该失败
// ========================================================================

#[tokio::test]
async fn test_nan_payment_then_complete_fails() {
    let manager = create_test_manager();
    let order_id =
        open_table_with_items(&manager, 221, vec![simple_item(1, "Coffee", 10.0, 1)]).await;

    // NaN payment — 被输入验证拒绝
    let pay_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::AddPayment {
            order_id: order_id.clone(),
            payment: PaymentInput {
                method: "CASH".to_string(),
                amount: f64::NAN,
                tendered: None,
                note: None,
            },
        },
    );
    let pay_resp = manager.execute_command(pay_cmd).await;
    assert!(
        !pay_resp.success,
        "NaN payment should be rejected by validation"
    );

    // 尝试完成 — 应该失败因为没有成功的支付
    let complete_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::CompleteOrder {
            order_id: order_id.clone(),
            service_type: Some(ServiceType::DineIn),
        },
    );
    let resp = manager.execute_command(complete_cmd).await;
    assert!(!resp.success, "Should fail: no payment was recorded");
}

// ========================================================================
// 40. 快照重建一致性 — 带边界值
// ========================================================================

#[tokio::test]
async fn test_rebuild_snapshot_with_edge_values() {
    let manager = create_test_manager();

    let open_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::OpenTable {
            table_id: Some(321),
            table_name: Some("Table Rebuild Edge".to_string()),
            zone_id: None,
            zone_name: None,
            guest_count: 1,
            is_retail: false,
        },
    );
    let resp = manager.execute_command(open_cmd).await;
    let order_id = resp.order_id.unwrap();

    // 添加零价格商品
    let add_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::AddItems {
            order_id: order_id.clone(),
            items: vec![
                simple_item(1, "Free", 0.0, 5),
                simple_item(2, "Penny", 0.01, 99),
            ],
        },
    );
    manager.execute_command(add_cmd).await;

    let stored = manager.get_snapshot(&order_id).unwrap().unwrap();
    let rebuilt = manager.rebuild_snapshot(&order_id).unwrap();

    assert_eq!(stored.subtotal, rebuilt.subtotal);
    assert_eq!(stored.total, rebuilt.total);
    assert_eq!(stored.state_checksum, rebuilt.state_checksum);
    // 0*5 + 0.01*99 = 0.99
    assert_eq!(stored.subtotal, 0.99);
}

// ========================================================================
// 41. 批量小金额累加精度
// ========================================================================

#[tokio::test]
async fn test_many_small_amounts_precision() {
    let manager = create_test_manager();

    let open_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::OpenTable {
            table_id: Some(322),
            table_name: Some("Table Small".to_string()),
            zone_id: None,
            zone_name: None,
            guest_count: 1,
            is_retail: false,
        },
    );
    let resp = manager.execute_command(open_cmd).await;
    let order_id = resp.order_id.unwrap();

    // 添加 10 次，每次 1 个 0.1 的商品
    for i in 0i64..10 {
        let add_cmd = OrderCommand::new(
            1,
            "Test Operator".to_string(),
            OrderCommandPayload::AddItems {
                order_id: order_id.clone(),
                items: vec![CartItemInput {
                    product_id: i + 1,
                    name: format!("Item {}", i),
                    price: 0.1,
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
        manager.execute_command(add_cmd).await;
    }

    let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
    // 0.1 * 10 = 1.0 (使用 Decimal 精确计算)
    assert_eq!(snapshot.subtotal, 1.0, "10 x 0.1 should be exactly 1.0");
    assert_eq!(snapshot.total, 1.0);
}

// ========================================================================
// 42. NaN tendered — 对应 amount 为正值
// ========================================================================

#[tokio::test]
async fn test_add_cash_payment_nan_tendered() {
    let manager = create_test_manager();
    let order_id =
        open_table_with_items(&manager, 222, vec![simple_item(1, "Coffee", 10.0, 1)]).await;

    let pay_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::AddPayment {
            order_id: order_id.clone(),
            payment: PaymentInput {
                method: "CASH".to_string(),
                amount: 10.0,
                tendered: Some(f64::NAN), // NaN tendered
                note: None,
            },
        },
    );
    let resp = manager.execute_command(pay_cmd).await;
    // to_decimal(NaN) = 0, to_decimal(10.0) - 0.01 = 9.99
    // 0 < 9.99 → tendered 不足被拒绝
    assert!(!resp.success, "NaN tendered should fail: Decimal(0) < 9.99");
}

// ========================================================================
// 43. 移桌后仍可正常支付
// ========================================================================

#[tokio::test]
async fn test_add_payment_after_move_order() {
    let manager = create_test_manager();
    let order_id =
        open_table_with_items(&manager, 223, vec![simple_item(1, "Coffee", 10.0, 1)]).await;

    // Move order
    let move_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::MoveOrder {
            order_id: order_id.clone(),
            target_table_id: 330,
            target_table_name: "Table 2".to_string(),
            target_zone_id: None,
            target_zone_name: None,
            authorizer_id: None,
            authorizer_name: None,
        },
    );
    manager.execute_command(move_cmd).await;

    // MoveOrder 只移动桌台，订单保持 Active，仍可支付
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
    let resp = manager.execute_command(pay_cmd).await;
    assert!(
        resp.success,
        "Order should accept payments after MoveOrder (status stays Active)"
    );
}

// ========================================================================
// 44. 极小金额差异 — 支付容差边界
// ========================================================================

#[tokio::test]
async fn test_payment_tolerance_boundary() {
    let manager = create_test_manager();
    let order_id =
        open_table_with_items(&manager, 224, vec![simple_item(1, "Coffee", 10.0, 1)]).await;

    // 支付 9.99 — 差 0.01，在容差内
    let pay_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::AddPayment {
            order_id: order_id.clone(),
            payment: PaymentInput {
                method: "CARD".to_string(),
                amount: 9.99,
                tendered: None,
                note: None,
            },
        },
    );
    manager.execute_command(pay_cmd).await;

    let complete_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::CompleteOrder {
            order_id: order_id.clone(),
            service_type: Some(ServiceType::DineIn),
        },
    );
    let resp = manager.execute_command(complete_cmd).await;
    assert!(
        resp.success,
        "9.99 should be sufficient for 10.0 (within 0.01 tolerance)"
    );
}

#[tokio::test]
async fn test_payment_below_tolerance_rejected() {
    let manager = create_test_manager();
    let order_id =
        open_table_with_items(&manager, 225, vec![simple_item(1, "Coffee", 10.0, 1)]).await;

    // 支付 9.98 — 差 0.02，超出容差
    let pay_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::AddPayment {
            order_id: order_id.clone(),
            payment: PaymentInput {
                method: "CARD".to_string(),
                amount: 9.98,
                tendered: None,
                note: None,
            },
        },
    );
    manager.execute_command(pay_cmd).await;

    let complete_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::CompleteOrder {
            order_id: order_id.clone(),
            service_type: Some(ServiceType::DineIn),
        },
    );
    let resp = manager.execute_command(complete_cmd).await;
    assert!(
        !resp.success,
        "9.98 should be insufficient for 10.0 (outside 0.01 tolerance)"
    );
}

// ========================================================================
// 41. 多选项 + 手动折扣 + 规则字段: 端到端精度验证 (无双重计算)
// ========================================================================

#[tokio::test]
async fn test_options_discount_rule_fields_no_double_counting() {
    let manager = create_test_manager();

    let open_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::OpenTable {
            table_id: Some(323),
            table_name: Some("Table No Double".to_string()),
            zone_id: None,
            zone_name: None,
            guest_count: 1,
            is_retail: false,
        },
    );
    let resp = manager.execute_command(open_cmd).await;
    let order_id = resp.order_id.unwrap();

    // Item: price=20.0, options=+3.0+2.0=5.0, discount=10%
    // Expected: base_with_options=25.0, discount=2.5, unit_price=22.5
    // qty=2 → subtotal=45.0
    let add_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::AddItems {
            order_id: order_id.clone(),
            items: vec![CartItemInput {
                product_id: 1,
                name: "Steak".to_string(),
                price: 20.0,
                original_price: None,
                quantity: 2,
                selected_options: Some(vec![
                    shared::order::ItemOption {
                        attribute_id: 4,
                        attribute_name: "Sauce".to_string(),
                        option_id: 0,
                        option_name: "BBQ".to_string(),
                        price_modifier: Some(3.0),
                        quantity: 1,
                    },
                    shared::order::ItemOption {
                        attribute_id: 5,
                        attribute_name: "Side".to_string(),
                        option_id: 1,
                        option_name: "Fries".to_string(),
                        price_modifier: Some(2.0),
                        quantity: 1,
                    },
                ]),
                selected_specification: None,
                manual_discount_percent: Some(10.0),
                note: None,
                authorizer_id: None,
                authorizer_name: None,
            }],
        },
    );
    let resp = manager.execute_command(add_cmd).await;
    assert!(resp.success);

    let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
    let item = &snapshot.items[0];

    // original_price should be set to the input price (spec price)
    assert_eq!(item.original_price, 20.0, "original_price = input price");

    // unit_price: base(20)+options(5)=25, discount=25*10%=2.5, unit=22.5
    assert_eq!(
        item.unit_price, 22.5,
        "unit_price = 22.5 (no double counting)"
    );

    // subtotal = 22.5 * 2 = 45.0
    assert_eq!(snapshot.subtotal, 45.0, "subtotal = 45.0");
    assert_eq!(snapshot.total, 45.0, "total = 45.0 (no tax)");
}

// ========================================================================
// 42. ModifyItem 后 unit_price 一致性
// ========================================================================

#[tokio::test]
async fn test_modify_item_unit_price_consistency() {
    let manager = create_test_manager();

    let open_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::OpenTable {
            table_id: Some(324),
            table_name: Some("Table Mod Consistency".to_string()),
            zone_id: None,
            zone_name: None,
            guest_count: 1,
            is_retail: false,
        },
    );
    let resp = manager.execute_command(open_cmd).await;
    let order_id = resp.order_id.unwrap();

    // Add item: price=15.0, options=+2.5, no discount, qty=3
    let add_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::AddItems {
            order_id: order_id.clone(),
            items: vec![CartItemInput {
                product_id: 1,
                name: "Pasta".to_string(),
                price: 15.0,
                original_price: None,
                quantity: 3,
                selected_options: Some(vec![shared::order::ItemOption {
                    attribute_id: 6,
                    attribute_name: "Cheese".to_string(),
                    option_id: 0,
                    option_name: "Extra".to_string(),
                    price_modifier: Some(2.5),
                    quantity: 1,
                }]),
                selected_specification: None,
                manual_discount_percent: None,
                note: None,
                authorizer_id: None,
                authorizer_name: None,
            }],
        },
    );
    let resp = manager.execute_command(add_cmd).await;
    assert!(resp.success);

    let snapshot_before = manager.get_snapshot(&order_id).unwrap().unwrap();
    let instance_id = snapshot_before.items[0].instance_id.clone();
    // unit_price = 15 + 2.5 = 17.5, subtotal = 17.5 * 3 = 52.5
    assert_eq!(snapshot_before.items[0].unit_price, 17.5);
    assert_eq!(snapshot_before.subtotal, 52.5);

    // Modify: add 20% discount
    let modify_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::ModifyItem {
            order_id: order_id.clone(),
            instance_id,
            affected_quantity: None,
            changes: shared::order::ItemChanges {
                price: None,
                quantity: None,
                manual_discount_percent: Some(20.0),
                note: None,
                selected_options: None,
                selected_specification: None,
            },
            authorizer_id: None,
            authorizer_name: None,
        },
    );
    let resp = manager.execute_command(modify_cmd).await;
    assert!(resp.success, "ModifyItem should succeed");

    let snapshot_after = manager.get_snapshot(&order_id).unwrap().unwrap();
    let item = &snapshot_after.items[0];

    // original_price should still be 15.0
    assert_eq!(
        item.original_price, 15.0,
        "original_price unchanged after modify"
    );

    // unit_price: base(15)+options(2.5)=17.5, discount=17.5*20%=3.5, unit=14.0
    assert_eq!(item.unit_price, 14.0, "unit_price after 20% discount");

    // subtotal = 14.0 * 3 = 42.0
    assert_eq!(snapshot_after.subtotal, 42.0, "subtotal after modify");
    assert_eq!(snapshot_after.total, 42.0, "total after modify");
}

// ========================================================================
// ========================================================================
//  恶意数据防御 + 死胡同预防测试
// ========================================================================
// ========================================================================

// ========================================================================
// 状态守卫: Voided 订单不可操作
// ========================================================================

#[tokio::test]
async fn test_add_items_to_voided_order_fails() {
    let manager = create_test_manager();
    let order_id =
        open_table_with_items(&manager, 226, vec![simple_item(1, "Coffee", 10.0, 1)]).await;

    // Void
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
    manager.execute_command(void_cmd).await;

    // Try to add items
    let add_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::AddItems {
            order_id: order_id.clone(),
            items: vec![simple_item(2, "Tea", 5.0, 1)],
        },
    );
    let resp = manager.execute_command(add_cmd).await;
    assert!(!resp.success, "Should not add items to voided order");
}

#[tokio::test]
async fn test_add_payment_to_voided_order_fails() {
    let manager = create_test_manager();
    let order_id =
        open_table_with_items(&manager, 227, vec![simple_item(1, "Coffee", 10.0, 1)]).await;

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
    manager.execute_command(void_cmd).await;

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
    let resp = manager.execute_command(pay_cmd).await;
    assert!(!resp.success, "Should not add payment to voided order");
}

#[tokio::test]
async fn test_complete_voided_order_fails() {
    let manager = create_test_manager();
    let order_id = open_table_with_items(&manager, 102, vec![]).await;

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
    manager.execute_command(void_cmd).await;

    let complete_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::CompleteOrder {
            order_id: order_id.clone(),
            service_type: Some(ServiceType::DineIn),
        },
    );
    let resp = manager.execute_command(complete_cmd).await;
    assert!(!resp.success, "Should not complete a voided order");
}

// ========================================================================
// 状态守卫: Completed 订单不可 void
// ========================================================================

#[tokio::test]
async fn test_void_completed_order_fails() {
    let manager = create_test_manager();
    let order_id =
        open_table_with_items(&manager, 228, vec![simple_item(1, "Coffee", 10.0, 1)]).await;

    // Pay + complete
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
    manager.execute_command(pay_cmd).await;

    let complete_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::CompleteOrder {
            order_id: order_id.clone(),
            service_type: Some(ServiceType::DineIn),
        },
    );
    manager.execute_command(complete_cmd).await;

    // Try to void
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
    let resp = manager.execute_command(void_cmd).await;
    assert!(!resp.success, "Should not void a completed order");
}

#[tokio::test]
async fn test_double_complete_fails() {
    let manager = create_test_manager();
    let order_id =
        open_table_with_items(&manager, 229, vec![simple_item(1, "Coffee", 10.0, 1)]).await;

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
    manager.execute_command(pay_cmd).await;

    let complete_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::CompleteOrder {
            order_id: order_id.clone(),
            service_type: Some(ServiceType::DineIn),
        },
    );
    let resp1 = manager.execute_command(complete_cmd).await;
    assert!(resp1.success);

    let complete_cmd2 = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::CompleteOrder {
            order_id: order_id.clone(),
            service_type: Some(ServiceType::DineIn),
        },
    );
    let resp2 = manager.execute_command(complete_cmd2).await;
    assert!(!resp2.success, "Double complete should fail");
}

// ========================================================================
// 恶意 ModifyItem 数据
// ========================================================================

#[tokio::test]
async fn test_modify_item_nan_price_rejected() {
    let manager = create_test_manager();
    let order_id =
        open_table_with_items(&manager, 230, vec![simple_item(1, "Coffee", 10.0, 1)]).await;

    let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
    let instance_id = snapshot.items[0].instance_id.clone();

    let modify_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::ModifyItem {
            order_id: order_id.clone(),
            instance_id,
            affected_quantity: None,
            changes: shared::order::ItemChanges {
                price: Some(f64::NAN),
                quantity: None,
                manual_discount_percent: None,
                note: None,
                selected_options: None,
                selected_specification: None,
            },
            authorizer_id: None,
            authorizer_name: None,
        },
    );
    let resp = manager.execute_command(modify_cmd).await;
    assert!(
        !resp.success,
        "ModifyItem with NaN price should be rejected"
    );
}

#[tokio::test]
async fn test_modify_item_negative_price_rejected() {
    let manager = create_test_manager();
    let order_id =
        open_table_with_items(&manager, 231, vec![simple_item(1, "Coffee", 10.0, 1)]).await;

    let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
    let instance_id = snapshot.items[0].instance_id.clone();

    let modify_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::ModifyItem {
            order_id: order_id.clone(),
            instance_id,
            affected_quantity: None,
            changes: shared::order::ItemChanges {
                price: Some(-50.0),
                quantity: None,
                manual_discount_percent: None,
                note: None,
                selected_options: None,
                selected_specification: None,
            },
            authorizer_id: None,
            authorizer_name: None,
        },
    );
    let resp = manager.execute_command(modify_cmd).await;
    assert!(
        !resp.success,
        "ModifyItem with negative price should be rejected"
    );
}

#[tokio::test]
async fn test_modify_item_nan_discount_rejected() {
    let manager = create_test_manager();
    let order_id =
        open_table_with_items(&manager, 232, vec![simple_item(1, "Coffee", 10.0, 1)]).await;

    let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
    let instance_id = snapshot.items[0].instance_id.clone();

    let modify_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::ModifyItem {
            order_id: order_id.clone(),
            instance_id,
            affected_quantity: None,
            changes: shared::order::ItemChanges {
                price: None,
                quantity: None,
                manual_discount_percent: Some(f64::NAN),
                note: None,
                selected_options: None,
                selected_specification: None,
            },
            authorizer_id: None,
            authorizer_name: None,
        },
    );
    let resp = manager.execute_command(modify_cmd).await;
    assert!(
        !resp.success,
        "ModifyItem with NaN discount should be rejected"
    );
}

#[tokio::test]
async fn test_modify_item_discount_over_100_rejected() {
    let manager = create_test_manager();
    let order_id =
        open_table_with_items(&manager, 233, vec![simple_item(1, "Coffee", 10.0, 1)]).await;

    let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
    let instance_id = snapshot.items[0].instance_id.clone();

    let modify_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::ModifyItem {
            order_id: order_id.clone(),
            instance_id,
            affected_quantity: None,
            changes: shared::order::ItemChanges {
                price: None,
                quantity: None,
                manual_discount_percent: Some(150.0),
                note: None,
                selected_options: None,
                selected_specification: None,
            },
            authorizer_id: None,
            authorizer_name: None,
        },
    );
    let resp = manager.execute_command(modify_cmd).await;
    assert!(
        !resp.success,
        "ModifyItem with 150% discount should be rejected"
    );
}

#[tokio::test]
async fn test_modify_item_zero_quantity_rejected() {
    let manager = create_test_manager();
    let order_id =
        open_table_with_items(&manager, 234, vec![simple_item(1, "Coffee", 10.0, 2)]).await;

    let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
    let instance_id = snapshot.items[0].instance_id.clone();

    let modify_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::ModifyItem {
            order_id: order_id.clone(),
            instance_id,
            affected_quantity: None,
            changes: shared::order::ItemChanges {
                price: None,
                quantity: Some(0),
                manual_discount_percent: None,
                note: None,
                selected_options: None,
                selected_specification: None,
            },
            authorizer_id: None,
            authorizer_name: None,
        },
    );
    let resp = manager.execute_command(modify_cmd).await;
    assert!(
        !resp.success,
        "ModifyItem with quantity=0 should be rejected"
    );
}

// ========================================================================
// 空 items 数组攻击
// ========================================================================

#[tokio::test]
async fn test_add_empty_items_array() {
    let manager = create_test_manager();
    let order_id = open_table_with_items(&manager, 103, vec![]).await;

    let add_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::AddItems {
            order_id: order_id.clone(),
            items: vec![], // Empty array
        },
    );
    let _resp = manager.execute_command(add_cmd).await;
    // 即使 AddItems 允许空数组（当前行为），订单不应进入不一致状态
    // 记录当前行为，不管成功与否，订单仍可继续操作
    let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert_eq!(
        snapshot.status,
        OrderStatus::Active,
        "Order should remain Active"
    );
    assert_eq!(snapshot.items.len(), 0, "No items should be added");

    // 验证可以继续添加正常商品 (不进入死胡同)
    let add_cmd2 = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::AddItems {
            order_id: order_id.clone(),
            items: vec![simple_item(1, "Coffee", 10.0, 1)],
        },
    );
    let resp2 = manager.execute_command(add_cmd2).await;
    assert!(
        resp2.success,
        "Should be able to add items after empty array"
    );
}

// ========================================================================
// 合并操作: 无效目标
// ========================================================================

#[tokio::test]
async fn test_merge_voided_source_fails() {
    let manager = create_test_manager();
    let source_id = open_table_with_items(&manager, 104, vec![]).await;
    let target_id = open_table_with_items(&manager, 105, vec![]).await;

    // Void source
    let void_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::VoidOrder {
            order_id: source_id.clone(),
            void_type: VoidType::Cancelled,
            loss_reason: None,
            loss_amount: None,
            note: None,
            authorizer_id: None,
            authorizer_name: None,
        },
    );
    manager.execute_command(void_cmd).await;

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
    let resp = manager.execute_command(merge_cmd).await;
    assert!(!resp.success, "Should not merge a voided source order");
}

#[tokio::test]
async fn test_merge_into_voided_target_fails() {
    let manager = create_test_manager();
    let source_id = open_table_with_items(&manager, 106, vec![]).await;
    let target_id = open_table_with_items(&manager, 107, vec![]).await;

    // Void target
    let void_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::VoidOrder {
            order_id: target_id.clone(),
            void_type: VoidType::Cancelled,
            loss_reason: None,
            loss_amount: None,
            note: None,
            authorizer_id: None,
            authorizer_name: None,
        },
    );
    manager.execute_command(void_cmd).await;

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
    let resp = manager.execute_command(merge_cmd).await;
    assert!(!resp.success, "Should not merge into a voided target order");
}

#[tokio::test]
async fn test_merge_self_fails() {
    let manager = create_test_manager();
    let order_id = open_table_with_items(&manager, 108, vec![]).await;

    let merge_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::MergeOrders {
            source_order_id: order_id.clone(),
            target_order_id: order_id.clone(),
            authorizer_id: None,
            authorizer_name: None,
        },
    );
    let resp = manager.execute_command(merge_cmd).await;
    assert!(!resp.success, "Should not merge order with itself");
}

// ========================================================================
// AA Split 恶意数据
// ========================================================================

#[tokio::test]
async fn test_aa_split_zero_total_shares_fails() {
    let manager = create_test_manager();
    let order_id =
        open_table_with_items(&manager, 235, vec![simple_item(1, "Coffee", 30.0, 1)]).await;

    let cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::StartAaSplit {
            order_id: order_id.clone(),
            total_shares: 0, // Invalid
            shares: 0,
            payment_method: "CASH".to_string(),
            tendered: None,
        },
    );
    let resp = manager.execute_command(cmd).await;
    assert!(!resp.success, "AA split with 0 total shares should fail");
}

#[tokio::test]
async fn test_aa_split_one_share_fails() {
    let manager = create_test_manager();
    let order_id =
        open_table_with_items(&manager, 236, vec![simple_item(1, "Coffee", 30.0, 1)]).await;

    let cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::StartAaSplit {
            order_id: order_id.clone(),
            total_shares: 1, // Must be >= 2
            shares: 1,
            payment_method: "CASH".to_string(),
            tendered: None,
        },
    );
    let resp = manager.execute_command(cmd).await;
    assert!(
        !resp.success,
        "AA split with 1 total share should fail (need >= 2)"
    );
}

#[tokio::test]
async fn test_aa_split_shares_exceed_total_fails() {
    let manager = create_test_manager();
    let order_id =
        open_table_with_items(&manager, 237, vec![simple_item(1, "Coffee", 30.0, 1)]).await;

    let cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::StartAaSplit {
            order_id: order_id.clone(),
            total_shares: 3,
            shares: 5, // More than total
            payment_method: "CASH".to_string(),
            tendered: None,
        },
    );
    let resp = manager.execute_command(cmd).await;
    assert!(!resp.success, "AA split shares > total_shares should fail");
}

#[tokio::test]
async fn test_pay_aa_split_exceed_remaining_fails() {
    let manager = create_test_manager();
    let order_id =
        open_table_with_items(&manager, 238, vec![simple_item(1, "Coffee", 30.0, 1)]).await;

    // Start AA: 3 shares, pay 2
    let start_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::StartAaSplit {
            order_id: order_id.clone(),
            total_shares: 3,
            shares: 2,
            payment_method: "CASH".to_string(),
            tendered: None,
        },
    );
    let resp = manager.execute_command(start_cmd).await;
    assert!(resp.success);

    // Try to pay 3 more shares (only 1 remaining)
    let pay_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::PayAaSplit {
            order_id: order_id.clone(),
            shares: 3,
            payment_method: "CASH".to_string(),
            tendered: None,
        },
    );
    let resp = manager.execute_command(pay_cmd).await;
    assert!(
        !resp.success,
        "Pay AA split with shares > remaining should fail"
    );
}

// ========================================================================
// 取消已取消的支付
// ========================================================================

#[tokio::test]
async fn test_cancel_already_cancelled_payment_fails() {
    let manager = create_test_manager();
    let order_id =
        open_table_with_items(&manager, 239, vec![simple_item(1, "Coffee", 10.0, 1)]).await;

    // Pay
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
    manager.execute_command(pay_cmd).await;

    let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
    let payment_id = snapshot.payments[0].payment_id.clone();

    // Cancel once
    let cancel1 = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::CancelPayment {
            order_id: order_id.clone(),
            payment_id: payment_id.clone(),
            reason: Some("mistake".to_string()),
            authorizer_id: None,
            authorizer_name: None,
        },
    );
    let resp1 = manager.execute_command(cancel1).await;
    assert!(resp1.success);

    // Cancel again (should fail)
    let cancel2 = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::CancelPayment {
            order_id: order_id.clone(),
            payment_id,
            reason: Some("again".to_string()),
            authorizer_id: None,
            authorizer_name: None,
        },
    );
    let resp2 = manager.execute_command(cancel2).await;
    assert!(
        !resp2.success,
        "Should not cancel an already-cancelled payment"
    );
}

// ========================================================================
// 移桌到已占用桌台
// ========================================================================

#[tokio::test]
async fn test_move_to_occupied_table_fails() {
    let manager = create_test_manager();
    let _order1 =
        open_table_with_items(&manager, 240, vec![simple_item(1, "Coffee", 10.0, 1)]).await;
    let order2 = open_table_with_items(&manager, 241, vec![simple_item(2, "Tea", 5.0, 1)]).await;

    // Move order2 to T-occ-1 (occupied)
    let move_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::MoveOrder {
            order_id: order2.clone(),
            target_table_id: 240,
            target_table_name: "Table 1".to_string(),
            target_zone_id: None,
            target_zone_name: None,
            authorizer_id: None,
            authorizer_name: None,
        },
    );
    let resp = manager.execute_command(move_cmd).await;
    assert!(!resp.success, "Should not move to an occupied table");
}

// ========================================================================
// ModifyItem 对已完成/已取消订单
// ========================================================================

#[tokio::test]
async fn test_modify_item_on_completed_order_fails() {
    let manager = create_test_manager();
    let order_id =
        open_table_with_items(&manager, 242, vec![simple_item(1, "Coffee", 10.0, 1)]).await;

    let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
    let instance_id = snapshot.items[0].instance_id.clone();

    // Pay + complete
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
    manager.execute_command(pay_cmd).await;
    let complete_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::CompleteOrder {
            order_id: order_id.clone(),
            service_type: Some(ServiceType::DineIn),
        },
    );
    manager.execute_command(complete_cmd).await;

    // Try to modify
    let modify_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::ModifyItem {
            order_id: order_id.clone(),
            instance_id,
            affected_quantity: None,
            changes: shared::order::ItemChanges {
                price: Some(999.0),
                quantity: None,
                manual_discount_percent: None,
                note: None,
                selected_options: None,
                selected_specification: None,
            },
            authorizer_id: None,
            authorizer_name: None,
        },
    );
    let resp = manager.execute_command(modify_cmd).await;
    assert!(!resp.success, "Should not modify items on completed order");
}

// ========================================================================
// 支付负金额
// ========================================================================

#[tokio::test]
async fn test_add_payment_negative_amount_rejected() {
    let manager = create_test_manager();
    let order_id =
        open_table_with_items(&manager, 243, vec![simple_item(1, "Coffee", 10.0, 1)]).await;

    let pay_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::AddPayment {
            order_id: order_id.clone(),
            payment: PaymentInput {
                method: "CASH".to_string(),
                amount: -10.0,
                tendered: None,
                note: None,
            },
        },
    );
    let resp = manager.execute_command(pay_cmd).await;
    assert!(!resp.success, "Negative payment amount should be rejected");
}

#[tokio::test]
async fn test_add_payment_zero_amount_rejected() {
    let manager = create_test_manager();
    let order_id =
        open_table_with_items(&manager, 244, vec![simple_item(1, "Coffee", 10.0, 1)]).await;

    let pay_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::AddPayment {
            order_id: order_id.clone(),
            payment: PaymentInput {
                method: "CASH".to_string(),
                amount: 0.0,
                tendered: None,
                note: None,
            },
        },
    );
    let resp = manager.execute_command(pay_cmd).await;
    assert!(!resp.success, "Zero payment amount should be rejected");
}
