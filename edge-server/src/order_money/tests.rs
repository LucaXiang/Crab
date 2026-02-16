use super::*;

#[test]
fn test_to_decimal_precision() {
    // Classic floating point problem: 0.1 + 0.2 != 0.3
    let a = 0.1_f64;
    let b = 0.2_f64;
    let sum_f64 = a + b;

    // f64 fails
    assert_ne!(sum_f64, 0.3);

    // Decimal succeeds
    let sum_dec = to_decimal(a) + to_decimal(b);
    assert_eq!(to_f64(sum_dec), 0.3);
}

#[test]
fn test_accumulation_precision() {
    // Sum 0.01 one thousand times
    let mut total = Decimal::ZERO;
    for _ in 0..1000 {
        total += to_decimal(0.01);
    }
    assert_eq!(to_f64(total), 10.0);
}

#[test]
fn test_calculate_item_total_no_discount() {
    let item = CartItemSnapshot {
        id: 1,
        instance_id: "i1".to_string(),
        name: "Item".to_string(),
        price: 10.99,
        original_price: 0.0,
        quantity: 3,
        unpaid_quantity: 3,
        selected_options: None,
        selected_specification: None,
        manual_discount_percent: None,
        rule_discount_amount: 0.0,
        rule_surcharge_amount: 0.0,
        applied_rules: vec![],
        applied_mg_rules: vec![],
        mg_discount_amount: 0.0,
        note: None,
        authorizer_id: None,
        authorizer_name: None,
        category_id: None,
        category_name: None,
        is_comped: false,
        unit_price: 0.0,
        line_total: 0.0,
        tax: 0.0,
        tax_rate: 0,
    };

    let total = calculate_item_total(&item);
    assert_eq!(to_f64(total), 32.97); // 10.99 * 3
}

#[test]
fn test_calculate_item_total_with_discount() {
    let item = CartItemSnapshot {
        id: 1,
        instance_id: "i1".to_string(),
        name: "Item".to_string(),
        price: 100.0,
        original_price: 0.0,
        quantity: 1,
        unpaid_quantity: 1,
        selected_options: None,
        selected_specification: None,
        manual_discount_percent: Some(33.33), // Tricky percentage
        rule_discount_amount: 0.0,
        rule_surcharge_amount: 0.0,
        applied_rules: vec![],
        applied_mg_rules: vec![],
        mg_discount_amount: 0.0,
        note: None,
        authorizer_id: None,
        authorizer_name: None,
        category_id: None,
        category_name: None,
        is_comped: false,
        unit_price: 0.0,
        line_total: 0.0,
        tax: 0.0,
        tax_rate: 0,
    };

    let total = calculate_item_total(&item);
    assert_eq!(to_f64(total), 66.67); // 100 * (1 - 0.3333) = 66.67
}

#[test]
fn test_calculate_item_total_33_percent_discount() {
    // Edge case: 33% discount on $100 should be $67.00
    let item = CartItemSnapshot {
        id: 1,
        instance_id: "i1".to_string(),
        name: "Item".to_string(),
        price: 100.0,
        original_price: 0.0,
        quantity: 1,
        unpaid_quantity: 1,
        selected_options: None,
        selected_specification: None,
        manual_discount_percent: Some(33.0),
        rule_discount_amount: 0.0,
        rule_surcharge_amount: 0.0,
        applied_rules: vec![],
        applied_mg_rules: vec![],
        mg_discount_amount: 0.0,
        note: None,
        authorizer_id: None,
        authorizer_name: None,
        category_id: None,
        category_name: None,
        is_comped: false,
        unit_price: 0.0,
        line_total: 0.0,
        tax: 0.0,
        tax_rate: 0,
    };

    let total = calculate_item_total(&item);
    assert_eq!(to_f64(total), 67.0);
}

#[test]
fn test_is_payment_sufficient() {
    assert!(is_payment_sufficient(100.0, 100.0));
    assert!(is_payment_sufficient(100.01, 100.0));
    assert!(is_payment_sufficient(99.995, 100.0)); // Within tolerance
    assert!(!is_payment_sufficient(99.98, 100.0)); // Outside tolerance
}

#[test]
fn test_money_eq() {
    assert!(money_eq(100.0, 100.0));
    assert!(money_eq(100.004, 100.006)); // Both round to 100.00/100.01
    assert!(!money_eq(100.0, 100.02));
}

#[test]
fn test_rounding_half_up() {
    // 0.005 should round up to 0.01
    let value = Decimal::new(5, 3); // 0.005
    let rounded = value.round_dp_with_strategy(2, RoundingStrategy::MidpointAwayFromZero);
    assert_eq!(rounded.to_f64().unwrap(), 0.01);

    // 0.004 should round down to 0.00
    let value2 = Decimal::new(4, 3); // 0.004
    let rounded2 = value2.round_dp_with_strategy(2, RoundingStrategy::MidpointAwayFromZero);
    assert_eq!(rounded2.to_f64().unwrap(), 0.0);
}

#[test]
fn test_many_small_items() {
    // 100 items at $0.01 each
    let items: Vec<CartItemSnapshot> = (0..100)
        .map(|i| CartItemSnapshot {
            id: i as i64,
            instance_id: format!("i{}", i),
            name: "Penny Item".to_string(),
            price: 0.01,
            original_price: 0.0,
            quantity: 1,
            unpaid_quantity: 1,
            selected_options: None,
            selected_specification: None,
            manual_discount_percent: None,
            rule_discount_amount: 0.0,
            rule_surcharge_amount: 0.0,
            applied_rules: vec![],
            applied_mg_rules: vec![],
            mg_discount_amount: 0.0,
            note: None,
            authorizer_id: None,
            authorizer_name: None,
            category_id: None,
            category_name: None,
            is_comped: false,
            unit_price: 0.0,
            line_total: 0.0,
            tax: 0.0,
            tax_rate: 0,
        })
        .collect();

    let total: Decimal = items.iter().map(|i| calculate_item_total(i)).sum();
    assert_eq!(to_f64(total), 1.0);
}

#[test]
fn test_is_pre_payment_reset_when_total_changes() {
    use shared::order::OrderSnapshot;

    let mut snapshot = OrderSnapshot::new("order-1".to_string());
    snapshot.items.push(CartItemSnapshot {
        id: 1,
        instance_id: "i1".to_string(),
        name: "Item".to_string(),
        price: 100.0,
        original_price: 0.0,
        quantity: 1,
        unpaid_quantity: 1,
        selected_options: None,
        selected_specification: None,
        manual_discount_percent: None,
        rule_discount_amount: 0.0,
        rule_surcharge_amount: 0.0,
        applied_rules: vec![],
        applied_mg_rules: vec![],
        mg_discount_amount: 0.0,
        note: None,
        authorizer_id: None,
        authorizer_name: None,
        category_id: None,
        category_name: None,
        is_comped: false,
        unit_price: 0.0,
        line_total: 0.0,
        tax: 0.0,
        tax_rate: 0,
    });

    // Initial calculation
    recalculate_totals(&mut snapshot);
    assert_eq!(snapshot.total, 100.0);

    // Set pre-payment flag (simulating prepaid receipt printed)
    snapshot.is_pre_payment = true;

    // Recalculate without changing items - total unchanged, is_pre_payment stays true
    recalculate_totals(&mut snapshot);
    assert!(
        snapshot.is_pre_payment,
        "is_pre_payment should stay true when total unchanged"
    );

    // Add another item - total changes, is_pre_payment should reset
    snapshot.items.push(CartItemSnapshot {
        id: 2,
        instance_id: "i2".to_string(),
        name: "Item 2".to_string(),
        price: 50.0,
        original_price: 0.0,
        quantity: 1,
        unpaid_quantity: 1,
        selected_options: None,
        selected_specification: None,
        manual_discount_percent: None,
        rule_discount_amount: 0.0,
        rule_surcharge_amount: 0.0,
        applied_rules: vec![],
        applied_mg_rules: vec![],
        mg_discount_amount: 0.0,
        note: None,
        authorizer_id: None,
        authorizer_name: None,
        category_id: None,
        category_name: None,
        is_comped: false,
        unit_price: 0.0,
        line_total: 0.0,
        tax: 0.0,
        tax_rate: 0,
    });

    recalculate_totals(&mut snapshot);
    assert_eq!(snapshot.total, 150.0);
    assert!(
        !snapshot.is_pre_payment,
        "is_pre_payment should reset when total changes"
    );
}

#[test]
fn test_is_pre_payment_not_affected_when_false() {
    use shared::order::OrderSnapshot;

    let mut snapshot = OrderSnapshot::new("order-1".to_string());
    snapshot.items.push(CartItemSnapshot {
        id: 1,
        instance_id: "i1".to_string(),
        name: "Item".to_string(),
        price: 100.0,
        original_price: 0.0,
        quantity: 1,
        unpaid_quantity: 1,
        selected_options: None,
        selected_specification: None,
        manual_discount_percent: None,
        rule_discount_amount: 0.0,
        rule_surcharge_amount: 0.0,
        applied_rules: vec![],
        applied_mg_rules: vec![],
        mg_discount_amount: 0.0,
        note: None,
        authorizer_id: None,
        authorizer_name: None,
        category_id: None,
        category_name: None,
        is_comped: false,
        unit_price: 0.0,
        line_total: 0.0,
        tax: 0.0,
        tax_rate: 0,
    });

    // is_pre_payment is false by default
    assert!(!snapshot.is_pre_payment);

    recalculate_totals(&mut snapshot);

    // Add item and recalculate - is_pre_payment should stay false
    snapshot.items[0].price = 200.0;
    recalculate_totals(&mut snapshot);

    assert!(!snapshot.is_pre_payment, "is_pre_payment should stay false");
}

// ========================================================================
// Decimal 转换边界测试
// ========================================================================

#[test]
fn test_to_decimal_nan_becomes_zero() {
    // NaN 被 Decimal::from_f64 拒绝，unwrap_or_default 返回 0
    let result = to_decimal(f64::NAN);
    assert_eq!(result, Decimal::ZERO, "NaN should silently convert to 0");
}

