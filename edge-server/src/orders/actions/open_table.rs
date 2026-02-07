//! OpenTable command handler
//!
//! Creates a new order with table information.

use async_trait::async_trait;
use surrealdb::Surreal;
use surrealdb::engine::local::Db;
use tracing::{debug, info};
use uuid::Uuid;

use crate::db::models::PriceRule;
use crate::db::repository::PriceRuleRepository;
use crate::orders::traits::{CommandContext, CommandHandler, CommandMetadata, OrderError};
use shared::order::{EventPayload, OrderEvent, OrderEventType, OrderStatus};

/// 加载匹配区域的价格规则（静态缓存）
///
/// 区域是静态的（开台定格），时间是动态的（每次加菜实时检查）。
/// 此函数只做区域过滤，不做时间过滤。
///
/// DB 层过滤：
/// - zone_scope = "zone:all": 适用于所有区域
/// - zone_scope = "zone:retail": 适用于零售订单 (is_retail = true)
/// - zone_scope = "zone:xxx": 适用于特定区域 (zone_id 匹配)
/// - is_active = true: 规则必须是激活状态
///
/// # Arguments
/// * `db` - SurrealDB 数据库连接
/// * `zone_id` - 区域 ID (None 表示零售订单)
/// * `is_retail` - 是否为零售订单
///
/// # Returns
/// 返回区域匹配的活跃价格规则列表（不含时间过滤）
pub async fn load_matching_rules(
    db: &Surreal<Db>,
    zone_id: Option<&str>,
    is_retail: bool,
) -> Vec<PriceRule> {
    info!(
        zone_id = ?zone_id,
        is_retail,
        "[LoadRules] Loading zone-matched price rules"
    );

    let repo = PriceRuleRepository::new(db.clone());
    let rules = match repo.find_by_zone(zone_id, is_retail).await {
        Ok(rules) => rules,
        Err(e) => {
            tracing::error!("Failed to load price rules: {:?}", e);
            return vec![];
        }
    };

    info!(
        matched_rules_count = rules.len(),
        zone_id = ?zone_id,
        is_retail,
        "[LoadRules] Zone-matched rules loaded"
    );

    for rule in &rules {
        debug!(
            rule_name = %rule.name,
            rule_type = ?rule.rule_type,
            product_scope = ?rule.product_scope,
            zone_scope = rule.zone_scope,
            adjustment_type = ?rule.adjustment_type,
            adjustment_value = rule.adjustment_value,
            target = ?rule.target.as_ref().map(|t| t.to_string()),
            is_stackable = rule.is_stackable,
            is_exclusive = rule.is_exclusive,
            "[LoadRules] Matched rule detail"
        );
    }

    rules
}

/// OpenTable action
#[derive(Debug, Clone)]
pub struct OpenTableAction {
    pub table_id: Option<String>,
    pub table_name: Option<String>,
    pub zone_id: Option<String>,
    pub zone_name: Option<String>,
    pub guest_count: i32,
    pub is_retail: bool,
    /// 叫号（服务器预生成，零售订单使用）
    pub queue_number: Option<u32>,
    /// Server-generated receipt number
    pub receipt_number: String,
}

