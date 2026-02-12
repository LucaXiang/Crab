use super::*;
use shared::order::types::ServiceType;
use shared::order::{CartItemInput, OrderCommandPayload, OrderEventType, PaymentInput, VoidType};



fn create_test_manager() -> OrdersManager {
    let storage = OrderStorage::open_in_memory().unwrap();
    OrdersManager::with_storage(storage)
}


fn create_open_table_cmd(operator_id: i64) -> OrderCommand {
    OrderCommand::new(
        operator_id,
        "Test Operator".to_string(),
        OrderCommandPayload::OpenTable {
            table_id: Some(1),
            table_name: Some("Table 1".to_string()),
            zone_id: None,
            zone_name: None,
            guest_count: 2,
            is_retail: false,
        },
    )
}


// ========================================================================
// Helper: open a table with items
// ========================================================================

fn open_table_with_items(
    manager: &OrdersManager,
    table_id: i64,
    items: Vec<CartItemInput>,
) -> String {
    let open_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::OpenTable {
            table_id: Some(table_id),
            table_name: Some(format!("Table {}", table_id)),
            zone_id: Some(1),
            zone_name: Some("Zone A".to_string()),
            guest_count: 2,
            is_retail: false,
        },
    );
    let resp = manager.execute_command(open_cmd);
    assert!(resp.success, "Failed to open table");
    let order_id = resp.order_id.unwrap();

    if !items.is_empty() {
        let add_cmd = OrderCommand::new(
            1,
            "Test Operator".to_string(),
            OrderCommandPayload::AddItems {
                order_id: order_id.clone(),
                items,
            },
        );
        let resp = manager.execute_command(add_cmd);
        assert!(resp.success, "Failed to add items");
    }

    order_id
}


fn simple_item(product_id: i64, name: &str, price: f64, quantity: i32) -> CartItemInput {
    CartItemInput {
        product_id,
        name: name.to_string(),
        price,
        original_price: None,
        quantity,
        selected_options: None,
        selected_specification: None,
        manual_discount_percent: None,
        note: None,
        authorizer_id: None,
        authorizer_name: None,
    }
}


// ========================================================================
// 规则快照持久化测试
// ========================================================================

fn create_test_rule(name: &str) -> PriceRule {
    use shared::models::price_rule::{AdjustmentType, ProductScope, RuleType};
    PriceRule {
        id: 0,
        name: name.to_string(),
        display_name: name.to_string(),
        receipt_name: name.to_string(),
        description: None,
        rule_type: RuleType::Discount,
        product_scope: ProductScope::Global,
        target_id: None,
        zone_scope: "all".to_string(),
        adjustment_type: AdjustmentType::Percentage,
        adjustment_value: 10.0,
        is_stackable: false,
        is_exclusive: false,
        valid_from: None,
        valid_until: None,
        active_days: None,
        active_start_time: None,
        active_end_time: None,
        is_active: true,
        created_by: None,
        created_at: 0,
    }
}


// ========================================================================
// ========================================================================
//  新增测试工具函数
// ========================================================================
// ========================================================================

/// 创建带选项的商品
fn item_with_options(
    product_id: i64,
    name: &str,
    price: f64,
    quantity: i32,
    options: Vec<shared::order::ItemOption>,
) -> CartItemInput {
    CartItemInput {
        product_id,
        name: name.to_string(),
        price,
        original_price: None,
        quantity,
        selected_options: Some(options),
        selected_specification: None,
        manual_discount_percent: None,
        note: None,
        authorizer_id: None,
        authorizer_name: None,
    }
}


/// 创建带折扣的商品
fn item_with_discount(
    product_id: i64,
    name: &str,
    price: f64,
    quantity: i32,
    discount_percent: f64,
) -> CartItemInput {
    CartItemInput {
        product_id,
        name: name.to_string(),
        price,
        original_price: None,
        quantity,
        selected_options: None,
        selected_specification: None,
        manual_discount_percent: Some(discount_percent),
        note: None,
        authorizer_id: None,
        authorizer_name: None,
    }
}