#[test]
fn test_to_decimal_infinity_becomes_zero() {
    let result = to_decimal(f64::INFINITY);
    assert_eq!(
        result,
        Decimal::ZERO,
        "INFINITY should silently convert to 0"
    );

    let result_neg = to_decimal(f64::NEG_INFINITY);
    assert_eq!(
        result_neg,
        Decimal::ZERO,
        "NEG_INFINITY should silently convert to 0"
    );
}

#[test]
fn test_to_decimal_f64_max_becomes_zero() {
    // f64::MAX 超出 Decimal 范围
    let result = to_decimal(f64::MAX);
    assert_eq!(
        result,
        Decimal::ZERO,
        "f64::MAX should silently convert to 0"
    );
}

#[test]
fn test_to_decimal_f64_min_becomes_zero() {
    let result = to_decimal(f64::MIN);
    assert_eq!(
        result,
        Decimal::ZERO,
        "f64::MIN should silently convert to 0"
    );
}

#[test]
fn test_to_decimal_negative_price() {
    // 负价格被正常转换 (不会被拒绝)
    let result = to_decimal(-10.0);
    assert_eq!(result, Decimal::new(-10, 0));
}

#[test]
fn test_to_decimal_very_large_but_valid() {
    // 1_000_000_000.99 在 Decimal 范围内
    let result = to_decimal(1_000_000_000.99);
    assert!(
        result > Decimal::ZERO,
        "Large but valid f64 should convert normally"
    );
}

// ========================================================================
// calculate_unit_price 边界测试
// ========================================================================

#[test]
fn test_unit_price_negative_base_clamped_to_zero() {
    let item = CartItemSnapshot {
        id: 1,
        instance_id: "i1".to_string(),
        name: "Item".to_string(),
        price: -50.0,
        original_price: 0.0,
        quantity: 1,
        unpaid_quantity: 1,
        selected_options: None,
        selected_specification: None,
        manual_discount_percent: None,
        rule_discount_amount: 0.0,
        rule_surcharge_amount: 0.0,
        applied_rules: vec![],
        applied_mg_rules: vec![],
        mg_discount_amount: 0.0,
        note: None,
        authorizer_id: None,
        authorizer_name: None,
        category_id: None,
        category_name: None,
        is_comped: false,
        unit_price: 0.0,
        line_total: 0.0,
        tax: 0.0,
        tax_rate: 0,
    };

    let result = calculate_unit_price(&item);
    // 负价格 clamp 到 0
    assert_eq!(
        result,
        Decimal::ZERO,
        "Negative price should be clamped to 0"
    );
}

#[test]
fn test_unit_price_discount_exceeding_100_percent() {
    let item = CartItemSnapshot {
        id: 1,
        instance_id: "i1".to_string(),
        name: "Item".to_string(),
        price: 100.0,
        original_price: 0.0,
        quantity: 1,
        unpaid_quantity: 1,
        selected_options: None,
        selected_specification: None,
        manual_discount_percent: Some(150.0), // 150% 折扣
        rule_discount_amount: 0.0,
        rule_surcharge_amount: 0.0,
        applied_rules: vec![],
        applied_mg_rules: vec![],
        mg_discount_amount: 0.0,
        note: None,
        authorizer_id: None,
        authorizer_name: None,
        category_id: None,
        category_name: None,
        is_comped: false,
        unit_price: 0.0,
        line_total: 0.0,
        tax: 0.0,
        tax_rate: 0,
    };

    let result = calculate_unit_price(&item);
    // 150% 折扣使价格变负，clamp 到 0
    assert_eq!(result, Decimal::ZERO, "150% discount should clamp to 0");
}

#[test]
fn test_unit_price_nan_price_becomes_zero() {
    let item = CartItemSnapshot {
        id: 1,
        instance_id: "i1".to_string(),
        name: "Item".to_string(),
        price: f64::NAN,
        original_price: 0.0,
        quantity: 1,
        unpaid_quantity: 1,
        selected_options: None,
        selected_specification: None,
        manual_discount_percent: None,
        rule_discount_amount: 0.0,
        rule_surcharge_amount: 0.0,
        applied_rules: vec![],
        applied_mg_rules: vec![],
        mg_discount_amount: 0.0,
        note: None,
        authorizer_id: None,
        authorizer_name: None,
        category_id: None,
        category_name: None,
        is_comped: false,
        unit_price: 0.0,
        line_total: 0.0,
        tax: 0.0,
        tax_rate: 0,
    };

    let result = calculate_unit_price(&item);
    assert_eq!(
        result,
        Decimal::ZERO,
        "NaN price should result in 0 unit price"
    );
}

#[test]
fn test_unit_price_infinity_price_becomes_zero() {
    let item = CartItemSnapshot {
        id: 1,
        instance_id: "i1".to_string(),
        name: "Item".to_string(),
        price: f64::INFINITY,
        original_price: 0.0,
        quantity: 1,
        unpaid_quantity: 1,
        selected_options: None,
        selected_specification: None,
        manual_discount_percent: None,
        rule_discount_amount: 0.0,
        rule_surcharge_amount: 0.0,
        applied_rules: vec![],
        applied_mg_rules: vec![],
        mg_discount_amount: 0.0,
        note: None,
        authorizer_id: None,
        authorizer_name: None,
        category_id: None,
        category_name: None,
        is_comped: false,
        unit_price: 0.0,
        line_total: 0.0,
        tax: 0.0,
        tax_rate: 0,
    };

    let result = calculate_unit_price(&item);
    assert_eq!(
        result,
        Decimal::ZERO,
        "Infinity price should result in 0 unit price"
    );
}

#[test]
fn test_unit_price_negative_discount_increases_price() {
    let item = CartItemSnapshot {
        id: 1,
        instance_id: "i1".to_string(),
        name: "Item".to_string(),
        price: 100.0,
        original_price: 0.0,
        quantity: 1,
        unpaid_quantity: 1,
        selected_options: None,
        selected_specification: None,
        manual_discount_percent: Some(-20.0), // 负折扣 = 加价
        rule_discount_amount: 0.0,
        rule_surcharge_amount: 0.0,
        applied_rules: vec![],
        applied_mg_rules: vec![],
        mg_discount_amount: 0.0,
        note: None,
        authorizer_id: None,
        authorizer_name: None,
        category_id: None,
        category_name: None,
        is_comped: false,
        unit_price: 0.0,
        line_total: 0.0,
        tax: 0.0,
        tax_rate: 0,
    };

    let result = calculate_unit_price(&item);
    // -20% discount = +20% => 100 + 20 = 120
    assert_eq!(
        to_f64(result),
        120.0,
        "Negative discount should increase price"
    );
}

// ========================================================================
// calculate_item_total 边界测试
// ========================================================================

#[test]
fn test_calculate_item_total_negative_quantity() {
    let item = CartItemSnapshot {
        id: 1,
        instance_id: "i1".to_string(),
        name: "Item".to_string(),
        price: 10.0,
        original_price: 0.0,
        quantity: -5,
        unpaid_quantity: -5,
        selected_options: None,
        selected_specification: None,
        manual_discount_percent: None,
        rule_discount_amount: 0.0,
        rule_surcharge_amount: 0.0,
        applied_rules: vec![],
        applied_mg_rules: vec![],
        mg_discount_amount: 0.0,
        note: None,
        authorizer_id: None,
        authorizer_name: None,
        category_id: None,
        category_name: None,
        is_comped: false,
        unit_price: 0.0,
        line_total: 0.0,
        tax: 0.0,
        tax_rate: 0,
    };

    let result = calculate_item_total(&item);
    // 10.0 * -5 = -50.0 — 负数行总计
    assert_eq!(
        to_f64(result),
        -50.0,
        "Negative quantity produces negative line total"
    );
}

#[test]
fn test_calculate_item_total_zero_quantity() {
    let item = CartItemSnapshot {
        id: 1,
        instance_id: "i1".to_string(),
        name: "Item".to_string(),
        price: 10.0,
        original_price: 0.0,
        quantity: 0,
        unpaid_quantity: 0,
        selected_options: None,
        selected_specification: None,
        manual_discount_percent: None,
        rule_discount_amount: 0.0,
        rule_surcharge_amount: 0.0,
        applied_rules: vec![],
        applied_mg_rules: vec![],
        mg_discount_amount: 0.0,
        note: None,
        authorizer_id: None,
        authorizer_name: None,
        category_id: None,
        category_name: None,
        is_comped: false,
        unit_price: 0.0,
        line_total: 0.0,
        tax: 0.0,
        tax_rate: 0,
    };

    let result = calculate_item_total(&item);
    assert_eq!(
        to_f64(result),
        0.0,
        "Zero quantity produces zero line total"
    );
}

#[test]
fn test_calculate_item_total_large_quantity_times_price() {
    // 大数量 × 大价格，但在 Decimal 范围内
    let item = CartItemSnapshot {
        id: 1,
        instance_id: "i1".to_string(),
        name: "Item".to_string(),
        price: 999999.99,
        original_price: 0.0,
        quantity: 10000,
        unpaid_quantity: 10000,
        selected_options: None,
        selected_specification: None,
        manual_discount_percent: None,
        rule_discount_amount: 0.0,
        rule_surcharge_amount: 0.0,
        applied_rules: vec![],
        applied_mg_rules: vec![],
        mg_discount_amount: 0.0,
        note: None,
        authorizer_id: None,
        authorizer_name: None,
        category_id: None,
        category_name: None,
        is_comped: false,
        unit_price: 0.0,
        line_total: 0.0,
        tax: 0.0,
        tax_rate: 0,
    };

    let result = calculate_item_total(&item);
    // 999999.99 * 10000 = 9_999_999_900.0
    assert_eq!(to_f64(result), 9_999_999_900.0);
}

// ========================================================================
// recalculate_totals 边界测试
// ========================================================================

