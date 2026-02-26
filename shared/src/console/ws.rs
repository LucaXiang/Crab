//! Console WebSocket protocol
//!
//! Cloud → Console: ConsoleMessage (推送)
//! Console → Cloud: ConsoleCommand (订阅控制 + 未来扩展)

use serde::{Deserialize, Serialize};

use crate::order::{OrderEvent, OrderSnapshot};

/// Cloud → Console 推送消息
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ConsoleMessage {
    /// 握手完成，携带当前全量活跃订单 + 在线 edge 列表
    Ready {
        snapshots: Vec<LiveOrderSnapshot>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        online_edge_ids: Vec<i64>,
    },

    /// 单个订单更新（新建 or 变更）
    OrderUpdated { snapshot: Box<LiveOrderSnapshot> },

    /// 订单已移除（完成/作废/合并）
    OrderRemoved { order_id: String, store_id: i64 },

    /// Edge 上线/下线通知
    EdgeStatus {
        store_id: i64,
        online: bool,
        /// Edge 离线时被清除的订单 ID（console 应移除这些订单）
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        cleared_order_ids: Vec<String>,
    },
}

/// Console → Cloud 命令
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ConsoleCommand {
    /// 订阅指定门店（空列表 = 订阅 tenant 下全部门店）
    Subscribe { store_ids: Vec<i64> },
}

/// 活跃订单快照 + 事件历史 + 来源信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveOrderSnapshot {
    pub store_id: i64,
    #[serde(flatten)]
    pub order: OrderSnapshot,
    /// 该订单的完整事件历史（event sourcing）
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub events: Vec<OrderEvent>,
}