/// 快速完成订单
fn complete_order(manager: &OrdersManager, order_id: &str) -> CommandResponse {
    let complete_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::CompleteOrder {
            order_id: order_id.to_string(),
            service_type: Some(ServiceType::DineIn),
        },
    );
    manager.execute_command(complete_cmd)
}


/// 快速作废订单
fn void_order_helper(manager: &OrdersManager, order_id: &str, void_type: VoidType) -> CommandResponse {
    let void_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::VoidOrder {
            order_id: order_id.to_string(),
            void_type,
            loss_reason: None,
            loss_amount: None,
            note: None,
            authorizer_id: None,
            authorizer_name: None,
        },
    );
    manager.execute_command(void_cmd)
}


/// 断言订单状态
fn assert_order_status(manager: &OrdersManager, order_id: &str, expected: OrderStatus) {
    let snapshot = manager.get_snapshot(order_id).unwrap().unwrap();
    assert_eq!(
        snapshot.status, expected,
        "Expected order status {:?}, got {:?}",
        expected, snapshot.status
    );
}




/// 打开零售订单
fn open_retail_order(manager: &OrdersManager) -> String {
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
    assert!(resp.success, "Failed to open retail order");
    resp.order_id.unwrap()
}


// ========================================================================
// Edge-case combo tests: 奇怪组合场景
// ========================================================================

/// Helper: 修改商品（折扣/价格/数量）
fn modify_item(
    manager: &OrdersManager,
    order_id: &str,
    instance_id: &str,
    changes: shared::order::ItemChanges,
) -> CommandResponse {
    let cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::ModifyItem {
            order_id: order_id.to_string(),
            instance_id: instance_id.to_string(),
            affected_quantity: None,
            changes,
            authorizer_id: None,
            authorizer_name: None,
        },
    );
    manager.execute_command(cmd)
}


/// Helper: 添加支付
fn pay(manager: &OrdersManager, order_id: &str, amount: f64, method: &str) -> CommandResponse {
    let cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::AddPayment {
            order_id: order_id.to_string(),
            payment: PaymentInput {
                method: method.to_string(),
                amount,
                tendered: if method == "CASH" { Some(amount) } else { None },
                note: None,
            },
        },
    );
    manager.execute_command(cmd)
}


/// Helper: 取消支付
fn cancel_payment(
    manager: &OrdersManager,
    order_id: &str,
    payment_id: &str,
) -> CommandResponse {
    let cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::CancelPayment {
            order_id: order_id.to_string(),
            payment_id: payment_id.to_string(),
            reason: Some("test".to_string()),
            authorizer_id: None,
            authorizer_name: None,
        },
    );
    manager.execute_command(cmd)
}


/// Helper: 整单折扣
fn apply_discount(manager: &OrdersManager, order_id: &str, percent: f64) -> CommandResponse {
    let cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::ApplyOrderDiscount {
            order_id: order_id.to_string(),
            discount_percent: Some(percent),
            discount_fixed: None,
            authorizer_id: None,
            authorizer_name: None,
        },
    );
    manager.execute_command(cmd)
}


/// Helper: 清除整单折扣
fn clear_discount(manager: &OrdersManager, order_id: &str) -> CommandResponse {
    let cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::ApplyOrderDiscount {
            order_id: order_id.to_string(),
            discount_percent: Some(0.0),
            discount_fixed: None,
            authorizer_id: None,
            authorizer_name: None,
        },
    );
    manager.execute_command(cmd)
}


/// Helper: 分单支付（按商品）
fn split_by_items(
    manager: &OrdersManager,
    order_id: &str,
    items: Vec<shared::order::SplitItem>,
    method: &str,
) -> CommandResponse {
    let cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::SplitByItems {
            order_id: order_id.to_string(),
            items,
            payment_method: method.to_string(),
            tendered: None,
        },
    );
    manager.execute_command(cmd)
}


/// Helper: comp 商品 (comp all unpaid quantity)
fn comp_item(manager: &OrdersManager, order_id: &str, instance_id: &str) -> CommandResponse {
    let s = manager.get_snapshot(order_id).unwrap().unwrap();
    let qty = s.items.iter()
        .find(|i| i.instance_id == instance_id)
        .map(|i| i.unpaid_quantity)
        .unwrap_or(1);
    let cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::CompItem {
            order_id: order_id.to_string(),
            instance_id: instance_id.to_string(),
            quantity: qty,
            reason: "test comp".to_string(),
            authorizer_id: 1,
            authorizer_name: "Test".to_string(),
        },
    );
    manager.execute_command(cmd)
}