#[test]
fn test_recalculate_totals_with_mixed_edge_items() {
    use shared::order::OrderSnapshot;

    let mut snapshot = OrderSnapshot::new("order-1".to_string());

    // 正常商品
    snapshot.items.push(CartItemSnapshot {
        id: 1,
        instance_id: "i1".to_string(),
        name: "Normal".to_string(),
        price: 10.0,
        original_price: 0.0,
        quantity: 2,
        unpaid_quantity: 2,
        selected_options: None,
        selected_specification: None,
        manual_discount_percent: None,
        rule_discount_amount: 0.0,
        rule_surcharge_amount: 0.0,
        applied_rules: vec![],
        applied_mg_rules: vec![],
        mg_discount_amount: 0.0,
        note: None,
        authorizer_id: None,
        authorizer_name: None,
        category_id: None,
        category_name: None,
        is_comped: false,
        unit_price: 0.0,
        line_total: 0.0,
        tax: 0.0,
        tax_rate: 0,
    });

    // 零价格商品
    snapshot.items.push(CartItemSnapshot {
        id: 2,
        instance_id: "i2".to_string(),
        name: "Free".to_string(),
        price: 0.0,
        original_price: 0.0,
        quantity: 1,
        unpaid_quantity: 1,
        selected_options: None,
        selected_specification: None,
        manual_discount_percent: None,
        rule_discount_amount: 0.0,
        rule_surcharge_amount: 0.0,
        applied_rules: vec![],
        applied_mg_rules: vec![],
        mg_discount_amount: 0.0,
        note: None,
        authorizer_id: None,
        authorizer_name: None,
        category_id: None,
        category_name: None,
        is_comped: false,
        unit_price: 0.0,
        line_total: 0.0,
        tax: 0.0,
        tax_rate: 0,
    });

    recalculate_totals(&mut snapshot);

    assert_eq!(snapshot.subtotal, 20.0);
    assert_eq!(snapshot.total, 20.0);
    assert_eq!(snapshot.remaining_amount, 20.0);
}

#[test]
fn test_recalculate_totals_order_discount_exceeds_subtotal() {
    use shared::order::OrderSnapshot;

    let mut snapshot = OrderSnapshot::new("order-1".to_string());
    snapshot.items.push(CartItemSnapshot {
        id: 1,
        instance_id: "i1".to_string(),
        name: "Item".to_string(),
        price: 50.0,
        original_price: 0.0,
        quantity: 1,
        unpaid_quantity: 1,
        selected_options: None,
        selected_specification: None,
        manual_discount_percent: None,
        rule_discount_amount: 0.0,
        rule_surcharge_amount: 0.0,
        applied_rules: vec![],
        applied_mg_rules: vec![],
        mg_discount_amount: 0.0,
        note: None,
        authorizer_id: None,
        authorizer_name: None,
        category_id: None,
        category_name: None,
        is_comped: false,
        unit_price: 0.0,
        line_total: 0.0,
        tax: 0.0,
        tax_rate: 0,
    });
    // 订单级固定折扣大于小计
    snapshot.order_manual_discount_fixed = Some(100.0);

    recalculate_totals(&mut snapshot);

    // total = max(subtotal(50) - order_discount(100), 0) = 0
    assert_eq!(snapshot.subtotal, 50.0);
    assert_eq!(
        snapshot.total, 0.0,
        "total 被 clamp 到 0 (折扣不产生负总额)"
    );
    assert_eq!(snapshot.remaining_amount, 0.0, "remaining_amount 也为 0");
}

// ========================================================================
// is_payment_sufficient 边界测试
// ========================================================================

#[test]
fn test_is_payment_sufficient_nan_values() {
    // NaN 转为 0, 所以 is_payment_sufficient(NaN, 100) → 0 >= 99.99 → false
    assert!(!is_payment_sufficient(f64::NAN, 100.0));
    // is_payment_sufficient(100, NaN) → 100 >= -0.01 → true
    assert!(is_payment_sufficient(100.0, f64::NAN));
    // is_payment_sufficient(NaN, NaN) → 0 >= -0.01 → true
    assert!(is_payment_sufficient(f64::NAN, f64::NAN));
}

#[test]
fn test_is_payment_sufficient_infinity_values() {
    // Infinity → 0
    assert!(!is_payment_sufficient(f64::INFINITY, 100.0));
    assert!(is_payment_sufficient(100.0, f64::INFINITY));
}

// ========================================================================
// original_price = base price for calculations; avoid double-counting
// ========================================================================

#[test]
fn test_unit_price_with_original_price_and_options_no_double_counting() {
    // Scenario: reducer sets original_price=Some(spec_price), price=item_final
    // money.rs should use original_price as base, add options, not double-count
    let item = CartItemSnapshot {
        id: 1,
        instance_id: "i1".to_string(),
        name: "Pizza".to_string(),
        price: 16.50,         // item_final from reducer (already includes options)
        original_price: 12.0, // spec price set by reducer
        quantity: 1,
        unpaid_quantity: 1,
        selected_options: Some(vec![
            shared::order::ItemOption {
                attribute_id: 1,
                attribute_name: "Size".to_string(),
                option_id: 2,
                option_name: "Large".to_string(),
                price_modifier: Some(3.0),
                quantity: 1,
            },
            shared::order::ItemOption {
                attribute_id: 2,
                attribute_name: "Topping".to_string(),
                option_id: 0,
                option_name: "Extra Cheese".to_string(),
                price_modifier: Some(1.50),
                quantity: 1,
            },
        ]),
        selected_specification: None,
        manual_discount_percent: None,
        rule_discount_amount: 0.0,
        rule_surcharge_amount: 0.0,
        applied_rules: vec![],
        applied_mg_rules: vec![],
        mg_discount_amount: 0.0,
        unit_price: 0.0,
        line_total: 0.0,
        tax: 0.0,
        tax_rate: 0,
        note: None,
        authorizer_id: None,
        authorizer_name: None,
        category_id: None,
        category_name: None,
        is_comped: false,
    };

    let unit_price = calculate_unit_price(&item);
    // base_price = original_price = 12.0
    // options = 3.0 + 1.50 = 4.50
    // base_with_options = 16.50
    // No discounts, no surcharges
    // unit_price = 16.50
    assert_eq!(to_f64(unit_price), 16.50);
}

#[test]
fn test_rule_discount_plus_options_plus_manual_discount() {
    // Full combination: rule_discount + options + manual_discount
    let item = CartItemSnapshot {
        id: 1,
        instance_id: "i1".to_string(),
        name: "Item".to_string(),
        price: 85.0,           // item_final from reducer
        original_price: 100.0, // spec price
        quantity: 2,
        unpaid_quantity: 2,
        selected_options: Some(vec![shared::order::ItemOption {
            attribute_id: 1,
            attribute_name: "Extra".to_string(),
            option_id: 0,
            option_name: "Cheese".to_string(),
            price_modifier: Some(5.0),
            quantity: 1,
        }]),
        selected_specification: None,
        manual_discount_percent: Some(10.0), // 10% off
        rule_discount_amount: 3.0,           // -3 per unit
        rule_surcharge_amount: 0.0,
        applied_rules: vec![],
        applied_mg_rules: vec![],
        mg_discount_amount: 0.0,
        unit_price: 0.0,
        line_total: 0.0,
        tax: 0.0,
        tax_rate: 0,
        note: None,
        authorizer_id: None,
        authorizer_name: None,
        category_id: None,
        category_name: None,
        is_comped: false,
    };

    let unit_price = calculate_unit_price(&item);
    // base_price = original_price = 100.0
    // options = 5.0
    // base_with_options = 105.0
    // manual_discount = 105.0 * 10% = 10.5
    // rule_discount = 3.0
    // unit_price = 105.0 - 10.5 - 3.0 = 91.5
    assert_eq!(to_f64(unit_price), 91.5);

    let total = calculate_item_total(&item);
    // 91.5 * 2 = 183.0
    assert_eq!(to_f64(total), 183.0);
}

#[test]
fn test_option_quantity_multiplies_price_modifier() {
    // Test that option price_modifier is multiplied by quantity
    // Scenario: +鸡蛋 ×3 with price_modifier=2.0 should add 6.0 to the price
    let item = CartItemSnapshot {
        id: 1,
        instance_id: "i1".to_string(),
        name: "Noodles".to_string(),
        price: 16.0,          // item_final from reducer
        original_price: 10.0, // base price
        quantity: 1,
        unpaid_quantity: 1,
        selected_options: Some(vec![shared::order::ItemOption {
            attribute_id: 1,
            attribute_name: "加蛋".to_string(),
            option_id: 0,
            option_name: "鸡蛋".to_string(),
            price_modifier: Some(2.0), // +2 per egg
            quantity: 3,               // 3 eggs!
        }]),
        selected_specification: None,
        manual_discount_percent: None,
        rule_discount_amount: 0.0,
        rule_surcharge_amount: 0.0,
        applied_rules: vec![],
        applied_mg_rules: vec![],
        mg_discount_amount: 0.0,
        unit_price: 0.0,
        line_total: 0.0,
        tax: 0.0,
        tax_rate: 0,
        note: None,
        authorizer_id: None,
        authorizer_name: None,
        category_id: None,
        category_name: None,
        is_comped: false,
    };

    let unit_price = calculate_unit_price(&item);
    // base_price = original_price = 10.0
    // options = 2.0 * 3 = 6.0
    // base_with_options = 16.0
    // No discounts
    // unit_price = 16.0
    assert_eq!(to_f64(unit_price), 16.0);

    let total = calculate_item_total(&item);
    assert_eq!(to_f64(total), 16.0);
}

