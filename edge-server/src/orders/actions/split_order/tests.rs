use super::*;
use crate::orders::storage::OrderStorage;
use crate::orders::traits::{CommandContext, CommandHandler, CommandMetadata};
use shared::order::{CartItemSnapshot, EventPayload, OrderEventType, OrderSnapshot, OrderStatus, SplitItem};

fn create_test_metadata() -> CommandMetadata {
    CommandMetadata {
        command_id: "cmd-1".to_string(),
        operator_id: 1,
        operator_name: "Test User".to_string(),
        timestamp: 1234567890,
    }
}

fn create_active_order_with_items(order_id: &str) -> OrderSnapshot {
    let mut snapshot = OrderSnapshot::new(order_id.to_string());
    snapshot.status = OrderStatus::Active;
    snapshot.table_id = Some(1);
    snapshot.table_name = Some("Table 1".to_string());

    let item1 = CartItemSnapshot {
        id: 1,
        instance_id: "item-1".to_string(),
        name: "Coffee".to_string(),
        price: 10.0,
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
        unit_price: 0.0,
        line_total: 0.0,
        note: None,
        authorizer_id: None,
        authorizer_name: None,
        category_name: None,
        is_comped: false,
        tax: 0.0,
        tax_rate: 0,
    };
    let item2 = CartItemSnapshot {
        id: 2,
        instance_id: "item-2".to_string(),
        name: "Tea".to_string(),
        price: 8.0,
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
        unit_price: 0.0,
        line_total: 0.0,
        note: None,
        authorizer_id: None,
        authorizer_name: None,
        category_name: None,
        is_comped: false,
        tax: 0.0,
        tax_rate: 0,
    };
    snapshot.items.push(item1);
    snapshot.items.push(item2);
    snapshot.subtotal = 46.0; // 3*10 + 2*8
    snapshot.total = 46.0;

    snapshot
}

// ========== SplitByItems tests ==========

#[tokio::test]
async fn test_split_by_items_success() {
    let storage = OrderStorage::open_in_memory().unwrap();
    let txn = storage.begin_write().unwrap();

    let snapshot = create_active_order_with_items("order-1");
    storage.store_snapshot(&txn, &snapshot).unwrap();

    let current_seq = storage.get_next_sequence(&txn).unwrap();
    let mut ctx = CommandContext::new(&txn, &storage, current_seq);

    let action = SplitByItemsAction {
        order_id: "order-1".to_string(),
        payment_method: "CASH".to_string(),
        items: vec![SplitItem {
            instance_id: "item-1".to_string(),
            name: "Coffee".to_string(),
            quantity: 2,
            unit_price: 10.0,
        }],
        tendered: None,
    };

    let metadata = create_test_metadata();
    let events = action.execute(&mut ctx, &metadata).await.unwrap();

    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event_type, OrderEventType::ItemSplit);

    if let EventPayload::ItemSplit {
        split_amount,
        payment_method,
        items,
        ..
    } = &events[0].payload
    {
        assert_eq!(*split_amount, 20.0);
        assert_eq!(payment_method, "CASH");
        assert_eq!(items.len(), 1);
    } else {
        panic!("Expected ItemSplit payload");
    }
}

#[tokio::test]
async fn test_split_by_items_empty_fails() {
    let storage = OrderStorage::open_in_memory().unwrap();
    let txn = storage.begin_write().unwrap();

    let snapshot = create_active_order_with_items("order-1");
    storage.store_snapshot(&txn, &snapshot).unwrap();

    let current_seq = storage.get_next_sequence(&txn).unwrap();
    let mut ctx = CommandContext::new(&txn, &storage, current_seq);

    let action = SplitByItemsAction {
        order_id: "order-1".to_string(),
        payment_method: "CASH".to_string(),
        items: vec![],
        tendered: None,
    };

    let metadata = create_test_metadata();
    let result = action.execute(&mut ctx, &metadata).await;
    assert!(matches!(result, Err(OrderError::InvalidOperation(_))));
}

// ========== SplitByAmount tests ==========

