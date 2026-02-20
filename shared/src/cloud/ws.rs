//! WebSocket protocol types for edge-server ↔ crab-cloud duplex communication

use serde::{Deserialize, Serialize};

use super::{CloudCommand, CloudCommandResult, CloudSyncError, CloudSyncItem};

/// Duplex message protocol over WebSocket
///
/// Edge → Cloud: SyncBatch, CommandResult
/// Cloud → Edge: SyncAck, Command
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum CloudMessage {
    // === edge → cloud ===
    /// Batch of resource changes to sync
    SyncBatch {
        items: Vec<CloudSyncItem>,
        sent_at: i64,
        /// Results from previously executed commands (optional piggyback)
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        command_results: Vec<CloudCommandResult>,
    },

    /// Standalone command result delivery
    CommandResult { results: Vec<CloudCommandResult> },

    // === cloud → edge ===
    /// Acknowledgement of a SyncBatch
    SyncAck {
        accepted: u32,
        rejected: u32,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        errors: Vec<CloudSyncError>,
    },

    /// Command pushed from cloud to edge
    Command(CloudCommand),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cloud_message_sync_batch_roundtrip() {
        let msg = CloudMessage::SyncBatch {
            items: vec![CloudSyncItem {
                resource: "product".into(),
                version: 1,
                action: "upsert".into(),
                resource_id: "42".into(),
                data: serde_json::json!({"name": "Test"}),
            }],
            sent_at: 1700000000000,
            command_results: vec![],
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains(r#""type":"SyncBatch"#));

        let deserialized: CloudMessage = serde_json::from_str(&json).unwrap();
        match deserialized {
            CloudMessage::SyncBatch { items, sent_at, .. } => {
                assert_eq!(items.len(), 1);
                assert_eq!(sent_at, 1700000000000);
            }
            _ => panic!("Expected SyncBatch"),
        }
    }

    #[test]
    fn test_cloud_message_sync_ack_roundtrip() {
        let msg = CloudMessage::SyncAck {
            accepted: 5,
            rejected: 1,
            errors: vec![CloudSyncError {
                index: 3,
                resource_id: "99".into(),
                message: "Invalid data".into(),
            }],
        };

        let json = serde_json::to_string(&msg).unwrap();
        let deserialized: CloudMessage = serde_json::from_str(&json).unwrap();
        match deserialized {
            CloudMessage::SyncAck {
                accepted,
                rejected,
                errors,
            } => {
                assert_eq!(accepted, 5);
                assert_eq!(rejected, 1);
                assert_eq!(errors.len(), 1);
            }
            _ => panic!("Expected SyncAck"),
        }
    }

    #[test]
    fn test_cloud_message_command_roundtrip() {
        let msg = CloudMessage::Command(CloudCommand {
            id: "cmd-1".into(),
            command_type: "get_status".into(),
            payload: serde_json::json!({}),
            created_at: 1700000000000,
        });

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains(r#""type":"Command"#));

        let deserialized: CloudMessage = serde_json::from_str(&json).unwrap();
        match deserialized {
            CloudMessage::Command(cmd) => {
                assert_eq!(cmd.id, "cmd-1");
                assert_eq!(cmd.command_type, "get_status");
            }
            _ => panic!("Expected Command"),
        }
    }
}