#[test]
fn test_multiple_options_with_different_quantities() {
    // Test multiple options with different quantities
    let item = CartItemSnapshot {
        id: 1,
        instance_id: "i1".to_string(),
        name: "Burger".to_string(),
        price: 17.0,
        original_price: 10.0, // base price
        quantity: 2,          // 2 burgers
        unpaid_quantity: 2,
        selected_options: Some(vec![
            shared::order::ItemOption {
                attribute_id: 1,
                attribute_name: "Cheese".to_string(),
                option_id: 0,
                option_name: "Cheddar".to_string(),
                price_modifier: Some(1.5), // +1.5 per slice
                quantity: 2,               // 2 slices
            },
            shared::order::ItemOption {
                attribute_id: 2,
                attribute_name: "Bacon".to_string(),
                option_id: 0,
                option_name: "Crispy".to_string(),
                price_modifier: Some(2.0), // +2 per strip
                quantity: 2,               // 2 strips
            },
        ]),
        selected_specification: None,
        manual_discount_percent: None,
        rule_discount_amount: 0.0,
        rule_surcharge_amount: 0.0,
        applied_rules: vec![],
        applied_mg_rules: vec![],
        mg_discount_amount: 0.0,
        unit_price: 0.0,
        line_total: 0.0,
        tax: 0.0,
        tax_rate: 0,
        note: None,
        authorizer_id: None,
        authorizer_name: None,
        category_id: None,
        category_name: None,
        is_comped: false,
    };

    let unit_price = calculate_unit_price(&item);
    // base_price = 10.0
    // cheese = 1.5 * 2 = 3.0
    // bacon = 2.0 * 2 = 4.0
    // options total = 7.0
    // base_with_options = 17.0
    // unit_price = 17.0
    assert_eq!(to_f64(unit_price), 17.0);

    let total = calculate_item_total(&item);
    // 17.0 * 2 burgers = 34.0
    assert_eq!(to_f64(total), 34.0);
}

#[test]
fn test_rule_discount_exceeding_price_clamps_to_zero() {
    let item = CartItemSnapshot {
        id: 1,
        instance_id: "i1".to_string(),
        name: "Item".to_string(),
        price: 5.0,
        original_price: 10.0,
        quantity: 1,
        unpaid_quantity: 1,
        selected_options: None,
        selected_specification: None,
        manual_discount_percent: None,
        rule_discount_amount: 15.0, // Discount exceeds base price
        rule_surcharge_amount: 0.0,
        applied_rules: vec![],
        applied_mg_rules: vec![],
        mg_discount_amount: 0.0,
        unit_price: 0.0,
        line_total: 0.0,
        tax: 0.0,
        tax_rate: 0,
        note: None,
        authorizer_id: None,
        authorizer_name: None,
        category_id: None,
        category_name: None,
        is_comped: false,
    };

    let unit_price = calculate_unit_price(&item);
    // base_with_options = 10.0
    // rule_discount = 15.0 > 10.0
    // unit_price = max(0, 10.0 - 15.0) = 0
    assert_eq!(unit_price, Decimal::ZERO);
}

// ========================================================================
// validate_item_changes 隔离测试
// ========================================================================

#[test]
fn test_validate_item_changes_valid() {
    let changes = ItemChanges {
        price: Some(25.0),
        quantity: Some(3),
        manual_discount_percent: Some(15.0),
        note: Some("test".to_string()),
        selected_options: None,
        selected_specification: None,
    };
    assert!(validate_item_changes(&changes).is_ok());
}

#[test]
fn test_validate_item_changes_all_none() {
    let changes = ItemChanges {
        price: None,
        quantity: None,
        manual_discount_percent: None,
        note: None,
        selected_options: None,
        selected_specification: None,
    };
    // All-None is technically valid (no-op)
    assert!(validate_item_changes(&changes).is_ok());
}

#[test]
fn test_validate_item_changes_nan_price() {
    let changes = ItemChanges {
        price: Some(f64::NAN),
        quantity: None,
        manual_discount_percent: None,
        note: None,
        selected_options: None,
        selected_specification: None,
    };
    assert!(
        validate_item_changes(&changes).is_err(),
        "NaN price should be rejected"
    );
}

#[test]
fn test_validate_item_changes_infinity_price() {
    let changes = ItemChanges {
        price: Some(f64::INFINITY),
        quantity: None,
        manual_discount_percent: None,
        note: None,
        selected_options: None,
        selected_specification: None,
    };
    assert!(
        validate_item_changes(&changes).is_err(),
        "Infinity price should be rejected"
    );
}

#[test]
fn test_validate_item_changes_negative_price() {
    let changes = ItemChanges {
        price: Some(-5.0),
        quantity: None,
        manual_discount_percent: None,
        note: None,
        selected_options: None,
        selected_specification: None,
    };
    assert!(
        validate_item_changes(&changes).is_err(),
        "Negative price should be rejected"
    );
}

#[test]
fn test_validate_item_changes_exceeds_max_price() {
    let changes = ItemChanges {
        price: Some(MAX_PRICE + 1.0),
        quantity: None,
        manual_discount_percent: None,
        note: None,
        selected_options: None,
        selected_specification: None,
    };
    assert!(
        validate_item_changes(&changes).is_err(),
        "Price > MAX_PRICE should be rejected"
    );
}

#[test]
fn test_validate_item_changes_zero_quantity() {
    let changes = ItemChanges {
        price: None,
        quantity: Some(0),
        manual_discount_percent: None,
        note: None,
        selected_options: None,
        selected_specification: None,
    };
    assert!(
        validate_item_changes(&changes).is_err(),
        "Zero quantity should be rejected"
    );
}

#[test]
fn test_validate_item_changes_negative_quantity() {
    let changes = ItemChanges {
        price: None,
        quantity: Some(-1),
        manual_discount_percent: None,
        note: None,
        selected_options: None,
        selected_specification: None,
    };
    assert!(
        validate_item_changes(&changes).is_err(),
        "Negative quantity should be rejected"
    );
}

#[test]
fn test_validate_item_changes_exceeds_max_quantity() {
    let changes = ItemChanges {
        price: None,
        quantity: Some(MAX_QUANTITY + 1),
        manual_discount_percent: None,
        note: None,
        selected_options: None,
        selected_specification: None,
    };
    assert!(
        validate_item_changes(&changes).is_err(),
        "Quantity > MAX should be rejected"
    );
}

#[test]
fn test_validate_item_changes_discount_nan() {
    let changes = ItemChanges {
        price: None,
        quantity: None,
        manual_discount_percent: Some(f64::NAN),
        note: None,
        selected_options: None,
        selected_specification: None,
    };
    assert!(
        validate_item_changes(&changes).is_err(),
        "NaN discount should be rejected"
    );
}

#[test]
fn test_validate_item_changes_discount_negative() {
    let changes = ItemChanges {
        price: None,
        quantity: None,
        manual_discount_percent: Some(-10.0),
        note: None,
        selected_options: None,
        selected_specification: None,
    };
    assert!(
        validate_item_changes(&changes).is_err(),
        "Negative discount should be rejected"
    );
}

#[test]
fn test_validate_item_changes_discount_over_100() {
    let changes = ItemChanges {
        price: None,
        quantity: None,
        manual_discount_percent: Some(101.0),
        note: None,
        selected_options: None,
        selected_specification: None,
    };
    assert!(
        validate_item_changes(&changes).is_err(),
        "Discount > 100% should be rejected"
    );
}

#[test]
fn test_validate_item_changes_option_nan_modifier() {
    let changes = ItemChanges {
        price: None,
        quantity: None,
        manual_discount_percent: None,
        note: None,
        selected_options: Some(vec![shared::order::ItemOption {
            attribute_id: 1,
            attribute_name: "Size".to_string(),
            option_id: 0,
            option_name: "Large".to_string(),
            price_modifier: Some(f64::NAN),
            quantity: 1,
        }]),
        selected_specification: None,
    };
    assert!(
        validate_item_changes(&changes).is_err(),
        "NaN option modifier should be rejected"
    );
}

#[test]
fn test_validate_item_changes_option_exceeds_max_modifier() {
    let changes = ItemChanges {
        price: None,
        quantity: None,
        manual_discount_percent: None,
        note: None,
        selected_options: Some(vec![shared::order::ItemOption {
            attribute_id: 1,
            attribute_name: "Size".to_string(),
            option_id: 0,
            option_name: "Large".to_string(),
            price_modifier: Some(MAX_PRICE + 1.0),
            quantity: 1,
        }]),
        selected_specification: None,
    };
    assert!(
        validate_item_changes(&changes).is_err(),
        "Option modifier > MAX_PRICE should be rejected"
    );
}

// ========================================================================
// sum_payments 隔离测试
// ========================================================================

#[test]
fn test_sum_payments_empty() {
    assert_eq!(sum_payments(&[]), 0.0);
}

#[test]
fn test_sum_payments_single() {
    let payments = vec![shared::order::PaymentRecord {
        payment_id: "p1".to_string(),
        method: "CASH".to_string(),
        amount: 25.50,
        tendered: None,
        change: None,
        note: None,
        cancelled: false,
        cancel_reason: None,
        split_items: None,
        aa_shares: None,
        split_type: None,
        timestamp: 1000,
    }];
    assert_eq!(sum_payments(&payments), 25.50);
}

#[test]
fn test_sum_payments_with_cancelled() {
    let payments = vec![
        shared::order::PaymentRecord {
            payment_id: "p1".to_string(),
            method: "CASH".to_string(),
            amount: 30.0,
            tendered: None,
            change: None,
            note: None,
            cancelled: true, // cancelled — should be excluded
            cancel_reason: Some("wrong".to_string()),
            split_items: None,
            aa_shares: None,
            split_type: None,
            timestamp: 1000,
        },
        shared::order::PaymentRecord {
            payment_id: "p2".to_string(),
            method: "CARD".to_string(),
            amount: 15.0,
            tendered: None,
            change: None,
            note: None,
            cancelled: false,
            cancel_reason: None,
            split_items: None,
            aa_shares: None,
            split_type: None,
            timestamp: 2000,
        },
    ];
    assert_eq!(
        sum_payments(&payments),
        15.0,
        "Cancelled payment should be excluded"
    );
}

#[test]
fn test_sum_payments_all_cancelled() {
    let payments = vec![shared::order::PaymentRecord {
        payment_id: "p1".to_string(),
        method: "CASH".to_string(),
        amount: 50.0,
        tendered: None,
        change: None,
        note: None,
        cancelled: true,
        cancel_reason: None,
        split_items: None,
        aa_shares: None,
        split_type: None,
        timestamp: 1000,
    }];
    assert_eq!(sum_payments(&payments), 0.0, "All cancelled = 0");
}