/// Helper: 折扣 changes
fn discount_changes(percent: f64) -> shared::order::ItemChanges {
    shared::order::ItemChanges {
        price: None,
        quantity: None,
        manual_discount_percent: Some(percent),
        note: None,
        selected_options: None,
        selected_specification: None,
    }
}


/// Helper: 价格 changes
fn price_changes(price: f64) -> shared::order::ItemChanges {
    shared::order::ItemChanges {
        price: Some(price),
        quantity: None,
        manual_discount_percent: None,
        note: None,
        selected_options: None,
        selected_specification: None,
    }
}


/// Helper: 数量 changes
fn qty_changes(qty: i32) -> shared::order::ItemChanges {
    shared::order::ItemChanges {
        price: None,
        quantity: Some(qty),
        manual_discount_percent: None,
        note: None,
        selected_options: None,
        selected_specification: None,
    }
}


/// Helper: 验证快照一致性 (stored vs rebuilt from events)
fn assert_snapshot_consistent(manager: &OrdersManager, order_id: &str) {
    let stored = manager.get_snapshot(order_id).unwrap().unwrap();
    let rebuilt = manager.rebuild_snapshot(order_id).unwrap();
    assert_eq!(
        stored.state_checksum, rebuilt.state_checksum,
        "Snapshot diverged from event replay!\n  stored items: {:?}\n  rebuilt items: {:?}\n  stored paid_amount: {}\n  rebuilt paid_amount: {}",
        stored.items.iter().map(|i| (&i.instance_id, i.quantity, i.unpaid_quantity)).collect::<Vec<_>>(),
        rebuilt.items.iter().map(|i| (&i.instance_id, i.quantity, i.unpaid_quantity)).collect::<Vec<_>>(),
        stored.paid_amount, rebuilt.paid_amount,
    );
}


// ========================================================================
// More complex combo tests: 支付→改动→取消→再操作 链式场景
// ========================================================================

/// Helper: 整单附加费
fn apply_surcharge(manager: &OrdersManager, order_id: &str, percent: f64) -> CommandResponse {
    let cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::ApplyOrderSurcharge {
            order_id: order_id.to_string(),
            surcharge_percent: Some(percent),
            surcharge_amount: None,
            authorizer_id: None,
            authorizer_name: None,
        },
    );
    manager.execute_command(cmd)
}


/// Helper: 整单固定附加费
fn apply_surcharge_fixed(manager: &OrdersManager, order_id: &str, amount: f64) -> CommandResponse {
    let cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::ApplyOrderSurcharge {
            order_id: order_id.to_string(),
            surcharge_percent: None,
            surcharge_amount: Some(amount),
            authorizer_id: None,
            authorizer_name: None,
        },
    );
    manager.execute_command(cmd)
}


/// Helper: 整单固定折扣
fn apply_discount_fixed(manager: &OrdersManager, order_id: &str, amount: f64) -> CommandResponse {
    let cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::ApplyOrderDiscount {
            order_id: order_id.to_string(),
            discount_percent: None,
            discount_fixed: Some(amount),
            authorizer_id: None,
            authorizer_name: None,
        },
    );
    manager.execute_command(cmd)
}


/// Helper: 删除商品
fn remove_item(manager: &OrdersManager, order_id: &str, instance_id: &str, qty: Option<i32>) -> CommandResponse {
    let cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::RemoveItem {
            order_id: order_id.to_string(),
            instance_id: instance_id.to_string(),
            quantity: qty,
            reason: Some("test".to_string()),
            authorizer_id: None,
            authorizer_name: None,
        },
    );
    manager.execute_command(cmd)
}


/// Helper: uncomp 商品
fn uncomp_item(manager: &OrdersManager, order_id: &str, instance_id: &str) -> CommandResponse {
    let cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::UncompItem {
            order_id: order_id.to_string(),
            instance_id: instance_id.to_string(),
            authorizer_id: 1,
            authorizer_name: "Test".to_string(),
        },
    );
    manager.execute_command(cmd)
}


