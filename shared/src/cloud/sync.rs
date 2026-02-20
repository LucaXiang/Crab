//! Cloud sync batch protocol types
//!
//! Used by edge-server to push data to crab-cloud,
//! and by crab-cloud to receive and store synced data.

use serde::{Deserialize, Serialize};

/// A batch of sync items from an edge-server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudSyncBatch {
    /// Edge server entity_id (from SignedBinding)
    pub edge_id: String,
    /// Sync items in this batch
    pub items: Vec<CloudSyncItem>,
    /// Timestamp when the batch was sent (Unix millis)
    pub sent_at: i64,
    /// Results from previously executed commands
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub command_results: Vec<CloudCommandResult>,
}

/// A single resource change to sync to the cloud
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudSyncItem {
    /// Resource type: "product", "category", "archived_order",
    /// "daily_report", "store_info"
    pub resource: String,
    /// Monotonically increasing version for this resource on this edge
    pub version: u64,
    /// Action: "upsert" or "delete"
    pub action: String,
    /// Resource ID (source ID on the edge-server)
    pub resource_id: String,
    /// Full resource data as JSON
    pub data: serde_json::Value,
}

/// Response from crab-cloud after processing a sync batch
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudSyncResponse {
    /// Number of items accepted
    pub accepted: u32,
    /// Number of items rejected
    pub rejected: u32,
    /// Errors for rejected items
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<CloudSyncError>,
    /// Pending commands for the edge-server (future use)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pending_commands: Vec<CloudCommand>,
}

/// Error detail for a rejected sync item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudSyncError {
    /// Index of the item in the batch
    pub index: u32,
    /// Resource ID that failed
    pub resource_id: String,
    /// Error message
    pub message: String,
}

/// A command from cloud to edge-server (future use)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudCommand {
    /// Command ID
    pub id: String,
    /// Command type
    pub command_type: String,
    /// Command payload
    pub payload: serde_json::Value,
    /// Created at (Unix millis)
    pub created_at: i64,
}

/// Result of executing a cloud command on edge-server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudCommandResult {
    /// Command ID
    pub command_id: String,
    /// Whether the command succeeded
    pub success: bool,
    /// Result data if succeeded
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
    /// Error message if failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Executed at (Unix millis)
    pub executed_at: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cloud_sync_batch_serialization() {
        let batch = CloudSyncBatch {
            edge_id: "edge-001".to_string(),
            items: vec![CloudSyncItem {
                resource: "product".to_string(),
                version: 1,
                action: "upsert".to_string(),
                resource_id: "42".to_string(),
                data: serde_json::json!({"name": "Test Product", "price": 9.99}),
            }],
            sent_at: 1700000000000,
            command_results: vec![],
        };

        let json = serde_json::to_string(&batch).unwrap();
        let deserialized: CloudSyncBatch = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.edge_id, "edge-001");
        assert_eq!(deserialized.items.len(), 1);
        assert_eq!(deserialized.items[0].resource, "product");
        assert_eq!(deserialized.items[0].version, 1);
    }

    #[test]
    fn test_cloud_sync_response_serialization() {
        let response = CloudSyncResponse {
            accepted: 5,
            rejected: 1,
            errors: vec![CloudSyncError {
                index: 3,
                resource_id: "99".to_string(),
                message: "Invalid data".to_string(),
            }],
            pending_commands: vec![],
        };

        let json = serde_json::to_string(&response).unwrap();
        let deserialized: CloudSyncResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.accepted, 5);
        assert_eq!(deserialized.rejected, 1);
        assert_eq!(deserialized.errors.len(), 1);
    }

    #[test]
    fn test_empty_response_skips_optional_fields() {
        let response = CloudSyncResponse {
            accepted: 10,
            rejected: 0,
            errors: vec![],
            pending_commands: vec![],
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(!json.contains("errors"));
        assert!(!json.contains("pending_commands"));
    }
}
