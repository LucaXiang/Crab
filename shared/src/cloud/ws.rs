//! WebSocket protocol types for edge-server ↔ crab-cloud duplex communication

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::order::{OrderEvent, OrderSnapshot};

use super::catalog::{CatalogOp, CatalogOpResult};
use super::{CloudCommand, CloudCommandResult, CloudSyncError, CloudSyncItem};

/// Duplex message protocol over WebSocket
///
/// Edge → Cloud: SyncBatch, CommandResult, RpcResult
/// Cloud → Edge: SyncAck, Command, Rpc
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

    /// Command pushed from cloud to edge (legacy string-based)
    Command(CloudCommand),

    // === cloud → edge (handshake) ===
    /// Cloud → Edge: 连接建立后发送，包含 cloud 已确认的各资源版本号
    Welcome { cursors: HashMap<String, u64> },

    // === 双向 RPC ===
    /// 强类型 RPC 请求（双向: cloud↔edge）
    /// id 用于 correlation + 幂等性去重
    Rpc { id: String, payload: Box<CloudRpc> },

    /// RPC 响应（与 Rpc.id 匹配）
    RpcResult { id: String, result: CloudRpcResult },

    // === edge → cloud: 活跃订单推送 ===
    /// 单个活跃订单快照更新（新建 or 变更）+ 事件历史
    ActiveOrderSnapshot {
        snapshot: Box<OrderSnapshot>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        events: Vec<OrderEvent>,
    },

    /// 活跃订单已移除（完成/作废/合并）
    ActiveOrderRemoved { order_id: String },
}

/// 强类型 RPC 载荷 — 替代 CloudCommand 的 string command_type + JSON payload
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum CloudRpc {
    // ── Cloud → Edge ──
    /// 查询 edge 状态
    GetStatus,
    /// 查询订单详情
    GetOrderDetail { order_key: String },
    /// 刷新订阅信息
    RefreshSubscription,
    /// Catalog 操作 (CRUD + FullSync)
    CatalogOp(Box<CatalogOp>),
    // ── Edge → Cloud (预留) ──
}

/// RPC 结果
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum CloudRpcResult {
    /// 通用 JSON 结果（GetStatus, GetOrderDetail, RefreshSubscription）
    Json {
        success: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        data: Option<serde_json::Value>,
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<String>,
    },
    /// Catalog 操作结果
    CatalogOp(Box<CatalogOpResult>),
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

    #[test]
    fn test_cloud_message_welcome_roundtrip() {
        let mut cursors = std::collections::HashMap::new();
        cursors.insert("product".to_string(), 42u64);
        cursors.insert("category".to_string(), 5u64);

        let msg = CloudMessage::Welcome { cursors };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains(r#""type":"Welcome"#));

        let deserialized: CloudMessage = serde_json::from_str(&json).unwrap();
        match deserialized {
            CloudMessage::Welcome { cursors } => {
                assert_eq!(cursors.get("product"), Some(&42));
                assert_eq!(cursors.get("category"), Some(&5));
            }
            _ => panic!("Expected Welcome"),
        }
    }

    #[test]
    fn test_rpc_catalog_op_roundtrip() {
        use crate::models::product::ProductSpecInput;

        let msg = CloudMessage::Rpc {
            id: "rpc-001".into(),
            payload: Box::new(CloudRpc::CatalogOp(Box::new(
                super::super::catalog::CatalogOp::CreateProduct {
                    data: crate::models::product::ProductCreate {
                        name: "Test".into(),
                        image: None,
                        category_id: 1,
                        sort_order: None,
                        tax_rate: Some(10),
                        receipt_name: None,
                        kitchen_print_name: None,
                        is_kitchen_print_enabled: None,
                        is_label_print_enabled: None,
                        external_id: None,
                        tags: None,
                        specs: vec![ProductSpecInput {
                            name: "默认".into(),
                            price: 5.0,
                            display_order: 0,
                            is_default: true,
                            is_active: true,
                            receipt_name: None,
                            is_root: true,
                        }],
                    },
                },
            ))),
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains(r#""type":"Rpc"#));
        assert!(json.contains(r#""kind":"CatalogOp"#));
        assert!(json.contains(r#""op":"CreateProduct"#));

        let deserialized: CloudMessage = serde_json::from_str(&json).unwrap();
        let CloudMessage::Rpc { id, payload } = deserialized else {
            panic!("Expected Rpc");
        };
        assert_eq!(id, "rpc-001");
        let CloudRpc::CatalogOp(op) = *payload else {
            panic!("Expected CatalogOp");
        };
        let super::super::catalog::CatalogOp::CreateProduct { data } = *op else {
            panic!("Expected CreateProduct");
        };
        assert_eq!(data.name, "Test");
    }

    #[test]
    fn test_rpc_result_catalog_roundtrip() {
        use super::super::catalog::CatalogOpResult;

        let msg = CloudMessage::RpcResult {
            id: "rpc-001".into(),
            result: CloudRpcResult::CatalogOp(Box::new(CatalogOpResult::created(42))),
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains(r#""type":"RpcResult"#));
        assert!(json.contains(r#""kind":"CatalogOp"#));

        let deserialized: CloudMessage = serde_json::from_str(&json).unwrap();
        match deserialized {
            CloudMessage::RpcResult { id, result } => {
                assert_eq!(id, "rpc-001");
                match result {
                    CloudRpcResult::CatalogOp(r) => {
                        assert!(r.success);
                        assert_eq!(r.created_id, Some(42));
                    }
                    _ => panic!("Expected CatalogOp result"),
                }
            }
            _ => panic!("Expected RpcResult"),
        }
    }
}