#[async_trait]
impl CommandHandler for OpenTableAction {
    async fn execute(
        &self,
        ctx: &mut CommandContext<'_>,
        metadata: &CommandMetadata,
    ) -> Result<Vec<OrderEvent>, OrderError> {
        info!(
            table_id = ?self.table_id,
            table_name = ?self.table_name,
            receipt_number = %self.receipt_number,
            "OpenTableAction::execute starting"
        );

        // 0. Validate table is not occupied (only for non-retail orders with table_id)
        if let Some(ref table_id) = self.table_id
            && let Some(existing_order_id) = ctx.find_active_order_for_table(table_id)?
        {
            let table_name = self.table_name.as_deref().unwrap_or(table_id);
            return Err(OrderError::TableOccupied(format!(
                "桌台 {} 已被占用 (订单: {})",
                table_name, existing_order_id
            )));
        }

        // 1. Generate new order ID
        let order_id = Uuid::new_v4().to_string();
        info!(order_id = %order_id, "Generated new order ID");

        // 2. Allocate sequence number
        let seq = ctx.next_sequence();

        // 3. Create snapshot with server-generated receipt_number
        let mut snapshot = ctx.create_snapshot(order_id.clone());
        snapshot.table_id = self.table_id.clone();
        snapshot.table_name = self.table_name.clone();
        snapshot.zone_id = self.zone_id.clone();
        snapshot.zone_name = self.zone_name.clone();
        snapshot.guest_count = self.guest_count;
        snapshot.is_retail = self.is_retail;
        snapshot.queue_number = self.queue_number;
        snapshot.receipt_number = self.receipt_number.clone();
        snapshot.status = OrderStatus::Active;
        snapshot.start_time = metadata.timestamp;
        snapshot.created_at = metadata.timestamp;
        snapshot.updated_at = metadata.timestamp;
        snapshot.last_sequence = seq;

        // 4. Update checksum
        snapshot.update_checksum();

        // 5. Save to context
        ctx.save_snapshot(snapshot);

        // 6. Create event with server-generated receipt_number
        let event = OrderEvent::new(
            seq,
            order_id.clone(),
            metadata.operator_id.clone(),
            metadata.operator_name.clone(),
            metadata.command_id.clone(),
            Some(metadata.timestamp), // Preserve client timestamp
            OrderEventType::TableOpened,
            EventPayload::TableOpened {
                table_id: self.table_id.clone(),
                table_name: self.table_name.clone(),
                zone_id: self.zone_id.clone(),
                zone_name: self.zone_name.clone(),
                guest_count: self.guest_count,
                is_retail: self.is_retail,
                queue_number: self.queue_number,
                receipt_number: self.receipt_number.clone(),
            },
        );

        info!(
            order_id = %order_id,
            seq = seq,
            receipt_number = %self.receipt_number,
            "OpenTableAction::execute completed"
        );
        Ok(vec![event])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orders::storage::OrderStorage;
    use crate::orders::traits::CommandContext;
    use shared::order::OrderSnapshot;

    fn create_test_metadata() -> CommandMetadata {
        CommandMetadata {
            command_id: "cmd-1".to_string(),
            operator_id: "user-1".to_string(),
            operator_name: "Test User".to_string(),
            timestamp: 1234567890,
        }
    }

    #[tokio::test]
    async fn test_open_table_success() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        let action = OpenTableAction {
            table_id: Some("T1".to_string()),
            table_name: Some("Table 1".to_string()),
            zone_id: Some("Z1".to_string()),
            zone_name: Some("Zone A".to_string()),
            guest_count: 4,
            is_retail: false,
            queue_number: None,
            receipt_number: "FAC2026012410001".to_string(),
        };

        let metadata = create_test_metadata();
        let events = action.execute(&mut ctx, &metadata).await.unwrap();

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, OrderEventType::TableOpened);
    }

    #[tokio::test]
    async fn test_open_table_occupied_fails() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        // Create an existing active order at table T1
        let mut existing = OrderSnapshot::new("existing-order".to_string());
        existing.status = OrderStatus::Active;
        existing.table_id = Some("T1".to_string());
        existing.table_name = Some("Table 1".to_string());
        storage.store_snapshot(&txn, &existing).unwrap();
        storage.mark_order_active(&txn, "existing-order").unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        // Try to open a new order at the same table
        let action = OpenTableAction {
            table_id: Some("T1".to_string()),
            table_name: Some("Table 1".to_string()),
            zone_id: None,
            zone_name: None,
            guest_count: 2,
            is_retail: false,
            queue_number: None,
            receipt_number: "FAC2026012410002".to_string(),
        };

        let metadata = create_test_metadata();
        let result = action.execute(&mut ctx, &metadata).await;

        assert!(matches!(result, Err(OrderError::TableOccupied(_))));
    }

    #[tokio::test]
    async fn test_open_retail_order_no_table_validation() {
        let storage = OrderStorage::open_in_memory().unwrap();
        let txn = storage.begin_write().unwrap();

        let current_seq = storage.get_next_sequence(&txn).unwrap();
        let mut ctx = CommandContext::new(&txn, &storage, current_seq);

        // Open a retail order (no table_id, service_type 在结单时设置)
        let action = OpenTableAction {
            table_id: None,
            table_name: Some("Retail".to_string()),
            zone_id: None,
            zone_name: None,
            guest_count: 1,
            is_retail: true,
            queue_number: Some(42),
            receipt_number: "FAC2026012410003".to_string(),
        };

        let metadata = create_test_metadata();
        let events = action.execute(&mut ctx, &metadata).await.unwrap();

        assert_eq!(events.len(), 1);
        if let EventPayload::TableOpened { is_retail, .. } = &events[0].payload {
            assert!(*is_retail);
        } else {
            panic!("Expected TableOpened payload");
        }
    }
}