#[test]
fn test_sum_payments_precision() {
    // 10 payments of 0.1 each should sum to exactly 1.0
    let payments: Vec<shared::order::PaymentRecord> = (0..10)
        .map(|i| shared::order::PaymentRecord {
            payment_id: format!("p{}", i),
            method: "CASH".to_string(),
            amount: 0.1,
            tendered: None,
            change: None,
            note: None,
            cancelled: false,
            cancel_reason: None,
            split_items: None,
            aa_shares: None,
            split_type: None,
            timestamp: 1000 + i,
        })
        .collect();
    assert_eq!(
        sum_payments(&payments),
        1.0,
        "0.1 * 10 = 1.0 with Decimal precision"
    );
}

// ========================================================================
// effective_rule_* helpers: skipped flag handling
// ========================================================================

use shared::models::price_rule::{AdjustmentType, ProductScope};
use shared::order::AppliedRule;

fn make_applied_rule(
    rule_id: i64,
    rule_type: RuleType,
    adjustment_value: f64,
    skipped: bool,
) -> AppliedRule {
    AppliedRule {
        rule_id,
        name: format!("rule-{rule_id}"),
        display_name: format!("rule-{rule_id}"),
        receipt_name: "R".to_string(),
        rule_type,
        adjustment_type: AdjustmentType::Percentage,
        product_scope: ProductScope::Global,
        zone_scope: "zone:all".to_string(),
        adjustment_value,
        calculated_amount: 0.0, // no longer authoritative; dynamically recalculated
        is_stackable: true,
        is_exclusive: false,
        skipped,
    }
}

fn make_item_with_rules(
    original_price: f64,
    rules: Vec<AppliedRule>,
    legacy_discount: Option<f64>,
    legacy_surcharge: Option<f64>,
) -> CartItemSnapshot {
    CartItemSnapshot {
        id: 1,
        instance_id: "i1".to_string(),
        name: "Item".to_string(),
        price: original_price,
        original_price,
        quantity: 1,
        unpaid_quantity: 1,
        selected_options: None,
        selected_specification: None,
        manual_discount_percent: None,
        rule_discount_amount: legacy_discount.unwrap_or(0.0),
        rule_surcharge_amount: legacy_surcharge.unwrap_or(0.0),
        applied_rules: rules,
        applied_mg_rules: vec![],
        mg_discount_amount: 0.0,
        unit_price: 0.0,
        line_total: 0.0,
        tax: 0.0,
        tax_rate: 0,
        note: None,
        authorizer_id: None,
        authorizer_name: None,
        category_id: None,
        category_name: None,
        is_comped: false,
    }
}

#[test]
fn test_effective_rule_discount_skipped_excluded() {
    // One active discount + one skipped discount → only active counts
    let item = make_item_with_rules(
        100.0,
        vec![
            make_applied_rule(1, RuleType::Discount, 5.0, false),
            make_applied_rule(2, RuleType::Discount, 3.0, true), // skipped
        ],
        Some(8.0), // legacy total (should be ignored when applied_rules present)
        None,
    );
    // basis = 100 (no manual discount), adjustment_value=5 → 100*5/100=5.0
    let eff = effective_rule_discount(&item, to_decimal(100.0));
    assert_eq!(to_f64(eff), 5.0, "Only non-skipped discount should count");
}

#[test]
fn test_effective_rule_surcharge_skipped_excluded() {
    let item = make_item_with_rules(
        100.0,
        vec![
            make_applied_rule(11, RuleType::Surcharge, 7.0, true), // skipped
            make_applied_rule(12, RuleType::Surcharge, 4.0, false),
        ],
        None,
        Some(11.0), // legacy total (should be ignored)
    );
    // basis = 100 (base_with_options), adjustment_value=4 → 100*4/100=4.0
    let eff = effective_rule_surcharge(&item, to_decimal(100.0));
    assert_eq!(to_f64(eff), 4.0, "Only non-skipped surcharge should count");
}

#[test]
fn test_effective_rule_discount_all_skipped() {
    let item = make_item_with_rules(
        100.0,
        vec![
            make_applied_rule(1, RuleType::Discount, 5.0, true),
            make_applied_rule(2, RuleType::Discount, 3.0, true),
        ],
        Some(8.0),
        None,
    );
    let eff = effective_rule_discount(&item, to_decimal(100.0));
    assert_eq!(eff, Decimal::ZERO, "All skipped → zero effective discount");
}

#[test]
fn test_effective_rule_surcharge_all_skipped() {
    let item = make_item_with_rules(
        100.0,
        vec![make_applied_rule(11, RuleType::Surcharge, 5.0, true)],
        None,
        Some(5.0),
    );
    let eff = effective_rule_surcharge(&item, to_decimal(100.0));
    assert_eq!(eff, Decimal::ZERO, "All skipped → zero effective surcharge");
}

#[test]
fn test_effective_rule_discount_empty_applied_rules() {
    // applied_rules is empty vec → falls back to legacy rule_discount_amount
    // (with Vec<AppliedRule> there is no distinction between "absent" and "empty")
    let item = make_item_with_rules(100.0, vec![], Some(10.0), None);
    let eff = effective_rule_discount(&item, to_decimal(100.0));
    assert_eq!(to_f64(eff), 10.0, "Empty applied_rules → legacy fallback");
}

#[test]
fn test_effective_rule_discount_legacy_fallback() {
    // applied_rules is None → fall back to legacy rule_discount_amount
    let mut item = make_item_with_rules(100.0, vec![], None, None);
    item.applied_rules = vec![];
    item.rule_discount_amount = 7.5;
    let eff = effective_rule_discount(&item, to_decimal(100.0));
    assert_eq!(to_f64(eff), 7.5, "None applied_rules → use legacy field");
}

#[test]
fn test_effective_rule_surcharge_legacy_fallback() {
    let mut item = make_item_with_rules(100.0, vec![], None, None);
    item.applied_rules = vec![];
    item.rule_surcharge_amount = 3.5;
    let eff = effective_rule_surcharge(&item, to_decimal(100.0));
    assert_eq!(to_f64(eff), 3.5, "None applied_rules → use legacy field");
}

#[test]
fn test_effective_rule_discount_ignores_surcharge_rules() {
    // applied_rules has both discount and surcharge → only discount counted
    let item = make_item_with_rules(
        100.0,
        vec![
            make_applied_rule(1, RuleType::Discount, 5.0, false),
            make_applied_rule(11, RuleType::Surcharge, 10.0, false),
        ],
        None,
        None,
    );
    let eff = effective_rule_discount(&item, to_decimal(100.0));
    assert_eq!(to_f64(eff), 5.0, "Only Discount type rules counted");
}

#[test]
fn test_effective_rule_surcharge_ignores_discount_rules() {
    let item = make_item_with_rules(
        100.0,
        vec![
            make_applied_rule(1, RuleType::Discount, 5.0, false),
            make_applied_rule(11, RuleType::Surcharge, 10.0, false),
        ],
        None,
        None,
    );
    let eff = effective_rule_surcharge(&item, to_decimal(100.0));
    assert_eq!(to_f64(eff), 10.0, "Only Surcharge type rules counted");
}

#[test]
fn test_unit_price_with_skipped_discount_rule() {
    // Item: base 100, active discount 5, skipped discount 3 → unit_price = 95
    let item = make_item_with_rules(
        100.0,
        vec![
            make_applied_rule(1, RuleType::Discount, 5.0, false),
            make_applied_rule(2, RuleType::Discount, 3.0, true), // skipped
        ],
        Some(8.0),
        None,
    );
    let up = calculate_unit_price(&item);
    assert_eq!(to_f64(up), 95.0, "Skipped discount should not reduce price");
}

#[test]
fn test_unit_price_with_skipped_surcharge_rule() {
    // Item: base 100, skipped surcharge 10 → unit_price = 100 (not 110)
    let item = make_item_with_rules(
        100.0,
        vec![make_applied_rule(11, RuleType::Surcharge, 10.0, true)],
        None,
        Some(10.0),
    );
    let up = calculate_unit_price(&item);
    assert_eq!(
        to_f64(up),
        100.0,
        "Skipped surcharge should not increase price"
    );
}

#[test]
fn test_unit_price_comped_item_ignores_applied_rules() {
    // Comped item always returns 0 regardless of rules
    let mut item = make_item_with_rules(
        100.0,
        vec![make_applied_rule(1, RuleType::Discount, 5.0, false)],
        Some(5.0),
        None,
    );
    item.is_comped = true;
    let up = calculate_unit_price(&item);
    assert_eq!(up, Decimal::ZERO, "Comped item always zero");
}

#[test]
fn test_unit_price_manual_plus_rule_discount_combined() {
    // manual 60% + rule discount 50% → rule now based on after_manual
    let mut item = make_item_with_rules(
        100.0,
        vec![make_applied_rule(1, RuleType::Discount, 50.0, false)],
        Some(50.0),
        None,
    );
    item.manual_discount_percent = Some(60.0);
    let up = calculate_unit_price(&item);
    // base = 100, manual = 60, after_manual = 40
    // rule_discount = 40 * 50% = 20
    // unit_price = 100 - 60 - 20 = 20
    assert_eq!(
        up,
        Decimal::from(20),
        "Rule discount based on after_manual price"
    );
}