/// Helper: 添加更多商品
fn add_items(manager: &OrdersManager, order_id: &str, items: Vec<CartItemInput>) -> CommandResponse {
    let cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::AddItems {
            order_id: order_id.to_string(),
            items,
        },
    );
    manager.execute_command(cmd)
}


/// Helper: 验证 remaining_amount 字段和方法一致
fn assert_remaining_consistent(s: &shared::order::OrderSnapshot) {
    let computed = (s.total - s.paid_amount).max(0.0);
    assert!(
        (s.remaining_amount - computed).abs() < 0.02,
        "remaining_amount field({:.2}) diverged from total({:.2}) - paid({:.2}) = {:.2}",
        s.remaining_amount, s.total, s.paid_amount, computed
    );
}


// ========================================================================
// Price Rule + Options + Spec 复杂组合测试 (Tests 31-40)
// ========================================================================

/// Helper: 开台（不加商品）
fn open_table(manager: &OrdersManager, table_id: i64) -> String {
    let open_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::OpenTable {
            table_id: Some(table_id),
            table_name: Some(format!("Table {}", table_id)),
            zone_id: Some(1),
            zone_name: Some("Zone A".to_string()),
            guest_count: 2,
            is_retail: false,
        },
    );
    let resp = manager.execute_command(open_cmd);
    assert!(resp.success, "Failed to open table");
    resp.order_id.unwrap()
}


/// Helper: 跳过/恢复规则
fn toggle_rule_skip(
    manager: &OrdersManager,
    order_id: &str,
    rule_id: i64,
    skipped: bool,
) -> CommandResponse {
    let cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::ToggleRuleSkip {
            order_id: order_id.to_string(),
            rule_id,
            skipped,
        },
    );
    manager.execute_command(cmd)
}


/// Helper: 创建百分比折扣规则
fn make_discount_rule(id: i64, percent: f64) -> PriceRule {
    use shared::models::price_rule::*;
    PriceRule {
        id,
        name: format!("discount_{}", id),
        display_name: format!("Discount {}", id),
        receipt_name: "DISC".to_string(),
        description: None,
        rule_type: RuleType::Discount,
        product_scope: ProductScope::Global,
        target_id: None,
        zone_scope: "all".to_string(),
        adjustment_type: AdjustmentType::Percentage,
        adjustment_value: percent,
        is_stackable: true,
        is_exclusive: false,
        valid_from: None,
        valid_until: None,
        active_days: None,
        active_start_time: None,
        active_end_time: None,
        is_active: true,
        created_by: None,
        created_at: 0,
    }
}


/// Helper: 创建百分比附加费规则
fn make_surcharge_rule(id: i64, percent: f64) -> PriceRule {
    use shared::models::price_rule::*;
    PriceRule {
        id,
        name: format!("surcharge_{}", id),
        display_name: format!("Surcharge {}", id),
        receipt_name: "SURCH".to_string(),
        description: None,
        rule_type: RuleType::Surcharge,
        product_scope: ProductScope::Global,
        target_id: None,
        zone_scope: "all".to_string(),
        adjustment_type: AdjustmentType::Percentage,
        adjustment_value: percent,
        is_stackable: true,
        is_exclusive: false,
        valid_from: None,
        valid_until: None,
        active_days: None,
        active_start_time: None,
        active_end_time: None,
        is_active: true,
        created_by: None,
        created_at: 0,
    }
}


/// Helper: 创建固定金额折扣规则
fn make_fixed_discount_rule(id: i64, amount: f64) -> PriceRule {
    use shared::models::price_rule::*;
    PriceRule {
        id,
        name: format!("fixed_discount_{}", id),
        display_name: format!("Fixed Discount {}", id),
        receipt_name: "FDISC".to_string(),
        description: None,
        rule_type: RuleType::Discount,
        product_scope: ProductScope::Global,
        target_id: None,
        zone_scope: "all".to_string(),
        adjustment_type: AdjustmentType::FixedAmount,
        adjustment_value: amount,
        is_stackable: true,
        is_exclusive: false,
        valid_from: None,
        valid_until: None,
        active_days: None,
        active_start_time: None,
        active_end_time: None,
        is_active: true,
        created_by: None,
        created_at: 0,
    }
}


