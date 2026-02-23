//! LiveOrderHub — 活跃订单实时分发
//!
//! 管理 edge → cloud → console 的活跃订单推送。
//! 所有数据按 tenant 严格隔离。
//!
//! ```text
//! Edge WS handler
//!       │ ActiveOrderSnapshot / ActiveOrderRemoved
//!       ▼
//! LiveOrderHub
//!   ├── tenants: 按 tenant 隔离的缓存 + broadcast
//!   │     ├── snapshots: edge_server_id → (order_id → LiveOrderSnapshot)
//!   │     └── broadcast: Sender<LiveHubEvent> (fan-out 到多个 console)
//!   │           │
//!   │           ▼
//!   └── Console WS handler (subscribe → 过滤 → 推送)
//! ```

use dashmap::DashMap;
use dashmap::DashSet;
use shared::console::LiveOrderSnapshot;
use std::sync::Arc;
use tokio::sync::broadcast;

/// Hub 内部事件
#[derive(Debug, Clone)]
pub enum LiveHubEvent {
    OrderUpdated(Box<LiveOrderSnapshot>),
    OrderRemoved {
        order_id: String,
        edge_server_id: i64,
    },
    EdgeOnline {
        edge_server_id: i64,
    },
    EdgeOffline {
        edge_server_id: i64,
        cleared_order_ids: Vec<String>,
    },
}

/// Broadcast channel 容量 — 足以缓冲连接时突发
const BROADCAST_CAPACITY: usize = 256;

/// 单个 tenant 的活跃订单数据
struct TenantLive {
    /// edge_server_id → (order_id → LiveOrderSnapshot)
    snapshots: DashMap<i64, DashMap<String, LiveOrderSnapshot>>,
    /// 当前在线的 edge server ID
    online_edges: DashSet<i64>,
    /// 广播给该 tenant 的所有 console 订阅者
    tx: broadcast::Sender<LiveHubEvent>,
}

impl TenantLive {
    fn new() -> Self {
        let (tx, _) = broadcast::channel(BROADCAST_CAPACITY);
        Self {
            snapshots: DashMap::new(),
            online_edges: DashSet::new(),
            tx,
        }
    }
}

/// 全局活跃订单 hub — 按 tenant 严格隔离
#[derive(Clone, Default)]
pub struct LiveOrderHub {
    /// tenant_id → TenantLive
    tenants: Arc<DashMap<String, TenantLive>>,
}

impl LiveOrderHub {
    pub fn new() -> Self {
        Self::default()
    }

    /// Edge 上线（WS 连接建立时调用）
    pub fn mark_edge_online(&self, tenant_id: &str, edge_server_id: i64) {
        let tenant = self.get_or_create_tenant(tenant_id);
        tenant.online_edges.insert(edge_server_id);
        let _ = tenant.tx.send(LiveHubEvent::EdgeOnline { edge_server_id });
    }

    /// 查询指定 edge 列表中哪些在线
    ///
    /// `edge_server_ids` 为空时返回该 tenant 下所有在线 edge
    pub fn get_online_edges(&self, tenant_id: &str, edge_server_ids: &[i64]) -> Vec<i64> {
        let tenant = match self.tenants.get(tenant_id) {
            Some(t) => t,
            None => return Vec::new(),
        };

        if edge_server_ids.is_empty() {
            tenant.online_edges.iter().map(|e| *e).collect()
        } else {
            edge_server_ids
                .iter()
                .filter(|id| tenant.online_edges.contains(id))
                .copied()
                .collect()
        }
    }

    /// Edge 推送订单快照更新
    pub fn publish_update(&self, tenant_id: &str, snapshot: LiveOrderSnapshot) {
        let tenant = self.get_or_create_tenant(tenant_id);
        let edge_id = snapshot.edge_server_id;
        let order_id = snapshot.order.order_id.clone();

        // 更新缓存
        tenant
            .snapshots
            .entry(edge_id)
            .or_default()
            .insert(order_id, snapshot.clone());

        // 广播（无订阅者时 send 返回 Err，安全忽略）
        let _ = tenant
            .tx
            .send(LiveHubEvent::OrderUpdated(Box::new(snapshot)));
    }