#[test]
fn test_recalculate_totals_with_skipped_item_rule() {
    // Verify recalculate_totals produces correct results when item has skipped rules
    let mut snapshot = OrderSnapshot::new("order-1".to_string());
    snapshot.items.push(make_item_with_rules(
        100.0,
        vec![
            make_applied_rule(1, RuleType::Discount, 10.0, true), // skipped
            make_applied_rule(2, RuleType::Discount, 5.0, false), // active
        ],
        Some(15.0),
        None,
    ));
    snapshot.items[0].quantity = 2;
    snapshot.items[0].unpaid_quantity = 2;

    recalculate_totals(&mut snapshot);

    // unit_price = 100 - 5 (only active discount) = 95
    assert_eq!(snapshot.items[0].unit_price, 95.0);
    // line_total = 95 * 2 = 190
    assert_eq!(snapshot.items[0].line_total, 190.0);
    // subtotal = 190
    assert_eq!(snapshot.subtotal, 190.0);
    assert_eq!(snapshot.total, 190.0);
    // original_total = 100 * 2 = 200
    assert_eq!(snapshot.original_total, 200.0);
    // total_discount should count only active rule: 5 * 2 = 10
    assert_eq!(snapshot.total_discount, 10.0);
}

#[test]
fn test_recalculate_totals_skipped_order_level_discount() {
    let mut snapshot = OrderSnapshot::new("order-1".to_string());
    // Simple item, no item-level rules
    let mut item = make_item_with_rules(100.0, vec![], None, None);
    item.applied_rules = vec![];
    snapshot.items.push(item);

    // Order-level discount, skipped
    snapshot.order_applied_rules = vec![
        make_applied_rule(101, RuleType::Discount, 20.0, true), // skipped
    ];
    snapshot.order_rule_discount_amount = 20.0; // legacy

    recalculate_totals(&mut snapshot);

    // Order discount skipped → total equals subtotal
    assert_eq!(snapshot.subtotal, 100.0);
    assert_eq!(snapshot.total, 100.0);
    assert_eq!(snapshot.discount, 0.0, "Skipped order discount → 0");
}

#[test]
fn test_recalculate_totals_skipped_order_level_surcharge() {
    let mut snapshot = OrderSnapshot::new("order-1".to_string());
    let mut item = make_item_with_rules(100.0, vec![], None, None);
    item.applied_rules = vec![];
    snapshot.items.push(item);

    snapshot.order_applied_rules = vec![
        make_applied_rule(111, RuleType::Surcharge, 15.0, true), // skipped
    ];
    snapshot.order_rule_surcharge_amount = 15.0;

    recalculate_totals(&mut snapshot);

    assert_eq!(snapshot.subtotal, 100.0);
    assert_eq!(
        snapshot.total, 100.0,
        "Skipped order surcharge → no increase"
    );
}

#[test]
fn test_recalculate_totals_mixed_active_and_skipped_order_rules() {
    let mut snapshot = OrderSnapshot::new("order-1".to_string());
    let mut item = make_item_with_rules(200.0, vec![], None, None);
    item.applied_rules = vec![];
    snapshot.items.push(item);

    snapshot.order_applied_rules = vec![
        make_applied_rule(101, RuleType::Discount, 10.0, false), // active
        make_applied_rule(102, RuleType::Discount, 20.0, true),  // skipped
        make_applied_rule(111, RuleType::Surcharge, 5.0, false), // active
        make_applied_rule(112, RuleType::Surcharge, 8.0, true),  // skipped
    ];

    recalculate_totals(&mut snapshot);

    // subtotal = 200
    assert_eq!(snapshot.subtotal, 200.0);
    // order_discount = 200*10/100=20 (only or1), order_surcharge = 200*5/100=10 (only os1)
    // total = 200 - 20 + 10 = 190
    assert_eq!(snapshot.total, 190.0);
    assert_eq!(snapshot.discount, 20.0);
}

#[test]
fn test_recalculate_totals_pre_payment_reset_on_rule_skip() {
    // When a rule skip causes total to change, is_pre_payment must reset
    let mut snapshot = OrderSnapshot::new("order-1".to_string());
    snapshot.items.push(make_item_with_rules(
        100.0,
        vec![make_applied_rule(1, RuleType::Discount, 10.0, false)],
        Some(10.0),
        None,
    ));
    // First: calculate initial totals
    recalculate_totals(&mut snapshot);
    assert_eq!(snapshot.total, 90.0);

    // Set pre-payment flag
    snapshot.is_pre_payment = true;

    // Now "skip" the rule (simulating applier toggle)
    snapshot.items[0].applied_rules[0].skipped = true;
    recalculate_totals(&mut snapshot);

    // Total changed from 90 to 100 → pre_payment should reset
    assert_eq!(snapshot.total, 100.0);
    assert!(
        !snapshot.is_pre_payment,
        "Pre-payment must reset when total changes from rule skip"
    );
}

#[test]
fn test_recalculate_totals_tax_on_item_with_skipped_surcharge() {
    // Tax should be calculated on the effective total (after skipping surcharge)
    let mut snapshot = OrderSnapshot::new("order-1".to_string());
    let mut item = make_item_with_rules(
        100.0,
        vec![make_applied_rule(11, RuleType::Surcharge, 10.0, true)], // skipped
        None,
        Some(10.0),
    );
    item.tax_rate = 21; // 21% IVA
    snapshot.items.push(item);

    recalculate_totals(&mut snapshot);

    // Surcharge skipped → unit_price = 100, line_total = 100
    assert_eq!(snapshot.items[0].unit_price, 100.0);
    // Tax: 100 * 21 / (100 + 21) = 100 * 21/121 ≈ 17.36
    assert_eq!(snapshot.items[0].tax, 17.36);
    assert_eq!(snapshot.tax, 17.36);
}

#[test]
fn test_recalculate_totals_tax_on_item_with_active_surcharge() {
    // Compare: with active surcharge, tax is on higher amount
    let mut snapshot = OrderSnapshot::new("order-1".to_string());
    let mut item = make_item_with_rules(
        100.0,
        vec![make_applied_rule(11, RuleType::Surcharge, 10.0, false)], // active
        None,
        Some(10.0),
    );
    item.tax_rate = 21;
    snapshot.items.push(item);

    recalculate_totals(&mut snapshot);

    // Surcharge active → unit_price = 110, line_total = 110
    assert_eq!(snapshot.items[0].unit_price, 110.0);
    // Tax: 110 * 21 / 121 ≈ 19.09
    assert_eq!(snapshot.items[0].tax, 19.09);
}

#[test]
fn test_recalculate_totals_skip_item_rule_changes_order_discount_basis() {
    // When item-level rule is skipped, subtotal changes, which affects
    // order-level percentage discount calculation
    let mut snapshot = OrderSnapshot::new("order-1".to_string());
    snapshot.items.push(make_item_with_rules(
        100.0,
        vec![make_applied_rule(1, RuleType::Discount, 20.0, false)], // active: -20
        Some(20.0),
        None,
    ));
    // Order-level manual percentage discount
    snapshot.order_manual_discount_percent = Some(10.0); // 10% of subtotal

    recalculate_totals(&mut snapshot);

    // subtotal = 80 (100-20), order_discount = 80 * 10% = 8, total = 72
    assert_eq!(snapshot.subtotal, 80.0);
    assert_eq!(snapshot.total, 72.0);

    // Now skip the item rule
    snapshot.items[0].applied_rules[0].skipped = true;
    recalculate_totals(&mut snapshot);

    // subtotal = 100 (no item discount), order_discount = 100 * 10% = 10, total = 90
    assert_eq!(snapshot.subtotal, 100.0);
    assert_eq!(snapshot.total, 90.0);
}

#[test]
fn test_unit_price_with_options_and_skipped_rule() {
    // Options modifier + skipped rule → options still apply, rule doesn't
    let mut item = make_item_with_rules(
        50.0,
        vec![make_applied_rule(1, RuleType::Discount, 5.0, true)], // skipped
        Some(5.0),
        None,
    );
    item.selected_options = Some(vec![shared::order::ItemOption {
        attribute_id: 1,
        attribute_name: "Size".to_string(),
        option_id: 1,
        option_name: "Large".to_string(),
        price_modifier: Some(3.0),
        quantity: 1,
    }]);

    let up = calculate_unit_price(&item);
    // base = 50, options = 3 → base_with_options = 53
    // rule discount skipped → 0
    // unit_price = 53
    assert_eq!(to_f64(up), 53.0);
}

#[test]
fn test_effective_rule_discount_multiple_active_stacked() {
    // 3 active discount rules → sum all calculated_amounts
    let item = make_item_with_rules(
        100.0,
        vec![
            make_applied_rule(1, RuleType::Discount, 3.0, false),
            make_applied_rule(2, RuleType::Discount, 4.0, false),
            make_applied_rule(3, RuleType::Discount, 2.5, false),
        ],
        Some(9.5),
        None,
    );
    // basis=100, values=3+4+2.5 → 100*3/100 + 100*4/100 + 100*2.5/100 = 9.5
    let eff = effective_rule_discount(&item, to_decimal(100.0));
    assert_eq!(to_f64(eff), 9.5, "Sum of all active discounts");
}

#[test]
fn test_skip_unskip_cycle_preserves_amounts() {
    // Skip a rule, then unskip it → amounts should return to original
    let mut snapshot = OrderSnapshot::new("order-1".to_string());
    snapshot.items.push(make_item_with_rules(
        100.0,
        vec![make_applied_rule(1, RuleType::Discount, 10.0, false)],
        Some(10.0),
        None,
    ));

    recalculate_totals(&mut snapshot);
    let original_total = snapshot.total;
    let original_subtotal = snapshot.subtotal;
    assert_eq!(original_total, 90.0);

    // Skip
    snapshot.items[0].applied_rules[0].skipped = true;
    recalculate_totals(&mut snapshot);
    assert_eq!(snapshot.total, 100.0);

    // Unskip
    snapshot.items[0].applied_rules[0].skipped = false;
    recalculate_totals(&mut snapshot);

    assert_eq!(
        snapshot.total, original_total,
        "Total restored after unskip"
    );
    assert_eq!(
        snapshot.subtotal, original_subtotal,
        "Subtotal restored after unskip"
    );
}

#[test]
fn test_effective_order_rule_discount_skipped() {
    let mut snapshot = OrderSnapshot::new("order-1".to_string());
    snapshot.order_applied_rules = vec![
        make_applied_rule(101, RuleType::Discount, 10.0, false),
        make_applied_rule(102, RuleType::Discount, 5.0, true), // skipped
    ];
    snapshot.order_rule_discount_amount = 15.0;

    // subtotal=100 as basis, adjustment_value=10 → 100*10/100=10.0
    let eff = effective_order_rule_discount(&snapshot, to_decimal(100.0));
    assert_eq!(to_f64(eff), 10.0, "Only active order discount counted");
}