/// Helper: 带规格的商品
fn item_with_spec(
    product_id: i64,
    name: &str,
    price: f64,
    quantity: i32,
    spec: shared::order::SpecificationInfo,
) -> CartItemInput {
    CartItemInput {
        product_id,
        name: name.to_string(),
        price,
        original_price: None,
        quantity,
        selected_options: None,
        selected_specification: Some(spec),
        manual_discount_percent: None,
        note: None,
        authorizer_id: None,
        authorizer_name: None,
    }
}


/// Helper: 创建选项
fn make_option(attr_id: i64, attr_name: &str, id: i64, opt_name: &str, modifier: f64) -> shared::order::ItemOption {
    shared::order::ItemOption {
        attribute_id: attr_id,
        attribute_name: attr_name.to_string(),
        option_id: id,
        option_name: opt_name.to_string(),
        price_modifier: Some(modifier),
        quantity: 1,
    }
}


/// Helper: 创建规格
fn make_spec(id: i64, name: &str, price: Option<f64>) -> shared::order::SpecificationInfo {
    shared::order::SpecificationInfo {
        id,
        name: name.to_string(),
        receipt_name: None,
        price,
        is_multi_spec: false,
    }
}


/// Helper: 组合 changes
fn combo_changes(
    price: Option<f64>,
    qty: Option<i32>,
    discount: Option<f64>,
    options: Option<Vec<shared::order::ItemOption>>,
    spec: Option<shared::order::SpecificationInfo>,
) -> shared::order::ItemChanges {
    shared::order::ItemChanges {
        price,
        quantity: qty,
        manual_discount_percent: discount,
        note: None,
        selected_options: options,
        selected_specification: spec,
    }
}


/// 浮点断言 helper
fn assert_close(actual: f64, expected: f64, msg: &str) {
    assert!(
        (actual - expected).abs() < 0.02,
        "{}: expected {:.2}, got {:.2}",
        msg, expected, actual
    );
}


// ========================================================================
// 联动测试 (Tests 46-60): 暴露 comp/uncomp + rules + payment 交互 bug
// ========================================================================

/// Helper: comp 指定数量
fn comp_item_qty(
    manager: &OrdersManager,
    order_id: &str,
    instance_id: &str,
    quantity: i32,
) -> CommandResponse {
    let cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::CompItem {
            order_id: order_id.to_string(),
            instance_id: instance_id.to_string(),
            quantity,
            reason: "test comp".to_string(),
            authorizer_id: 1,
            authorizer_name: "Test".to_string(),
        },
    );
    manager.execute_command(cmd)
}


// ========================================================================
// AddItems 时间动态过滤测试 (Tests: time-filter)
// ========================================================================

/// Helper: 创建带时间约束的折扣规则
fn make_timed_discount_rule(
    id: i64,
    percent: f64,
    valid_from: Option<i64>,
    valid_until: Option<i64>,
    active_days: Option<Vec<u8>>,
    active_start_time: Option<&str>,
    active_end_time: Option<&str>,
) -> PriceRule {
    use shared::models::price_rule::*;
    PriceRule {
        id,
        name: format!("timed_{}", id),
        display_name: format!("Timed {}", id),
        receipt_name: "DISC".to_string(),
        description: None,
        rule_type: RuleType::Discount,
        product_scope: ProductScope::Global,
        target_id: None,
        zone_scope: "all".to_string(),
        adjustment_type: AdjustmentType::Percentage,
        adjustment_value: percent,
        is_stackable: true,
        is_exclusive: false,
        valid_from,
        valid_until,
        active_days,
        active_start_time: active_start_time.map(|s| s.to_string()),
        active_end_time: active_end_time.map(|s| s.to_string()),
        is_active: true,
        created_by: None,
        created_at: 0,
    }
}

mod test_core;
mod test_boundary;
mod test_rules;
mod test_flows;
mod test_combos;
mod test_rules_combo;