    /// Edge 推送订单移除
    pub fn publish_remove(&self, tenant_id: &str, order_id: &str, edge_server_id: i64) {
        if let Some(tenant) = self.tenants.get(tenant_id) {
            // 从缓存移除
            if let Some(orders) = tenant.snapshots.get(&edge_server_id) {
                orders.remove(order_id);
            }

            // 广播
            let _ = tenant.tx.send(LiveHubEvent::OrderRemoved {
                order_id: order_id.to_string(),
                edge_server_id,
            });
        }
    }

    /// Edge 断线，清理该 edge 的缓存并通知 console
    pub fn clear_edge(&self, tenant_id: &str, edge_server_id: i64) {
        if let Some(tenant) = self.tenants.get(tenant_id) {
            // 标记下线
            tenant.online_edges.remove(&edge_server_id);

            // 收集被清除的 order_id（供 console 清理残留）
            let cleared_order_ids = tenant
                .snapshots
                .get(&edge_server_id)
                .map(|orders| orders.iter().map(|e| e.key().clone()).collect::<Vec<_>>())
                .unwrap_or_default();

            tenant.snapshots.remove(&edge_server_id);
            let _ = tenant.tx.send(LiveHubEvent::EdgeOffline {
                edge_server_id,
                cleared_order_ids,
            });

            // 无活跃 edge 且无 console 订阅者时，清理 tenant 条目
            if tenant.online_edges.is_empty()
                && tenant.snapshots.is_empty()
                && tenant.tx.receiver_count() == 0
            {
                drop(tenant);
                self.tenants.remove(tenant_id);
            }
        }
    }

    /// 获取 tenant 下指定 edge 的活跃订单（新 console 连接初始化用）
    ///
    /// `edge_server_ids` 为空时返回该 tenant 下所有 edge 的订单
    pub fn get_all_active(
        &self,
        tenant_id: &str,
        edge_server_ids: &[i64],
    ) -> Vec<LiveOrderSnapshot> {
        let tenant = match self.tenants.get(tenant_id) {
            Some(t) => t,
            None => return Vec::new(),
        };

        let mut result = Vec::new();

        if edge_server_ids.is_empty() {
            for entry in tenant.snapshots.iter() {
                for order in entry.value().iter() {
                    result.push(order.value().clone());
                }
            }
        } else {
            for &eid in edge_server_ids {
                if let Some(orders) = tenant.snapshots.get(&eid) {
                    for order in orders.iter() {
                        result.push(order.value().clone());
                    }
                }
            }
        }

        result
    }

    /// 订阅 tenant 的 broadcast channel
    pub fn subscribe(&self, tenant_id: &str) -> broadcast::Receiver<LiveHubEvent> {
        let tenant = self.get_or_create_tenant(tenant_id);
        tenant.tx.subscribe()
    }

    fn get_or_create_tenant(
        &self,
        tenant_id: &str,
    ) -> dashmap::mapref::one::Ref<'_, String, TenantLive> {
        self.tenants
            .entry(tenant_id.to_string())
            .or_insert_with(TenantLive::new)
            .downgrade()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::order::OrderSnapshot;

    fn make_snapshot(edge_server_id: i64, order_id: &str) -> LiveOrderSnapshot {
        LiveOrderSnapshot {
            edge_server_id,
            order: OrderSnapshot::new(order_id.to_string()),
            events: vec![],
        }
    }

    #[test]
    fn basic_publish_get_remove() {
        let hub = LiveOrderHub::new();

        hub.publish_update("t1", make_snapshot(1, "order-a"));
        hub.publish_update("t1", make_snapshot(1, "order-b"));

        let all = hub.get_all_active("t1", &[]);
        assert_eq!(all.len(), 2);

        let filtered = hub.get_all_active("t1", &[1]);
        assert_eq!(filtered.len(), 2);

        let empty = hub.get_all_active("t1", &[999]);
        assert!(empty.is_empty());

        hub.publish_remove("t1", "order-a", 1);
        let after = hub.get_all_active("t1", &[]);
        assert_eq!(after.len(), 1);
        assert_eq!(after[0].order.order_id, "order-b");

        hub.publish_remove("t1", "order-b", 1);
        assert!(hub.get_all_active("t1", &[]).is_empty());
    }

