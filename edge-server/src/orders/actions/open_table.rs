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
use crate::pricing::matcher::is_time_valid;
use shared::order::{EventPayload, OrderEvent, OrderEventType, OrderStatus};

/// 加载匹配的价格规则
///
/// 根据订单的 zone 信息加载适用的价格规则：
/// - zone_scope = "zone:all": 适用于所有区域
/// - zone_scope = "zone:retail": 适用于零售订单 (is_retail = true)
/// - zone_scope = "zone:xxx": 适用于特定区域 (zone_id 匹配)
///
/// 同时过滤：
/// - is_active = true: 规则必须是激活状态
/// - 时间有效性: 规则必须在当前时间有效
///
/// # Arguments
/// * `db` - SurrealDB 数据库连接
/// * `zone_id` - 区域 ID (None 表示零售订单)
/// * `is_retail` - 是否为零售订单
///
/// # Returns
/// 返回匹配的活跃价格规则列表
pub async fn load_matching_rules(
    db: &Surreal<Db>,
    zone_id: Option<&str>,
    is_retail: bool,
) -> Vec<PriceRule> {
    info!(
        zone_id = ?zone_id,
        is_retail,
        "[LoadRules] Loading matching price rules"
    );

    let repo = PriceRuleRepository::new(db.clone());
    let all_rules = match repo.find_all().await {
        Ok(rules) => rules,
        Err(e) => {
            tracing::error!("加载价格规则失败: {:?}", e);
            return vec![];
        }
    };

    info!(
        total_rules = all_rules.len(),
        "[LoadRules] Loaded all rules from database"
    );

    let current_time = chrono::Utc::now().timestamp_millis();

    // 过滤规则: is_active + zone_scope + 时间有效性
    let matched_rules: Vec<PriceRule> = all_rules
        .into_iter()
        .filter(|r| {
            let rule_name = &r.name;

            // 必须是激活状态
            if !r.is_active {
                debug!(
                    rule_name = %rule_name,
                    "[LoadRules] Rule filtered: not active"
                );
                return false;
            }

            // 检查时间有效性
            if !is_time_valid(r, current_time) {
                debug!(
                    rule_name = %rule_name,
                    valid_from = ?r.valid_from,
                    valid_until = ?r.valid_until,
                    active_days = ?r.active_days,
                    active_start_time = ?r.active_start_time,
                    active_end_time = ?r.active_end_time,
                    current_time,
                    "[LoadRules] Rule filtered: time invalid"
                );
                return false;
            }

            // zone_scope = "zone:all": 适用于所有区域
            if r.zone_scope == crate::db::models::ZONE_SCOPE_ALL {
                debug!(
                    rule_name = %rule_name,
                    zone_scope = %r.zone_scope,
                    "[LoadRules] Rule matched: zone_scope=all"
                );
                return true;
            }
            // zone_scope = "zone:retail": 仅适用于零售订单
            if r.zone_scope == crate::db::models::ZONE_SCOPE_RETAIL && is_retail {
                debug!(
                    rule_name = %rule_name,
                    zone_scope = %r.zone_scope,
                    "[LoadRules] Rule matched: zone_scope=retail"
                );
                return true;
            }
            // zone_scope = "zone:xxx": 匹配特定区域 ID
            // zone_id 格式规范: "zone:xxx" (与 zone_scope 格式一致)
            if let Some(zid) = zone_id {
                let matches = r.zone_scope == zid;
                if matches {
                    debug!(
                        rule_name = %rule_name,
                        zone_scope = r.zone_scope,
                        zone_id = %zid,
                        "[LoadRules] Rule matched: specific zone"
                    );
                } else {
                    debug!(
                        rule_name = %rule_name,
                        zone_scope = r.zone_scope,
                        zone_id = %zid,
                        "[LoadRules] Rule filtered: zone mismatch"
                    );
                }
                return matches;
            }

            debug!(
                rule_name = %rule_name,
                zone_scope = r.zone_scope,
                is_retail,
                zone_id = ?zone_id,
                "[LoadRules] Rule filtered: no zone match"
            );
            false
        })
        .collect();

    info!(
        matched_rules_count = matched_rules.len(),
        zone_id = ?zone_id,
        is_retail,
        "[LoadRules] Final matched rules"
    );

    for rule in &matched_rules {
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

    matched_rules
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

        // Open a retail order (no table_id)
        let action = OpenTableAction {
            table_id: None,
            table_name: Some("Retail".to_string()),
            zone_id: None,
            zone_name: None,
            guest_count: 1,
            is_retail: true,
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