#[tokio::test]
async fn test_split_by_amount_success() {
    let storage = OrderStorage::open_in_memory().unwrap();
    let txn = storage.begin_write().unwrap();

    let snapshot = create_active_order_with_items("order-1");
    storage.store_snapshot(&txn, &snapshot).unwrap();

    let current_seq = storage.get_next_sequence(&txn).unwrap();
    let mut ctx = CommandContext::new(&txn, &storage, current_seq);

    let action = SplitByAmountAction {
        order_id: "order-1".to_string(),
        split_amount: 20.0,
        payment_method: "CARD".to_string(),
        tendered: None,
    };

    let metadata = create_test_metadata();
    let events = action.execute(&mut ctx, &metadata).await.unwrap();

    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event_type, OrderEventType::AmountSplit);

    if let EventPayload::AmountSplit {
        split_amount,
        payment_method,
        ..
    } = &events[0].payload
    {
        assert_eq!(*split_amount, 20.0);
        assert_eq!(payment_method, "CARD");
    } else {
        panic!("Expected AmountSplit payload");
    }
}

#[tokio::test]
async fn test_split_by_amount_zero_fails() {
    let storage = OrderStorage::open_in_memory().unwrap();
    let txn = storage.begin_write().unwrap();

    let snapshot = create_active_order_with_items("order-1");
    storage.store_snapshot(&txn, &snapshot).unwrap();

    let current_seq = storage.get_next_sequence(&txn).unwrap();
    let mut ctx = CommandContext::new(&txn, &storage, current_seq);

    let action = SplitByAmountAction {
        order_id: "order-1".to_string(),
        split_amount: 0.0,
        payment_method: "CASH".to_string(),
        tendered: None,
    };

    let metadata = create_test_metadata();
    let result = action.execute(&mut ctx, &metadata).await;
    assert!(matches!(result, Err(OrderError::InvalidAmount)));
}

// ========== StartAASplit tests ==========

#[tokio::test]
async fn test_start_aa_split_success() {
    let storage = OrderStorage::open_in_memory().unwrap();
    let txn = storage.begin_write().unwrap();

    let snapshot = create_active_order_with_items("order-1");
    storage.store_snapshot(&txn, &snapshot).unwrap();

    let current_seq = storage.get_next_sequence(&txn).unwrap();
    let mut ctx = CommandContext::new(&txn, &storage, current_seq);

    let action = StartAaSplitAction {
        order_id: "order-1".to_string(),
        total_shares: 3,
        shares: 1,
        payment_method: "CASH".to_string(),
        tendered: None,
    };

    let metadata = create_test_metadata();
    let events = action.execute(&mut ctx, &metadata).await.unwrap();

    // Should produce 2 events: AaSplitStarted + AaSplitPaid
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].event_type, OrderEventType::AaSplitStarted);
    assert_eq!(events[1].event_type, OrderEventType::AaSplitPaid);

    if let EventPayload::AaSplitStarted {
        total_shares,
        order_total,
        ..
    } = &events[0].payload
    {
        assert_eq!(*total_shares, 3);
        assert_eq!(*order_total, 46.0);
    } else {
        panic!("Expected AaSplitStarted payload");
    }

    if let EventPayload::AaSplitPaid {
        shares,
        progress_paid,
        progress_total,
        ..
    } = &events[1].payload
    {
        assert_eq!(*shares, 1);
        assert_eq!(*progress_paid, 1);
        assert_eq!(*progress_total, 3);
    } else {
        panic!("Expected AaSplitPaid payload");
    }
}

#[tokio::test]
async fn test_start_aa_split_invalid_total_shares() {
    let storage = OrderStorage::open_in_memory().unwrap();
    let txn = storage.begin_write().unwrap();

    let snapshot = create_active_order_with_items("order-1");
    storage.store_snapshot(&txn, &snapshot).unwrap();

    let current_seq = storage.get_next_sequence(&txn).unwrap();
    let mut ctx = CommandContext::new(&txn, &storage, current_seq);

    let action = StartAaSplitAction {
        order_id: "order-1".to_string(),
        total_shares: 1, // Must be >= 2
        shares: 1,
        payment_method: "CASH".to_string(),
        tendered: None,
    };

    let metadata = create_test_metadata();
    let result = action.execute(&mut ctx, &metadata).await;
    assert!(matches!(result, Err(OrderError::InvalidOperation(_))));
}

// ========== PayAASplit tests ==========

