//! Synchronization API for order reconnection
//!
//! This module provides the sync protocol for clients reconnecting after
//! a disconnect. It allows clients to catch up on missed events.
//!
//! # Protocol
//!
//! 1. Client reconnects with last known sequence
//! 2. Server calculates gap
//! 3. If gap is small, return incremental events
//! 4. If gap is large, return full sync with all active orders
//!
//! # Guarantees
//!
//! - Events are ordered by sequence
//! - No gaps in sequence (can be validated)
//! - Full sync is always available as fallback

use super::manager::{ManagerError, OrdersManager};
use shared::order::{OrderEvent, OrderSnapshot};
use serde::{Deserialize, Serialize};

/// Maximum events to return in incremental sync
/// If gap exceeds this, full sync is recommended
const MAX_INCREMENTAL_EVENTS: usize = 1000;

/// Sync request from client
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncRequest {
    /// Client's last known sequence number
    pub since_sequence: u64,
}

/// Sync response to client
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncResponse {
    /// Events since the requested sequence
    pub events: Vec<OrderEvent>,
    /// Current active order snapshots
    pub active_orders: Vec<OrderSnapshot>,
    /// Server's current sequence number
    pub server_sequence: u64,
    /// Whether full sync is required (gap too large)
    pub requires_full_sync: bool,
    /// Server instance epoch (UUID generated on startup)
    /// Used to detect server restarts - if epoch changes, client must full sync
    pub server_epoch: String,
}

impl SyncResponse {
    /// Create a full sync response
    pub fn full_sync(active_orders: Vec<OrderSnapshot>, server_sequence: u64, epoch: String) -> Self {
        Self {
            events: vec![],
            active_orders,
            server_sequence,
            requires_full_sync: true,
            server_epoch: epoch,
        }
    }

    /// Create an incremental sync response
    pub fn incremental(events: Vec<OrderEvent>, server_sequence: u64, epoch: String) -> Self {
        Self {
            events,
            active_orders: vec![],
            server_sequence,
            requires_full_sync: false,
            server_epoch: epoch,
        }
    }
}

/// Sync service for handling reconnection
pub struct SyncService {
    manager: OrdersManager,
}

impl SyncService {
    /// Create a new sync service
    pub fn new(manager: OrdersManager) -> Self {
        Self { manager }
    }

    /// Handle a sync request
    ///
    /// The response includes `server_epoch` which clients use to detect server restarts.
    /// If the epoch changes, clients must perform a full sync regardless of sequence gap.
    pub fn sync(&self, request: SyncRequest) -> Result<SyncResponse, ManagerError> {
        let server_sequence = self.manager.get_current_sequence()?;
        let epoch = self.manager.epoch().to_string();

        // If client is up to date, return empty response
        if request.since_sequence >= server_sequence {
            return Ok(SyncResponse::incremental(vec![], server_sequence, epoch));
        }

        // Calculate gap
        let gap = server_sequence - request.since_sequence;

        // If gap is large, recommend full sync
        if gap > MAX_INCREMENTAL_EVENTS as u64 {
            let active_orders = self.manager.get_active_orders()?;
            return Ok(SyncResponse::full_sync(active_orders, server_sequence, epoch));
        }

        // Return incremental events
        let events = self.manager.get_active_events_since(request.since_sequence)?;

        // Double-check: if we got too many events, fall back to full sync
        if events.len() > MAX_INCREMENTAL_EVENTS {
            let active_orders = self.manager.get_active_orders()?;
            return Ok(SyncResponse::full_sync(active_orders, server_sequence, epoch));
        }

        Ok(SyncResponse::incremental(events, server_sequence, epoch))
    }

    /// Get all active orders (for initial connection or full sync)
    pub fn get_all_active_orders(&self) -> Result<Vec<OrderSnapshot>, ManagerError> {
        self.manager.get_active_orders()
    }

    /// Get current server sequence
    pub fn get_server_sequence(&self) -> Result<u64, ManagerError> {
        self.manager.get_current_sequence()
    }

    /// Verify snapshot integrity by rebuilding from events
    pub fn verify_snapshot(&self, order_id: &str) -> Result<bool, ManagerError> {
        let stored = self.manager.get_snapshot(order_id)?;
        let rebuilt = self.manager.rebuild_snapshot(order_id)?;

        match (stored, rebuilt) {
            (Some(s), Some(r)) => {
                // Compare key fields
                let match_status = s.status == r.status;
                let match_items = s.items.len() == r.items.len();
                let match_total = (s.total - r.total).abs() < 0.01;
                let match_sequence = s.last_sequence == r.last_sequence;

                Ok(match_status && match_items && match_total && match_sequence)
            }
            (None, None) => Ok(true), // Both missing is consistent
            _ => Ok(false),           // One exists, one doesn't
        }
    }

    /// Verify all active order snapshots
    pub fn verify_all_snapshots(&self) -> Result<Vec<(String, bool)>, ManagerError> {
        let active_orders = self.manager.get_active_orders()?;
        let mut results = Vec::new();

        for order in active_orders {
            let is_valid = self.verify_snapshot(&order.order_id)?;
            results.push((order.order_id, is_valid));
        }

        Ok(results)
    }
}

/// Client-side sync state tracker
#[derive(Debug, Default)]
pub struct ClientSyncState {
    /// Last processed sequence
    pub last_sequence: u64,
    /// Whether we're connected
    pub connected: bool,
    /// Whether we need full sync
    pub needs_full_sync: bool,
}

impl ClientSyncState {
    /// Create a new client sync state
    pub fn new() -> Self {
        Self::default()
    }

