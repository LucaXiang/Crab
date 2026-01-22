//! OpenTable command handler
//!
//! Creates a new order with table information.

use async_trait::async_trait;
use surrealdb::Surreal;
use surrealdb::engine::local::Db;
use uuid::Uuid;

use crate::db::models::PriceRule;
use crate::db::repository::PriceRuleRepository;
use crate::orders::traits::{CommandContext, CommandHandler, CommandMetadata, OrderError};
use shared::order::{EventPayload, OrderEvent, OrderEventType, OrderStatus};

/// 加载匹配的价格规则
///
/// 根据订单的 zone 信息加载适用的价格规则：
/// - zone_scope = -1: 适用于所有区域
/// - zone_scope = 0: 适用于零售订单 (is_retail = true)
/// - zone_scope > 0: 适用于特定区域 (zone_id 匹配)
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
    let repo = PriceRuleRepository::new(db.clone());
    let all_rules = match repo.find_all().await {
        Ok(rules) => rules,
        Err(e) => {
            tracing::error!("加载价格规则失败: {:?}", e);
            return vec![];
        }
    };

    // 根据 zone_scope 过滤规则
    all_rules
        .into_iter()
        .filter(|r| {
            // zone_scope = -1: 适用于所有区域
            if r.zone_scope == -1 {
                return true;
            }
            // zone_scope = 0: 仅适用于零售订单
            if r.zone_scope == 0 && is_retail {
                return true;
            }
            // zone_scope > 0: 匹配特定区域 ID
            if let Some(zid) = zone_id {
                // zone_scope 存储的可能是数字形式的 zone ID
                return r.zone_scope.to_string() == zid;
            }
            false
        })
        .collect()
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
}

#[async_trait]
impl CommandHandler for OpenTableAction {
    async fn execute(
        &self,
        ctx: &mut CommandContext<'_>,
        metadata: &CommandMetadata,
    ) -> Result<Vec<OrderEvent>, OrderError> {
        // 1. Generate new order ID
        let order_id = Uuid::new_v4().to_string();

        // 2. Allocate sequence number
        let seq = ctx.next_sequence();

        // 3. Create snapshot
        let mut snapshot = ctx.create_snapshot(order_id.clone());
        snapshot.table_id = self.table_id.clone();
        snapshot.table_name = self.table_name.clone();
        snapshot.zone_id = self.zone_id.clone();
        snapshot.zone_name = self.zone_name.clone();
        snapshot.guest_count = self.guest_count;
        snapshot.is_retail = self.is_retail;
        snapshot.status = OrderStatus::Active;
        snapshot.start_time = metadata.timestamp;
        snapshot.created_at = metadata.timestamp;
        snapshot.updated_at = metadata.timestamp;
        snapshot.last_sequence = seq;

        // 4. Update checksum
        snapshot.update_checksum();

        // 5. Save to context
        ctx.save_snapshot(snapshot);

        // 6. Create event
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
                receipt_number: None,
            },
        );

        Ok(vec![event])
    }
}