    #[test]
    fn tenant_isolation() {
        let hub = LiveOrderHub::new();

        hub.publish_update("tenant-a", make_snapshot(1, "order-1"));
        hub.publish_update("tenant-b", make_snapshot(2, "order-2"));

        let a = hub.get_all_active("tenant-a", &[]);
        assert_eq!(a.len(), 1);
        assert_eq!(a[0].order.order_id, "order-1");

        let b = hub.get_all_active("tenant-b", &[]);
        assert_eq!(b.len(), 1);
        assert_eq!(b[0].order.order_id, "order-2");

        assert!(hub.get_all_active("tenant-x", &[]).is_empty());
    }

    #[test]
    fn store_isolation_via_edge_filter() {
        let hub = LiveOrderHub::new();

        hub.publish_update("t1", make_snapshot(10, "s10-a"));
        hub.publish_update("t1", make_snapshot(10, "s10-b"));
        hub.publish_update("t1", make_snapshot(20, "s20-c"));

        let store10 = hub.get_all_active("t1", &[10]);
        assert_eq!(store10.len(), 2);
        assert!(store10.iter().all(|s| s.edge_server_id == 10));

        let store20 = hub.get_all_active("t1", &[20]);
        assert_eq!(store20.len(), 1);
        assert_eq!(store20[0].order.order_id, "s20-c");

        assert_eq!(hub.get_all_active("t1", &[]).len(), 3);
    }

    #[tokio::test]
    async fn clear_edge_broadcasts_offline_with_order_ids() {
        let hub = LiveOrderHub::new();
        let mut rx = hub.subscribe("t1");

        hub.publish_update("t1", make_snapshot(1, "ox"));
        hub.publish_update("t1", make_snapshot(1, "oy"));

        // 消费 OrderUpdated
        let _ = rx.recv().await.unwrap();
        let _ = rx.recv().await.unwrap();

        hub.clear_edge("t1", 1);
        assert!(hub.get_all_active("t1", &[1]).is_empty());

        match rx.recv().await.unwrap() {
            LiveHubEvent::EdgeOffline {
                edge_server_id,
                cleared_order_ids,
            } => {
                assert_eq!(edge_server_id, 1);
                assert_eq!(cleared_order_ids.len(), 2);
                assert!(cleared_order_ids.contains(&"ox".to_string()));
                assert!(cleared_order_ids.contains(&"oy".to_string()));
            }
            other => panic!("Expected EdgeOffline, got {other:?}"),
        }
    }

    #[test]
    fn tenant_auto_cleanup_when_empty() {
        let hub = LiveOrderHub::new();
        hub.publish_update("t-temp", make_snapshot(1, "o1"));
        assert_eq!(hub.get_all_active("t-temp", &[]).len(), 1);

        // clear_edge 后无 subscriber + 无 edge → 自动清理
        hub.clear_edge("t-temp", 1);
        assert!(hub.get_all_active("t-temp", &[]).is_empty());
    }

    #[tokio::test]
    async fn subscribe_receives_updates_and_removals() {
        let hub = LiveOrderHub::new();
        let mut rx = hub.subscribe("t1");

        hub.publish_update("t1", make_snapshot(5, "o1"));
        match rx.recv().await.unwrap() {
            LiveHubEvent::OrderUpdated(snap) => {
                assert_eq!(snap.edge_server_id, 5);
                assert_eq!(snap.order.order_id, "o1");
            }
            other => panic!("Expected OrderUpdated, got {other:?}"),
        }

        hub.publish_remove("t1", "o1", 5);
        match rx.recv().await.unwrap() {
            LiveHubEvent::OrderRemoved {
                order_id,
                edge_server_id,
            } => {
                assert_eq!(order_id, "o1");
                assert_eq!(edge_server_id, 5);
            }
            other => panic!("Expected OrderRemoved, got {other:?}"),
        }
    }
}