    /// Record that we're connected
    pub fn on_connect(&mut self) {
        self.connected = true;
    }

    /// Record that we disconnected
    pub fn on_disconnect(&mut self) {
        self.connected = false;
    }

    /// Process a sync response
    pub fn on_sync_response(&mut self, response: &SyncResponse) {
        self.last_sequence = response.server_sequence;
        self.needs_full_sync = false;
    }

    /// Process an event
    pub fn on_event(&mut self, event: &OrderEvent) {
        // Check for gap
        if event.sequence > self.last_sequence + 1 {
            // Gap detected, need sync
            self.needs_full_sync = true;
        }
        self.last_sequence = event.sequence;
    }

    /// Check if we need to sync
    pub fn should_sync(&self) -> bool {
        !self.connected || self.needs_full_sync
    }

    /// Create a sync request
    pub fn create_sync_request(&self) -> SyncRequest {
        SyncRequest {
            since_sequence: self.last_sequence,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orders::storage::OrderStorage;
    use shared::order::OrderCommandPayload;

    fn create_test_manager() -> OrdersManager {
        let storage = OrderStorage::open_in_memory().unwrap();
        OrdersManager::with_storage(storage)
    }

    fn create_open_table_cmd(operator_id: &str) -> shared::order::OrderCommand {
        shared::order::OrderCommand::new(
            operator_id.to_string(),
            "Test Operator".to_string(),
            OrderCommandPayload::OpenTable {
                table_id: Some("T1".to_string()),
                table_name: Some("Table 1".to_string()),
                zone_id: None,
                zone_name: None,
                guest_count: 2,
                is_retail: false,
            },
        )
    }

    #[test]
    fn test_sync_empty() {
        let manager = create_test_manager();
        let sync_service = SyncService::new(manager);

        let request = SyncRequest { since_sequence: 0 };
        let response = sync_service.sync(request).unwrap();

        assert!(!response.requires_full_sync);
        assert!(response.events.is_empty());
        assert_eq!(response.server_sequence, 0);
    }

    #[test]
    fn test_sync_incremental() {
        let manager = create_test_manager();
        let sync_service = SyncService::new(manager.clone());

        // Create some orders
        let cmd1 = create_open_table_cmd("op-1");
        manager.execute_command(cmd1);

        let cmd2 = create_open_table_cmd("op-1");
        manager.execute_command(cmd2);

        // Sync from beginning
        let request = SyncRequest { since_sequence: 0 };
        let response = sync_service.sync(request).unwrap();

        assert!(!response.requires_full_sync);
        assert_eq!(response.events.len(), 2);
        assert_eq!(response.server_sequence, 2);

        // Sync from middle
        let request = SyncRequest { since_sequence: 1 };
        let response = sync_service.sync(request).unwrap();

        assert!(!response.requires_full_sync);
        assert_eq!(response.events.len(), 1);
    }

    #[test]
    fn test_sync_up_to_date() {
        let manager = create_test_manager();
        let sync_service = SyncService::new(manager.clone());

        // Create an order
        let cmd = create_open_table_cmd("op-1");
        manager.execute_command(cmd);

        // Sync with current sequence
        let request = SyncRequest { since_sequence: 1 };
        let response = sync_service.sync(request).unwrap();

        assert!(!response.requires_full_sync);
        assert!(response.events.is_empty());
        assert_eq!(response.server_sequence, 1);
    }

    #[test]
    fn test_verify_snapshot() {
        let manager = create_test_manager();
        let sync_service = SyncService::new(manager.clone());

        // Create an order
        let cmd = create_open_table_cmd("op-1");
        let response = manager.execute_command(cmd);
        let order_id = response.order_id.unwrap();

        // Verify snapshot
        let is_valid = sync_service.verify_snapshot(&order_id).unwrap();
        assert!(is_valid);
    }

    #[test]
    fn test_client_sync_state() {
        let mut state = ClientSyncState::new();

        assert_eq!(state.last_sequence, 0);
        assert!(!state.connected);

        state.on_connect();
        assert!(state.connected);

        // Simulate receiving events
        let event1 = shared::order::OrderEvent {
            event_id: "e1".to_string(),
            sequence: 1,
            order_id: "o1".to_string(),
            timestamp: 0,
            client_timestamp: None,
            operator_id: "op".to_string(),
            operator_name: "Op".to_string(),
            command_id: "c1".to_string(),
            event_type: shared::order::OrderEventType::TableOpened,
            payload: shared::order::EventPayload::TableOpened {
                table_id: None,
                table_name: None,
                zone_id: None,
                zone_name: None,
                guest_count: 1,
                is_retail: false,
                surcharge: None,
                receipt_number: None,
            },
        };

        state.on_event(&event1);
        assert_eq!(state.last_sequence, 1);
        assert!(!state.needs_full_sync);

        // Gap detected
        let event3 = shared::order::OrderEvent {
            event_id: "e3".to_string(),
            sequence: 3, // Gap from 1 to 3
            order_id: "o1".to_string(),
            timestamp: 0,
            client_timestamp: None,
            operator_id: "op".to_string(),
            operator_name: "Op".to_string(),
            command_id: "c3".to_string(),
            event_type: shared::order::OrderEventType::TableOpened,
            payload: shared::order::EventPayload::TableOpened {
                table_id: None,
                table_name: None,
                zone_id: None,
                zone_name: None,
                guest_count: 1,
                is_retail: false,
                surcharge: None,
                receipt_number: None,
            },
        };

        state.on_event(&event3);
        assert!(state.needs_full_sync);
    }
}