#[test]
fn test_effective_order_rule_surcharge_skipped() {
    let mut snapshot = OrderSnapshot::new("order-1".to_string());
    snapshot.order_applied_rules = vec![
        make_applied_rule(111, RuleType::Surcharge, 8.0, true), // skipped
    ];
    snapshot.order_rule_surcharge_amount = 8.0;

    let eff = effective_order_rule_surcharge(&snapshot, to_decimal(100.0));
    assert_eq!(eff, Decimal::ZERO, "Skipped order surcharge → zero");
}

#[test]
fn test_effective_order_rule_discount_legacy_fallback() {
    let mut snapshot = OrderSnapshot::new("order-1".to_string());
    snapshot.order_applied_rules = vec![];
    snapshot.order_rule_discount_amount = 12.0;

    let eff = effective_order_rule_discount(&snapshot, to_decimal(100.0));
    assert_eq!(
        to_f64(eff),
        12.0,
        "None order_applied_rules → legacy fallback"
    );
}

#[test]
fn test_effective_order_rule_surcharge_legacy_fallback() {
    let mut snapshot = OrderSnapshot::new("order-1".to_string());
    snapshot.order_applied_rules = vec![];
    snapshot.order_rule_surcharge_amount = 6.0;

    let eff = effective_order_rule_surcharge(&snapshot, to_decimal(100.0));
    assert_eq!(
        to_f64(eff),
        6.0,
        "None order_applied_rules → legacy fallback"
    );
}

#[test]
fn test_recalculate_totals_remaining_amount_with_skipped_rule_and_payment() {
    // Verify remaining_amount is correct when a rule is skipped and partial payment exists
    let mut snapshot = OrderSnapshot::new("order-1".to_string());
    snapshot.items.push(make_item_with_rules(
        100.0,
        vec![make_applied_rule(1, RuleType::Discount, 20.0, false)],
        Some(20.0),
        None,
    ));
    snapshot.paid_amount = 50.0;

    recalculate_totals(&mut snapshot);
    // subtotal = 80, total = 80, remaining = 80 - 50 = 30
    assert_eq!(snapshot.total, 80.0);
    assert_eq!(snapshot.remaining_amount, 30.0);

    // Skip the discount → total goes up
    snapshot.items[0].applied_rules[0].skipped = true;
    recalculate_totals(&mut snapshot);
    // subtotal = 100, total = 100, remaining = 100 - 50 = 50
    assert_eq!(snapshot.total, 100.0);
    assert_eq!(snapshot.remaining_amount, 50.0);
}

// ========================================================================
// 动态重算新增测试
// ========================================================================

#[test]
fn test_rule_discount_recalculates_after_manual_discount_change() {
    // Core bug scenario: 10% rule discount, manual changes 0→50%
    // Rule discount should recalculate based on after_manual price
    let mut item = make_item_with_rules(
        100.0,
        vec![make_applied_rule(1, RuleType::Discount, 10.0, false)],
        None,
        None,
    );

    // No manual discount: rule_discount = 100 * 10% = 10, unit_price = 90
    let up1 = calculate_unit_price(&item);
    assert_eq!(to_f64(up1), 90.0);

    // Add 50% manual discount: after_manual = 50, rule_discount = 50 * 10% = 5
    // unit_price = 100 - 50 - 5 = 45
    item.manual_discount_percent = Some(50.0);
    let up2 = calculate_unit_price(&item);
    assert_eq!(
        to_f64(up2),
        45.0,
        "Rule discount should recalculate based on after_manual"
    );
}

#[test]
fn test_rule_surcharge_uses_base_not_after_manual() {
    // Surcharge should be based on base_with_options, not after_manual
    let mut item = make_item_with_rules(
        100.0,
        vec![make_applied_rule(11, RuleType::Surcharge, 10.0, false)],
        None,
        None,
    );
    item.manual_discount_percent = Some(50.0);

    let up = calculate_unit_price(&item);
    // base = 100, manual = 50, surcharge = 100 * 10% = 10 (based on base, not after_manual)
    // unit_price = 100 - 50 + 10 = 60
    assert_eq!(
        to_f64(up),
        60.0,
        "Surcharge should use base_with_options, not after_manual"
    );
}

#[test]
fn test_fixed_amount_rule_unaffected_by_manual_discount() {
    // FixedAmount rule discount stays constant regardless of manual discount
    let mut item = make_item_with_rules(100.0, vec![], None, None);
    item.applied_rules = vec![AppliedRule {
        rule_id: 1,
        name: "r1".to_string(),
        display_name: "r1".to_string(),
        receipt_name: "R".to_string(),
        rule_type: RuleType::Discount,
        adjustment_type: AdjustmentType::FixedAmount,
        product_scope: ProductScope::Global,
        zone_scope: "zone:all".to_string(),
        adjustment_value: 5.0,
        calculated_amount: 0.0,
        is_stackable: true,
        is_exclusive: false,
        skipped: false,
    }];

    // No manual discount: unit_price = 100 - 5 = 95
    let up1 = calculate_unit_price(&item);
    assert_eq!(to_f64(up1), 95.0);

    // With 50% manual: unit_price = 100 - 50 - 5 = 45
    item.manual_discount_percent = Some(50.0);
    let up2 = calculate_unit_price(&item);
    assert_eq!(to_f64(up2), 45.0, "FixedAmount discount should be constant");
}

#[test]
fn test_order_rule_recalculates_on_subtotal_change() {
    // Order-level 10% discount should recalculate when subtotal changes
    let mut snapshot = OrderSnapshot::new("order-1".to_string());
    let mut item = make_item_with_rules(100.0, vec![], None, None);
    item.applied_rules = vec![];
    snapshot.items.push(item);

    snapshot.order_applied_rules = vec![make_applied_rule(101, RuleType::Discount, 10.0, false)];

    recalculate_totals(&mut snapshot);
    // subtotal = 100, order_discount = 100 * 10% = 10, total = 90
    assert_eq!(snapshot.total, 90.0);

    // Change item price → subtotal changes
    snapshot.items[0].price = 200.0;
    snapshot.items[0].original_price = 200.0;
    recalculate_totals(&mut snapshot);
    // subtotal = 200, order_discount = 200 * 10% = 20, total = 180
    assert_eq!(
        snapshot.total, 180.0,
        "Order rule discount should recalculate on subtotal change"
    );
}

#[test]
fn test_recalculate_updates_calculated_amount_in_snapshot() {
    // Verify that recalculate_totals syncs calculated_amount in applied_rules
    let mut snapshot = OrderSnapshot::new("order-1".to_string());
    snapshot.items.push(make_item_with_rules(
        100.0,
        vec![make_applied_rule(1, RuleType::Discount, 10.0, false)],
        None,
        None,
    ));

    recalculate_totals(&mut snapshot);

    // calculated_amount should be synced: 100 * 10% = 10.0
    let ca = snapshot.items[0].applied_rules[0].calculated_amount;
    assert_eq!(
        ca, 10.0,
        "calculated_amount should be synced by recalculate_totals"
    );

    // Change manual discount → after_manual changes → calculated_amount should update
    snapshot.items[0].manual_discount_percent = Some(50.0);
    recalculate_totals(&mut snapshot);

    // after_manual = 50, rule discount = 50 * 10% = 5.0
    let ca2 = snapshot.items[0].applied_rules[0].calculated_amount;
    assert_eq!(
        ca2, 5.0,
        "calculated_amount should update after manual discount change"
    );
}

#[test]
fn test_recalculate_updates_order_calculated_amount_in_snapshot() {
    // Verify that recalculate_totals syncs calculated_amount in order_applied_rules
    let mut snapshot = OrderSnapshot::new("order-1".to_string());
    let mut item = make_item_with_rules(100.0, vec![], None, None);
    item.applied_rules = vec![];
    snapshot.items.push(item);

    snapshot.order_applied_rules = vec![make_applied_rule(101, RuleType::Discount, 10.0, false)];

    recalculate_totals(&mut snapshot);
    let ca = snapshot.order_applied_rules[0].calculated_amount;
    assert_eq!(ca, 10.0, "order calculated_amount should be 100*10%=10");

    // Change item price → subtotal changes
    snapshot.items[0].price = 200.0;
    snapshot.items[0].original_price = 200.0;
    recalculate_totals(&mut snapshot);
    let ca2 = snapshot.order_applied_rules[0].calculated_amount;
    assert_eq!(
        ca2, 20.0,
        "order calculated_amount should update to 200*10%=20"
    );
}

#[test]
fn test_recalculate_totals_with_option_quantity() {
    // Verify recalculate_totals correctly handles option quantity multiplication
    let mut snapshot = OrderSnapshot::new("order-1".to_string());

    // Item with options that have quantity > 1
    let item = CartItemSnapshot {
        id: 1,
        instance_id: "i1".to_string(),
        name: "Noodles".to_string(),
        price: 16.0, // base 10 + options 6
        original_price: 10.0,
        quantity: 2, // 2 bowls
        unpaid_quantity: 2,
        selected_options: Some(vec![shared::order::ItemOption {
            attribute_id: 1,
            attribute_name: "加蛋".to_string(),
            option_id: 0,
            option_name: "鸡蛋".to_string(),
            price_modifier: Some(2.0), // +2 per egg
            quantity: 3,               // 3 eggs per bowl
        }]),
        selected_specification: None,
        manual_discount_percent: None,
        rule_discount_amount: 0.0,
        rule_surcharge_amount: 0.0,
        applied_rules: vec![],
        applied_mg_rules: vec![],
        mg_discount_amount: 0.0,
        unit_price: 0.0,
        line_total: 0.0,
        tax: 0.0,
        tax_rate: 0,
        note: None,
        authorizer_id: None,
        authorizer_name: None,
        category_id: None,
        category_name: None,
        is_comped: false,
    };
    snapshot.items.push(item);

    recalculate_totals(&mut snapshot);

    // base_price = 10.0
    // options = 2.0 * 3 = 6.0
    // unit_price = 16.0
    // line_total = 16.0 * 2 = 32.0
    assert_eq!(snapshot.items[0].unit_price, 16.0);
    assert_eq!(snapshot.items[0].line_total, 32.0);
    assert_eq!(snapshot.subtotal, 32.0);
    assert_eq!(snapshot.total, 32.0);
}

