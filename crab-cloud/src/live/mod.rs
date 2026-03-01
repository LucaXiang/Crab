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
//!   │     ├── snapshots: store_id → (order_id → LiveOrderSnapshot)
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
        order_id: i64,
        store_id: i64,
    },
    EdgeOnline {
        store_id: i64,
    },
    EdgeOffline {
        store_id: i64,
        cleared_order_ids: Vec<i64>,
    },
    StoreInfoUpdated {
        store_id: i64,
        info: Box<shared::models::store_info::StoreInfo>,
    },
}

/// Broadcast channel 容量 — 足以缓冲连接时突发
const BROADCAST_CAPACITY: usize = 256;

/// 单个 tenant 的活跃订单数据
struct TenantLive {
    /// store_id → (order_id → LiveOrderSnapshot)
    snapshots: DashMap<i64, DashMap<i64, LiveOrderSnapshot>>,
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
    tenants: Arc<DashMap<i64, TenantLive>>,
}

impl LiveOrderHub {
    pub fn new() -> Self {
        Self::default()
    }

    /// Edge 上线（WS 连接建立时调用）
    pub fn mark_edge_online(&self, tenant_id: i64, store_id: i64) {
        let tenant = self.get_or_create_tenant(tenant_id);
        tenant.online_edges.insert(store_id);
        let _ = tenant.tx.send(LiveHubEvent::EdgeOnline { store_id });
    }

    /// 查询指定 edge 列表中哪些在线
    ///
    /// `store_ids` 为空时返回该 tenant 下所有在线 edge
    pub fn get_online_edges(&self, tenant_id: i64, store_ids: &[i64]) -> Vec<i64> {
        let tenant = match self.tenants.get(&tenant_id) {
            Some(t) => t,
            None => return Vec::new(),
        };

        if store_ids.is_empty() {
            tenant.online_edges.iter().map(|e| *e).collect()
        } else {
            store_ids
                .iter()
                .filter(|id| tenant.online_edges.contains(id))
                .copied()
                .collect()
        }
    }