#[tokio::test]
async fn test_pay_aa_split_success() {
    let storage = OrderStorage::open_in_memory().unwrap();
    let txn = storage.begin_write().unwrap();

    let mut snapshot = create_active_order_with_items("order-1");
    // Simulate AA already started
    snapshot.aa_total_shares = Some(3);
    snapshot.aa_paid_shares = 1;
    snapshot.paid_amount = 15.33; // ~46/3
    storage.store_snapshot(&txn, &snapshot).unwrap();

    let current_seq = storage.get_next_sequence(&txn).unwrap();
    let mut ctx = CommandContext::new(&txn, &storage, current_seq);

    let action = PayAaSplitAction {
        order_id: "order-1".to_string(),
        shares: 1,
        payment_method: "CARD".to_string(),
        tendered: None,
    };

    let metadata = create_test_metadata();
    let events = action.execute(&mut ctx, &metadata).await.unwrap();

    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event_type, OrderEventType::AaSplitPaid);

    if let EventPayload::AaSplitPaid {
        shares,
        progress_paid,
        progress_total,
        ..
    } = &events[0].payload
    {
        assert_eq!(*shares, 1);
        assert_eq!(*progress_paid, 2);
        assert_eq!(*progress_total, 3);
    } else {
        panic!("Expected AaSplitPaid payload");
    }
}

#[tokio::test]
async fn test_pay_aa_split_not_started_fails() {
    let storage = OrderStorage::open_in_memory().unwrap();
    let txn = storage.begin_write().unwrap();

    let snapshot = create_active_order_with_items("order-1");
    storage.store_snapshot(&txn, &snapshot).unwrap();

    let current_seq = storage.get_next_sequence(&txn).unwrap();
    let mut ctx = CommandContext::new(&txn, &storage, current_seq);

    let action = PayAaSplitAction {
        order_id: "order-1".to_string(),
        shares: 1,
        payment_method: "CASH".to_string(),
        tendered: None,
    };

    let metadata = create_test_metadata();
    let result = action.execute(&mut ctx, &metadata).await;
    assert!(matches!(result, Err(OrderError::InvalidOperation(_))));
}

// ========== Mutual exclusion tests ==========

#[tokio::test]
async fn test_item_split_then_amount_split_allowed() {
    let storage = OrderStorage::open_in_memory().unwrap();
    let txn = storage.begin_write().unwrap();

    let mut snapshot = create_active_order_with_items("order-1");
    // Simulate item split already happened: item-1 has 2 units paid
    snapshot
        .paid_item_quantities
        .insert("item-1".to_string(), 2);
    snapshot.paid_amount = 20.0;
    storage.store_snapshot(&txn, &snapshot).unwrap();

    let current_seq = storage.get_next_sequence(&txn).unwrap();
    let mut ctx = CommandContext::new(&txn, &storage, current_seq);

    // Amount split should be allowed after item split
    let action = SplitByAmountAction {
        order_id: "order-1".to_string(),
        split_amount: 10.0,
        payment_method: "CASH".to_string(),
        tendered: None,
    };

    let metadata = create_test_metadata();
    let result = action.execute(&mut ctx, &metadata).await;
    assert!(result.is_ok(), "Amount split should be allowed after item split");
}

#[tokio::test]
async fn test_item_split_then_aa_split_allowed() {
    let storage = OrderStorage::open_in_memory().unwrap();
    let txn = storage.begin_write().unwrap();

    let mut snapshot = create_active_order_with_items("order-1");
    // Simulate item split already happened
    snapshot
        .paid_item_quantities
        .insert("item-1".to_string(), 1);
    snapshot.paid_amount = 10.0;
    storage.store_snapshot(&txn, &snapshot).unwrap();

    let current_seq = storage.get_next_sequence(&txn).unwrap();
    let mut ctx = CommandContext::new(&txn, &storage, current_seq);

    // AA split should be allowed after item split
    let action = StartAaSplitAction {
        order_id: "order-1".to_string(),
        total_shares: 3,
        shares: 1,
        payment_method: "CASH".to_string(),
        tendered: None,
    };

    let metadata = create_test_metadata();
    let result = action.execute(&mut ctx, &metadata).await;
    assert!(result.is_ok(), "AA split should be allowed after item split");
}

#[tokio::test]
async fn test_amount_split_then_item_split_blocked() {
    let storage = OrderStorage::open_in_memory().unwrap();
    let txn = storage.begin_write().unwrap();

    let mut snapshot = create_active_order_with_items("order-1");
    snapshot.has_amount_split = true;
    snapshot.paid_amount = 10.0;
    storage.store_snapshot(&txn, &snapshot).unwrap();

    let current_seq = storage.get_next_sequence(&txn).unwrap();
    let mut ctx = CommandContext::new(&txn, &storage, current_seq);

    // Item split should be blocked after amount split
    let action = SplitByItemsAction {
        order_id: "order-1".to_string(),
        payment_method: "CASH".to_string(),
        items: vec![SplitItem {
            instance_id: "item-1".to_string(),
            name: "Coffee".to_string(),
            quantity: 1,
            unit_price: 10.0,
        }],
        tendered: None,
    };

    let metadata = create_test_metadata();
    let result = action.execute(&mut ctx, &metadata).await;
    assert!(
        matches!(result, Err(OrderError::InvalidOperation(_))),
        "Item split should be blocked after amount split"
    );
}

