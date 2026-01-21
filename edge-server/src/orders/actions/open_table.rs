//! OpenTable command handler
//!
//! Creates a new order with table information.

use async_trait::async_trait;
use uuid::Uuid;

use crate::orders::traits::{CommandContext, CommandHandler, CommandMetadata, OrderError};
use shared::order::{EventPayload, OrderEvent, OrderEventType, OrderStatus};

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