    /// Edge 推送订单快照更新
    pub fn publish_update(&self, tenant_id: i64, snapshot: LiveOrderSnapshot) {
        let tenant = self.get_or_create_tenant(tenant_id);
        let edge_id = snapshot.store_id;
        let order_id = snapshot.order.order_id;

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
    pub fn publish_remove(&self, tenant_id: i64, order_id: i64, store_id: i64) {
        if let Some(tenant) = self.tenants.get(&tenant_id) {
            // 从缓存移除
            if let Some(orders) = tenant.snapshots.get(&store_id) {
                orders.remove(&order_id);
            }

            // 广播
            let _ = tenant
                .tx
                .send(LiveHubEvent::OrderRemoved { order_id, store_id });
        }
    }

    /// Edge 断线，清理该 edge 的缓存并通知 console
    pub fn clear_edge(&self, tenant_id: i64, store_id: i64) {
        if let Some(tenant) = self.tenants.get(&tenant_id) {
            // 标记下线
            tenant.online_edges.remove(&store_id);

            // 收集被清除的 order_id（供 console 清理残留）
            let cleared_order_ids = tenant
                .snapshots
                .get(&store_id)
                .map(|orders| orders.iter().map(|e| *e.key()).collect::<Vec<_>>())
                .unwrap_or_default();

            tenant.snapshots.remove(&store_id);
            let _ = tenant.tx.send(LiveHubEvent::EdgeOffline {
                store_id,
                cleared_order_ids,
            });

            // 无活跃 edge 且无 console 订阅者时，清理 tenant 条目
            if tenant.online_edges.is_empty()
                && tenant.snapshots.is_empty()
                && tenant.tx.receiver_count() == 0
            {
                drop(tenant);
                self.tenants.remove(&tenant_id);
            }
        }
    }

    /// 获取 tenant 下指定 edge 的活跃订单（新 console 连接初始化用）
    ///
    /// `store_ids` 为空时返回该 tenant 下所有 edge 的订单
    pub fn get_all_active(&self, tenant_id: i64, store_ids: &[i64]) -> Vec<LiveOrderSnapshot> {
        let tenant = match self.tenants.get(&tenant_id) {
            Some(t) => t,
            None => return Vec::new(),
        };

        let mut result = Vec::new();

        if store_ids.is_empty() {
            for entry in tenant.snapshots.iter() {
                for order in entry.value().iter() {
                    result.push(order.value().clone());
                }
            }
        } else {
            for &eid in store_ids {
                if let Some(orders) = tenant.snapshots.get(&eid) {
                    for order in orders.iter() {
                        result.push(order.value().clone());
                    }
                }
            }
        }

        result
    }

    /// 门店信息变更广播（Console/Edge 修改后调用）
    pub fn publish_store_info_updated(
        &self,
        tenant_id: i64,
        store_id: i64,
        info: shared::models::store_info::StoreInfo,
    ) {
        let tenant = self.get_or_create_tenant(tenant_id);
        let _ = tenant.tx.send(LiveHubEvent::StoreInfoUpdated {
            store_id,
            info: Box::new(info),
        });
    }

    /// 订阅 tenant 的 broadcast channel
    pub fn subscribe(&self, tenant_id: i64) -> broadcast::Receiver<LiveHubEvent> {
        let tenant = self.get_or_create_tenant(tenant_id);
        tenant.tx.subscribe()
    }

    fn get_or_create_tenant(
        &self,
        tenant_id: i64,
    ) -> dashmap::mapref::one::Ref<'_, i64, TenantLive> {
        self.tenants
            .entry(tenant_id)
            .or_insert_with(TenantLive::new)
            .downgrade()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::order::OrderSnapshot;

    fn make_snapshot(store_id: i64, order_id: i64) -> LiveOrderSnapshot {
        LiveOrderSnapshot {
            store_id,
            order: OrderSnapshot::new(order_id),
            events: vec![],
        }
    }

    #[test]
    fn basic_publish_get_remove() {
        let hub = LiveOrderHub::new();

        hub.publish_update(1, make_snapshot(1, 1001));
        hub.publish_update(1, make_snapshot(1, 1002));

        let all = hub.get_all_active(1, &[]);
        assert_eq!(all.len(), 2);

        let filtered = hub.get_all_active(1, &[1]);
        assert_eq!(filtered.len(), 2);

        let empty = hub.get_all_active(1, &[999]);
        assert!(empty.is_empty());

        hub.publish_remove(1, 1001, 1);
        let after = hub.get_all_active(1, &[]);
        assert_eq!(after.len(), 1);
        assert_eq!(after[0].order.order_id, 1002);

        hub.publish_remove(1, 1002, 1);
        assert!(hub.get_all_active(1, &[]).is_empty());
    }

    #[test]
    fn tenant_isolation() {
        let hub = LiveOrderHub::new();

        hub.publish_update(100, make_snapshot(1, 2001));
        hub.publish_update(200, make_snapshot(2, 2002));

        let a = hub.get_all_active(100, &[]);
        assert_eq!(a.len(), 1);
        assert_eq!(a[0].order.order_id, 2001);

        let b = hub.get_all_active(200, &[]);
        assert_eq!(b.len(), 1);
        assert_eq!(b[0].order.order_id, 2002);

        assert!(hub.get_all_active(999, &[]).is_empty());
    }

    #[test]
    fn store_isolation_via_edge_filter() {
        let hub = LiveOrderHub::new();

        hub.publish_update(1, make_snapshot(10, 3001));
        hub.publish_update(1, make_snapshot(10, 3002));
        hub.publish_update(1, make_snapshot(20, 3003));

        let store10 = hub.get_all_active(1, &[10]);
        assert_eq!(store10.len(), 2);
        assert!(store10.iter().all(|s| s.store_id == 10));

        let store20 = hub.get_all_active(1, &[20]);
        assert_eq!(store20.len(), 1);
        assert_eq!(store20[0].order.order_id, 3003);

        assert_eq!(hub.get_all_active(1, &[]).len(), 3);
    }

    #[tokio::test]
    async fn clear_edge_broadcasts_offline_with_order_ids() {
        let hub = LiveOrderHub::new();
        let mut rx = hub.subscribe(1);

        hub.publish_update(1, make_snapshot(1, 4001));
        hub.publish_update(1, make_snapshot(1, 4002));

        // 消费 OrderUpdated
        let _ = rx.recv().await.unwrap();
        let _ = rx.recv().await.unwrap();

        hub.clear_edge(1, 1);
        assert!(hub.get_all_active(1, &[1]).is_empty());

        match rx.recv().await.unwrap() {
            LiveHubEvent::EdgeOffline {
                store_id,
                cleared_order_ids,
            } => {
                assert_eq!(store_id, 1);
                assert_eq!(cleared_order_ids.len(), 2);
                assert!(cleared_order_ids.contains(&4001));
                assert!(cleared_order_ids.contains(&4002));
            }
            other => panic!("Expected EdgeOffline, got {other:?}"),
        }
    }

    #[test]
    fn tenant_auto_cleanup_when_empty() {
        let hub = LiveOrderHub::new();
        hub.publish_update(300, make_snapshot(1, 5001));
        assert_eq!(hub.get_all_active(300, &[]).len(), 1);

        // clear_edge 后无 subscriber + 无 edge → 自动清理
        hub.clear_edge(300, 1);
        assert!(hub.get_all_active(300, &[]).is_empty());
    }

    #[tokio::test]
    async fn subscribe_receives_updates_and_removals() {
        let hub = LiveOrderHub::new();
        let mut rx = hub.subscribe(1);

        hub.publish_update(1, make_snapshot(5, 6001));
        match rx.recv().await.unwrap() {
            LiveHubEvent::OrderUpdated(snap) => {
                assert_eq!(snap.store_id, 5);
                assert_eq!(snap.order.order_id, 6001);
            }
            other => panic!("Expected OrderUpdated, got {other:?}"),
        }

        hub.publish_remove(1, 6001, 5);
        match rx.recv().await.unwrap() {
            LiveHubEvent::OrderRemoved { order_id, store_id } => {
                assert_eq!(order_id, 6001);
                assert_eq!(store_id, 5);
            }
            other => panic!("Expected OrderRemoved, got {other:?}"),
        }
    }
}