// ========================================================================
// 负数选项修改器防护测试 (price_modifier bug fix)
// ========================================================================

fn make_item_with_options(
    original_price: f64,
    options: Vec<shared::order::ItemOption>,
) -> CartItemSnapshot {
    CartItemSnapshot {
        id: 1,
        instance_id: "i1".to_string(),
        name: "Item".to_string(),
        price: original_price,
        original_price,
        quantity: 1,
        unpaid_quantity: 1,
        selected_options: Some(options),
        selected_specification: None,
        manual_discount_percent: None,
        rule_discount_amount: 0.0,
        rule_surcharge_amount: 0.0,
        applied_rules: vec![],
        applied_mg_rules: vec![],
        mg_discount_amount: 0.0,
        unit_price: 0.0,
        line_total: 0.0,
        tax: 0.0,
        tax_rate: 0,
        note: None,
        authorizer_id: None,
        authorizer_name: None,
        category_id: None,
        category_name: None,
        is_comped: false,
    }
}

fn make_option(name: &str, price_modifier: f64, quantity: i32) -> shared::order::ItemOption {
    shared::order::ItemOption {
        attribute_id: 1,
        attribute_name: "Attr".to_string(),
        option_id: 0,
        option_name: name.to_string(),
        price_modifier: Some(price_modifier),
        quantity,
    }
}

#[test]
fn test_negative_option_modifier_exceeding_base_price_clamps_unit_price() {
    // Bug scenario: Cortado 3.50€ with option -100€ → unit_price must be 0, not -96.50
    let item = make_item_with_options(3.50, vec![make_option("Avena", -100.0, 1)]);
    let up = calculate_unit_price(&item);
    assert_eq!(
        up,
        Decimal::ZERO,
        "Negative option exceeding base must clamp unit_price to 0"
    );
}

#[test]
fn test_negative_option_modifier_within_base_price() {
    // Valid scenario: -0.50€ discount option on 3.50€ item → 3.00€
    let item = make_item_with_options(3.50, vec![make_option("No cream", -0.50, 1)]);
    let up = calculate_unit_price(&item);
    assert_eq!(
        to_f64(up),
        3.0,
        "Small negative option within base price should work"
    );
}

#[test]
fn test_negative_option_modifier_exactly_equals_base_price() {
    // Edge case: option modifier exactly negates base price → 0
    let item = make_item_with_options(5.0, vec![make_option("Free modifier", -5.0, 1)]);
    let up = calculate_unit_price(&item);
    assert_eq!(
        up,
        Decimal::ZERO,
        "Option modifier equal to base price should yield 0"
    );
}

#[test]
fn test_multiple_negative_options_exceed_base() {
    // Multiple negative options that together exceed base price
    let item = make_item_with_options(
        10.0,
        vec![
            make_option("Discount A", -4.0, 1),
            make_option("Discount B", -4.0, 1),
            make_option("Discount C", -4.0, 1),
        ],
    );
    let up = calculate_unit_price(&item);
    // base=10, options=-12, base_with_options=max(-2, 0)=0
    assert_eq!(
        up,
        Decimal::ZERO,
        "Combined negative options exceeding base must clamp to 0"
    );
}

#[test]
fn test_negative_option_with_quantity_exceeds_base() {
    // -2€ × quantity 10 = -20€ on 5€ item
    let item = make_item_with_options(5.0, vec![make_option("Discount", -2.0, 10)]);
    let up = calculate_unit_price(&item);
    // base=5, options=-2*10=-20, base_with_options=max(-15,0)=0
    assert_eq!(
        up,
        Decimal::ZERO,
        "Negative option * quantity exceeding base must clamp to 0"
    );
}

#[test]
fn test_recalculate_totals_negative_option_original_total_never_negative() {
    // The original bug: original_total was -85.50€
    let mut snapshot = OrderSnapshot::new("order-1".to_string());

    // Normal item
    snapshot.items.push(make_item_with_options(5.50, vec![]));
    snapshot.items[0].selected_options = None;

    // Item with absurd negative modifier
    let mut bad_item = make_item_with_options(3.50, vec![make_option("Avena", -100.0, 1)]);
    bad_item.id = 2;
    bad_item.instance_id = "i2".to_string();
    snapshot.items.push(bad_item);

    recalculate_totals(&mut snapshot);

    assert!(
        snapshot.original_total >= 0.0,
        "original_total must never be negative, got {}",
        snapshot.original_total
    );
    assert!(
        snapshot.subtotal >= 0.0,
        "subtotal must never be negative, got {}",
        snapshot.subtotal
    );
    assert!(
        snapshot.total >= 0.0,
        "total must never be negative, got {}",
        snapshot.total
    );
    // original_total = 5.50 + max(3.50-100, 0) = 5.50 + 0 = 5.50
    assert_eq!(snapshot.original_total, 5.5);
}

#[test]
fn test_recalculate_totals_all_items_negative_options() {
    // Every item has options that exceed base price
    let mut snapshot = OrderSnapshot::new("order-1".to_string());

    let mut item1 = make_item_with_options(3.50, vec![make_option("Bad", -100.0, 1)]);
    item1.instance_id = "i1".to_string();
    snapshot.items.push(item1);

    let mut item2 = make_item_with_options(5.50, vec![make_option("Bad2", -200.0, 1)]);
    item2.id = 2;
    item2.instance_id = "i2".to_string();
    snapshot.items.push(item2);

    recalculate_totals(&mut snapshot);

    assert_eq!(
        snapshot.original_total, 0.0,
        "All-negative items should yield 0 original_total"
    );
    assert_eq!(snapshot.subtotal, 0.0);
    assert_eq!(snapshot.total, 0.0);
    assert_eq!(snapshot.remaining_amount, 0.0);
}

#[test]
fn test_recalculate_totals_comped_item_with_negative_option() {
    // Comped item with negative option modifier — comp_total must not go negative
    let mut snapshot = OrderSnapshot::new("order-1".to_string());

    let mut item = make_item_with_options(3.50, vec![make_option("Avena", -100.0, 1)]);
    item.is_comped = true;
    snapshot.items.push(item);

    recalculate_totals(&mut snapshot);

    assert!(
        snapshot.comp_total_amount >= 0.0,
        "comp_total_amount must never be negative, got {}",
        snapshot.comp_total_amount
    );
    assert_eq!(snapshot.comp_total_amount, 0.0);
}

#[test]
fn test_recalculate_totals_negative_option_with_multiple_quantity() {
    // Item quantity > 1 with negative option
    let mut snapshot = OrderSnapshot::new("order-1".to_string());

    let mut item = make_item_with_options(3.50, vec![make_option("Bad", -100.0, 1)]);
    item.quantity = 5;
    item.unpaid_quantity = 5;
    snapshot.items.push(item);

    recalculate_totals(&mut snapshot);

    // base_with_options = max(3.50 - 100, 0) = 0
    // original_total = 0 * 5 = 0
    assert_eq!(snapshot.original_total, 0.0);
    assert_eq!(snapshot.subtotal, 0.0);
    assert_eq!(snapshot.total, 0.0);
}

#[test]
fn test_validate_cart_item_option_quantity_must_be_positive() {
    use shared::order::CartItemInput;

    let input = CartItemInput {
        product_id: 1,
        name: "Item".to_string(),
        price: 10.0,
        original_price: None,
        quantity: 1,
        selected_options: Some(vec![shared::order::ItemOption {
            attribute_id: 1,
            attribute_name: "Attr".to_string(),
            option_id: 0,
            option_name: "Opt".to_string(),
            price_modifier: Some(1.0),
            quantity: 0, // Invalid!
        }]),
        selected_specification: None,
        manual_discount_percent: None,
        note: None,
        authorizer_id: None,
        authorizer_name: None,
    };

    let result = validate_cart_item(&input);
    assert!(result.is_err(), "Option quantity=0 must be rejected");
}

#[test]
fn test_validate_cart_item_option_quantity_negative() {
    use shared::order::CartItemInput;

    let input = CartItemInput {
        product_id: 1,
        name: "Item".to_string(),
        price: 10.0,
        original_price: None,
        quantity: 1,
        selected_options: Some(vec![shared::order::ItemOption {
            attribute_id: 1,
            attribute_name: "Attr".to_string(),
            option_id: 0,
            option_name: "Opt".to_string(),
            price_modifier: Some(1.0),
            quantity: -1, // Invalid!
        }]),
        selected_specification: None,
        manual_discount_percent: None,
        note: None,
        authorizer_id: None,
        authorizer_name: None,
    };

    let result = validate_cart_item(&input);
    assert!(result.is_err(), "Option quantity=-1 must be rejected");
}

#[test]
fn test_validate_cart_item_option_exceeds_max_quantity() {
    use shared::order::CartItemInput;

    let input = CartItemInput {
        product_id: 1,
        name: "Item".to_string(),
        price: 10.0,
        original_price: None,
        quantity: 1,
        selected_options: Some(vec![shared::order::ItemOption {
            attribute_id: 1,
            attribute_name: "Attr".to_string(),
            option_id: 0,
            option_name: "Opt".to_string(),
            price_modifier: Some(1.0),
            quantity: MAX_OPTION_QUANTITY + 1, // Exceeds max
        }]),
        selected_specification: None,
        manual_discount_percent: None,
        note: None,
        authorizer_id: None,
        authorizer_name: None,
    };

    let result = validate_cart_item(&input);
    assert!(
        result.is_err(),
        "Option quantity exceeding MAX must be rejected"
    );
}
