use super::*;

#[tokio::test]
async fn test_cache_rules_persists_to_redb() {
    let manager = create_test_manager();
    let order_id = "order-persist-1";

    let rules = vec![create_test_rule("Lunch Special"), create_test_rule("VIP")];
    manager.cache_rules(order_id, rules);

    // 内存缓存应该有
    let cached = manager.get_cached_rules(order_id);
    assert!(cached.is_some());
    assert_eq!(cached.unwrap().len(), 2);

    // redb 也应该有
    let persisted = manager.storage().get_rule_snapshot(order_id).unwrap();
    assert!(persisted.is_some());
    assert_eq!(persisted.unwrap().len(), 2);
}

#[tokio::test]
async fn test_remove_cached_rules_cleans_redb() {
    let manager = create_test_manager();
    let order_id = "order-remove-1";

    manager.cache_rules(order_id, vec![create_test_rule("Rule")]);
    assert!(manager.get_cached_rules(order_id).is_some());
    assert!(
        manager
            .storage()
            .get_rule_snapshot(order_id)
            .unwrap()
            .is_some()
    );

    // 清除
    manager.remove_cached_rules(order_id);

    // 内存和 redb 都应该被清除
    assert!(manager.get_cached_rules(order_id).is_none());
    assert!(
        manager
            .storage()
            .get_rule_snapshot(order_id)
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn test_restore_rule_snapshots_from_redb() {
    let storage = OrderStorage::open_in_memory().unwrap();

    // 注册为活跃订单（模拟正常开台后的状态）
    {
        let txn = storage.begin_write().unwrap();
        storage.mark_order_active(&txn, "order-a").unwrap();
        storage.mark_order_active(&txn, "order-b").unwrap();
        txn.commit().unwrap();
    }

    // 写入规则快照（模拟上次运行遗留的快照）
    storage
        .store_rule_snapshot("order-a", &vec![create_test_rule("Rule A")])
        .unwrap();
    storage
        .store_rule_snapshot(
            "order-b",
            &vec![create_test_rule("Rule B1"), create_test_rule("Rule B2")],
        )
        .unwrap();

    // 创建新 manager（模拟重启，内存缓存为空）
    let manager = OrdersManager::with_storage(storage);
    assert!(manager.get_cached_rules("order-a").is_none());
    assert!(manager.get_cached_rules("order-b").is_none());

    // 恢复
    let count = manager.restore_rule_snapshots_from_redb();
    assert_eq!(count, 2);

    // 内存缓存应该有了
    let rules_a = manager.get_cached_rules("order-a").unwrap();
    assert_eq!(rules_a.len(), 1);
    assert_eq!(rules_a[0].name, "Rule A");

    let rules_b = manager.get_cached_rules("order-b").unwrap();
    assert_eq!(rules_b.len(), 2);
}

#[tokio::test]
async fn test_restore_rule_snapshots_cleans_orphans() {
    let storage = OrderStorage::open_in_memory().unwrap();

    // 只注册 order-a 为活跃，order-orphan 不注册（模拟崩溃后的孤儿快照）
    {
        let txn = storage.begin_write().unwrap();
        storage.mark_order_active(&txn, "order-a").unwrap();
        txn.commit().unwrap();
    }

    storage
        .store_rule_snapshot("order-a", &vec![create_test_rule("Rule A")])
        .unwrap();
    storage
        .store_rule_snapshot("order-orphan", &vec![create_test_rule("Orphan Rule")])
        .unwrap();

    let manager = OrdersManager::with_storage(storage);
    let count = manager.restore_rule_snapshots_from_redb();

    // 只恢复了活跃订单的规则
    assert_eq!(count, 1);
    assert!(manager.get_cached_rules("order-a").is_some());
    assert!(manager.get_cached_rules("order-orphan").is_none());

    // 孤儿快照应该已从 redb 中清除
    assert!(
        manager
            .storage()
            .get_rule_snapshot("order-orphan")
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn test_complete_order_cleans_rules() {
    let manager = create_test_manager();

    // 开台
    let open_cmd = create_open_table_cmd(1);
    let resp = manager.execute_command(open_cmd).await;
    let order_id = resp.order_id.unwrap();

    // 缓存规则 (10% global discount)
    manager.cache_rules(&order_id, vec![create_test_rule("Rule")]);
    assert!(manager.get_cached_rules(&order_id).is_some());

    // 加菜
    let add_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::AddItems {
            order_id: order_id.clone(),
            items: vec![simple_item(1, "Coffee", 10.0, 1)],
        },
    );
    manager.execute_command(add_cmd).await;

    // 查询实际 total（可能因规则折扣而与原价不同）
    let snapshot = manager.get_snapshot(&order_id).unwrap().unwrap();
    let actual_total = snapshot.total;

    // 支付实际 total
    let pay_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::AddPayment {
            order_id: order_id.clone(),
            payment: PaymentInput {
                method: "CASH".to_string(),
                amount: actual_total,
                tendered: Some(actual_total),
                note: None,
            },
        },
    );
    let pay_resp = manager.execute_command(pay_cmd).await;
    assert!(pay_resp.success, "Payment should succeed");

    // 完成订单
    let complete_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::CompleteOrder {
            order_id: order_id.clone(),
            service_type: Some(ServiceType::DineIn),
        },
    );
    manager.execute_command(complete_cmd).await;

    // 规则缓存和 redb 快照都应该被清除
    assert!(manager.get_cached_rules(&order_id).is_none());
    assert!(
        manager
            .storage()
            .get_rule_snapshot(&order_id)
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn test_void_order_cleans_rules() {
    let manager = create_test_manager();

    // 开台
    let open_cmd = create_open_table_cmd(1);
    let resp = manager.execute_command(open_cmd).await;
    let order_id = resp.order_id.unwrap();

    // 缓存规则
    manager.cache_rules(&order_id, vec![create_test_rule("Rule")]);
    assert!(manager.get_cached_rules(&order_id).is_some());

    // 作废订单
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

    // 规则缓存和 redb 快照都应该被清除
    assert!(manager.get_cached_rules(&order_id).is_none());
    assert!(
        manager
            .storage()
            .get_rule_snapshot(&order_id)
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn test_move_order_preserves_rules() {
    let manager = create_test_manager();

    let order_id =
        open_table_with_items(&manager, 245, vec![simple_item(1, "Coffee", 5.0, 1)]).await;

    // 缓存规则
    manager.cache_rules(&order_id, vec![create_test_rule("Rule")]);
    assert!(manager.get_cached_rules(&order_id).is_some());
    assert!(
        manager
            .storage()
            .get_rule_snapshot(&order_id)
            .unwrap()
            .is_some()
    );

    // 换桌 — 订单保持 Active，规则不清除
    // （实际场景中由调用方按新区域重新加载规则）
    let move_cmd = OrderCommand::new(
        1,
        "Test Operator".to_string(),
        OrderCommandPayload::MoveOrder {
            order_id: order_id.clone(),
            target_table_id: 331,
            target_table_name: "Table T-rule-move-2".to_string(),
            target_zone_id: Some(2),
            target_zone_name: Some("Zone B".to_string()),
            authorizer_id: None,
            authorizer_name: None,
        },
    );
    let resp = manager.execute_command(move_cmd).await;
    assert!(resp.success);

    // MoveOrder 不是 terminal 操作，规则保留（由调用方用新区域重载）
    assert!(manager.get_cached_rules(&order_id).is_some());
    assert!(
        manager
            .storage()
            .get_rule_snapshot(&order_id)
            .unwrap()
            .is_some()
    );
}

#[tokio::test]
async fn test_merge_orders_cleans_source_rules() {
    let manager = create_test_manager();

    // 源订单
    let source_id =
        open_table_with_items(&manager, 246, vec![simple_item(1, "Coffee", 10.0, 1)]).await;

    // 目标订单
    let target_id = open_table_with_items(&manager, 247, vec![simple_item(2, "Tea", 8.0, 1)]).await;

    // 给源订单缓存规则
    manager.cache_rules(&source_id, vec![create_test_rule("SourceRule")]);
    assert!(manager.get_cached_rules(&source_id).is_some());
    assert!(
        manager
            .storage()
            .get_rule_snapshot(&source_id)
            .unwrap()
            .is_some()
    );

    // 合并 source → target
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
    assert!(resp.success);

    // 源订单的规则缓存和 redb 快照都应该被清除
    assert!(manager.get_cached_rules(&source_id).is_none());
    assert!(
        manager
            .storage()
            .get_rule_snapshot(&source_id)
            .unwrap()
            .is_none()
    );
}

/// valid_from 在未来 → 规则不应用，商品原价
#[tokio::test]
async fn test_add_items_filters_rule_valid_from_future() {
    let manager = create_test_manager();
    let order_id = open_table(&manager, 201).await;

    let future = shared::util::now_millis() + 3_600_000; // 1 小时后
    let rule = make_timed_discount_rule(1, 10.0, Some(future), None, None, None, None);
    manager.cache_rules(&order_id, vec![rule]);

    let r = add_items(&manager, &order_id, vec![simple_item(1, "Steak", 100.0, 1)]).await;
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    // 规则被过滤掉，100€ 原价
    assert_eq!(s.subtotal, 100.0);
}

/// valid_until 已过期 → 规则不应用
#[tokio::test]
async fn test_add_items_filters_rule_valid_until_expired() {
    let manager = create_test_manager();
    let order_id = open_table(&manager, 202).await;

    let past = shared::util::now_millis() - 3_600_000; // 1 小时前
    let rule = make_timed_discount_rule(2, 10.0, None, Some(past), None, None, None);
    manager.cache_rules(&order_id, vec![rule]);

    let r = add_items(&manager, &order_id, vec![simple_item(1, "Wine", 50.0, 2)]).await;
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    // 过期规则不生效，50×2=100
    assert_eq!(s.subtotal, 100.0);
}

/// valid_from ≤ now ≤ valid_until → 规则生效
#[tokio::test]
async fn test_add_items_applies_rule_within_valid_range() {
    let manager = create_test_manager();
    let order_id = open_table(&manager, 203).await;

    let now = shared::util::now_millis();
    let rule = make_timed_discount_rule(
        7,
        10.0,
        Some(now - 3_600_000), // 1小时前开始
        Some(now + 3_600_000), // 1小时后结束
        None,
        None,
        None,
    );
    manager.cache_rules(&order_id, vec![rule]);

    let r = add_items(&manager, &order_id, vec![simple_item(1, "Pasta", 100.0, 1)]).await;
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    // 10% 折扣: 100 → 90
    assert_eq!(s.subtotal, 90.0);
}

/// active_days 不匹配当前星期几 → 规则不应用
#[tokio::test]
async fn test_add_items_filters_rule_wrong_day() {
    let manager = create_test_manager();
    let order_id = open_table(&manager, 204).await;

    // 获取当前是星期几 (0=Sun, 1=Mon, ..., 6=Sat)
    let now_local = chrono::Utc::now().with_timezone(&chrono_tz::Europe::Madrid);
    let today = now_local.format("%u").to_string().parse::<u8>().unwrap() % 7; // ISO weekday → 0-6

    // 设置 active_days 只包含"明天"
    let wrong_day = (today + 1) % 7;
    let rule = make_timed_discount_rule(3, 10.0, None, None, Some(vec![wrong_day]), None, None);
    manager.cache_rules(&order_id, vec![rule]);

    let r = add_items(&manager, &order_id, vec![simple_item(1, "Salad", 40.0, 1)]).await;
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    // 不匹配日期，40€ 原价
    assert_eq!(s.subtotal, 40.0);
}

/// active_days 匹配当前星期几 → 规则生效
#[tokio::test]
async fn test_add_items_applies_rule_matching_day() {
    let manager = create_test_manager();
    let order_id = open_table(&manager, 205).await;

    let now_local = chrono::Utc::now().with_timezone(&chrono_tz::Europe::Madrid);
    let today = now_local.format("%u").to_string().parse::<u8>().unwrap() % 7;

    let rule = make_timed_discount_rule(4, 20.0, None, None, Some(vec![today]), None, None);
    manager.cache_rules(&order_id, vec![rule]);

    let r = add_items(&manager, &order_id, vec![simple_item(1, "Pizza", 50.0, 2)]).await;
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    // 20% 折扣: 50×2=100 → 80
    assert_eq!(s.subtotal, 80.0);
}

/// active_start_time/active_end_time 不在当前时间范围 → 规则不应用
#[tokio::test]
async fn test_add_items_filters_rule_outside_time_window() {
    let manager = create_test_manager();
    let order_id = open_table(&manager, 206).await;

    // 构造一个绝对不在当前时间的窗口: 凌晨 03:00-04:00（除非真的在这个时间运行测试）
    // 使用更安全的方法：当前时间 +3h 到 +4h
    let now_local = chrono::Utc::now().with_timezone(&chrono_tz::Europe::Madrid);
    let hour = now_local.format("%H").to_string().parse::<u32>().unwrap();
    let start = format!("{:02}:00", (hour + 3) % 24);
    let end = format!("{:02}:00", (hour + 4) % 24);

    let rule = make_timed_discount_rule(5, 15.0, None, None, None, Some(&start), Some(&end));
    manager.cache_rules(&order_id, vec![rule]);

    let r = add_items(&manager, &order_id, vec![simple_item(1, "Soup", 20.0, 3)]).await;
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    // 不在时间窗口，20×3=60 原价
    assert_eq!(s.subtotal, 60.0);
}

/// 混合规则: 一个过期 + 一个有效 → 只有有效的应用
#[tokio::test]
async fn test_add_items_mixed_expired_and_active_rules() {
    let manager = create_test_manager();
    let order_id = open_table(&manager, 207).await;

    let now = shared::util::now_millis();
    let expired_rule = make_timed_discount_rule(
        8,
        50.0, // 50% 折扣 — 如果被应用会很明显
        None,
        Some(now - 3_600_000), // 1小时前过期
        None,
        None,
        None,
    );
    let active_rule = make_timed_discount_rule(
        9,
        10.0,
        Some(now - 3_600_000),
        Some(now + 3_600_000),
        None,
        None,
        None,
    );
    manager.cache_rules(&order_id, vec![expired_rule, active_rule]);

    let r = add_items(&manager, &order_id, vec![simple_item(1, "Fish", 200.0, 1)]).await;
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    // 只有 10% 有效: 200 → 180 (如果 50% 也生效会是 90)
    assert_eq!(s.subtotal, 180.0);
}

/// 无时间约束的规则始终生效
#[tokio::test]
async fn test_add_items_no_time_constraint_always_applies() {
    let manager = create_test_manager();
    let order_id = open_table(&manager, 208).await;

    // 没有任何时间限制
    let rule = make_timed_discount_rule(6, 10.0, None, None, None, None, None);
    manager.cache_rules(&order_id, vec![rule]);

    let r = add_items(&manager, &order_id, vec![simple_item(1, "Bread", 10.0, 5)]).await;
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    // 10% 折扣: 10×5=50 → 45
    assert_eq!(s.subtotal, 45.0);
}

/// valid_from + active_days 组合: valid_from 有效但 active_days 不匹配 → 不应用
#[tokio::test]
async fn test_add_items_valid_from_ok_but_wrong_day() {
    let manager = create_test_manager();
    let order_id = open_table(&manager, 209).await;

    let now = shared::util::now_millis();
    let now_local = chrono::Utc::now().with_timezone(&chrono_tz::Europe::Madrid);
    let today = now_local.format("%u").to_string().parse::<u8>().unwrap() % 7;
    let wrong_day = (today + 1) % 7;

    let rule = make_timed_discount_rule(
        11,
        10.0,
        Some(now - 3_600_000), // valid_from OK
        None,
        Some(vec![wrong_day]), // wrong day
        None,
        None,
    );
    manager.cache_rules(&order_id, vec![rule]);

    let r = add_items(&manager, &order_id, vec![simple_item(1, "Cake", 30.0, 2)]).await;
    assert!(r.success);

    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    // active_days 不匹配，30×2=60 原价
    assert_eq!(s.subtotal, 60.0);
}

/// 第二次加菜时规则也需要实时检查时间
#[tokio::test]
async fn test_add_items_second_batch_also_checks_time() {
    let manager = create_test_manager();
    let order_id = open_table(&manager, 210).await;

    let now = shared::util::now_millis();
    // 规则有效
    let rule = make_timed_discount_rule(
        12,
        10.0,
        Some(now - 3_600_000),
        Some(now + 3_600_000),
        None,
        None,
        None,
    );
    manager.cache_rules(&order_id, vec![rule]);

    // 第一批加菜
    let r = add_items(&manager, &order_id, vec![simple_item(1, "A", 100.0, 1)]).await;
    assert!(r.success);
    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    assert_eq!(s.subtotal, 90.0); // 10% off

    // 第二批加菜（规则仍然有效）
    let r = add_items(&manager, &order_id, vec![simple_item(2, "B", 50.0, 2)]).await;
    assert!(r.success);
    let s = manager.get_snapshot(&order_id).unwrap().unwrap();
    // A: 90, B: 45×2=90 → 180
    assert_eq!(s.subtotal, 180.0);
}