#[tokio::test]
async fn test_amount_split_then_aa_split_allowed() {
    let storage = OrderStorage::open_in_memory().unwrap();
    let txn = storage.begin_write().unwrap();

    let mut snapshot = create_active_order_with_items("order-1");
    snapshot.has_amount_split = true;
    snapshot.paid_amount = 10.0;
    storage.store_snapshot(&txn, &snapshot).unwrap();

    let current_seq = storage.get_next_sequence(&txn).unwrap();
    let mut ctx = CommandContext::new(&txn, &storage, current_seq);

    // AA split should be allowed after amount split
    let action = StartAaSplitAction {
        order_id: "order-1".to_string(),
        total_shares: 3,
        shares: 1,
        payment_method: "CASH".to_string(),
        tendered: None,
    };

    let metadata = create_test_metadata();
    let result = action.execute(&mut ctx, &metadata).await;
    assert!(result.is_ok(), "AA split should be allowed after amount split");
}

#[tokio::test]
async fn test_aa_active_blocks_item_split() {
    let storage = OrderStorage::open_in_memory().unwrap();
    let txn = storage.begin_write().unwrap();

    let mut snapshot = create_active_order_with_items("order-1");
    snapshot.aa_total_shares = Some(3);
    snapshot.aa_paid_shares = 1;
    snapshot.paid_amount = 15.33;
    storage.store_snapshot(&txn, &snapshot).unwrap();

    let current_seq = storage.get_next_sequence(&txn).unwrap();
    let mut ctx = CommandContext::new(&txn, &storage, current_seq);

    let action = SplitByItemsAction {
        order_id: "order-1".to_string(),
        payment_method: "CASH".to_string(),
        items: vec![SplitItem {
            instance_id: "item-1".to_string(),
            name: "Coffee".to_string(),
            quantity: 1,
            unit_price: 10.0,
        }],
        tendered: None,
    };

    let metadata = create_test_metadata();
    let result = action.execute(&mut ctx, &metadata).await;
    assert!(
        matches!(result, Err(OrderError::InvalidOperation(_))),
        "Item split should be blocked while AA is active"
    );
}

#[tokio::test]
async fn test_aa_active_blocks_amount_split() {
    let storage = OrderStorage::open_in_memory().unwrap();
    let txn = storage.begin_write().unwrap();

    let mut snapshot = create_active_order_with_items("order-1");
    snapshot.aa_total_shares = Some(3);
    snapshot.aa_paid_shares = 1;
    snapshot.paid_amount = 15.33;
    storage.store_snapshot(&txn, &snapshot).unwrap();

    let current_seq = storage.get_next_sequence(&txn).unwrap();
    let mut ctx = CommandContext::new(&txn, &storage, current_seq);

    let action = SplitByAmountAction {
        order_id: "order-1".to_string(),
        split_amount: 10.0,
        payment_method: "CASH".to_string(),
        tendered: None,
    };

    let metadata = create_test_metadata();
    let result = action.execute(&mut ctx, &metadata).await;
    assert!(
        matches!(result, Err(OrderError::InvalidOperation(_))),
        "Amount split should be blocked while AA is active"
    );
}

// ========== Remaining AA tests ==========

#[tokio::test]
async fn test_pay_aa_split_exceeds_remaining_fails() {
    let storage = OrderStorage::open_in_memory().unwrap();
    let txn = storage.begin_write().unwrap();

    let mut snapshot = create_active_order_with_items("order-1");
    snapshot.aa_total_shares = Some(3);
    snapshot.aa_paid_shares = 2; // Only 1 remaining
    snapshot.paid_amount = 30.67;
    storage.store_snapshot(&txn, &snapshot).unwrap();

    let current_seq = storage.get_next_sequence(&txn).unwrap();
    let mut ctx = CommandContext::new(&txn, &storage, current_seq);

    let action = PayAaSplitAction {
        order_id: "order-1".to_string(),
        shares: 2, // Only 1 available
        payment_method: "CASH".to_string(),
        tendered: None,
    };

    let metadata = create_test_metadata();
    let result = action.execute(&mut ctx, &metadata).await;
    assert!(matches!(result, Err(OrderError::InvalidOperation(_))));
}
