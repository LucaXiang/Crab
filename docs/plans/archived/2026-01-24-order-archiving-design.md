# 订单归档到 SurrealDB 设计方案

## 概述

将已完成的订单从 redb（活跃订单存储）归档到 SurrealDB（持久化存储）。

## 设计决策

| 决策点 | 选择 |
|--------|------|
| 触发时机 | 完成即归档（状态变为终态时立即执行） |
| 存储内容 | 快照 + 事件（完整事件溯源） |
| 归档状态 | COMPLETED, VOID, MOVED, MERGED |
| 执行位置 | order_manager 内部处理 |
| redb 处理 | 归档成功后立即删除 |

## 数据流

```
订单状态变更 (COMPLETED/VOID/MOVED/MERGED)
         │
         ▼
┌─────────────────────────────────────────┐
│         order_manager 事务              │
│  ┌───────────────────────────────────┐  │
│  │ 1. 应用事件到快照                  │  │
│  │ 2. 写入 SurrealDB (order + events)│  │
│  │ 3. 从 redb 删除                    │  │
│  └───────────────────────────────────┘  │
│         (原子操作，任一失败则回滚)        │
└─────────────────────────────────────────┘
         │
         ▼
    广播事件到客户端
```

## SurrealDB Schema 更新

```surql
-- order 表状态字段更新
DEFINE FIELD OVERWRITE status ON order TYPE string
    ASSERT $value IN ["COMPLETED", "VOID", "MOVED", "MERGED"]
    PERMISSIONS FULL;

-- 新增字段：关联订单（用于 MOVED/MERGED 场景）
DEFINE FIELD OVERWRITE related_order_id ON order TYPE option<record<order>>
    PERMISSIONS FULL;

-- 新增字段：操作员
DEFINE FIELD OVERWRITE operator_id ON order TYPE option<string>
    PERMISSIONS FULL;
```

## Hash 链逻辑

```
system_state.last_order_hash
         │
         ▼ (作为 prev_hash)
┌─────────────────────────────────────┐
│            Order                     │
│  prev_hash: 来自 system_state        │
│  curr_hash: hash(order_data + last_event.curr_hash)
└─────────────────────────────────────┘
         │
         ▼ (更新)
system_state.last_order_hash = order.curr_hash
```

事件链：
```
event_1.curr_hash → event_2.prev_hash
event_2.curr_hash → event_3.prev_hash
...
last_event.curr_hash → 纳入 order.curr_hash 计算
```

## order_manager 归档逻辑

```rust
// 终态事件类型
const TERMINAL_EVENTS: &[&str] = &[
    "ORDER_COMPLETED",
    "ORDER_VOIDED",
    "ORDER_MOVED",
    "ORDER_MERGED",
];

impl OrderManager {
    async fn process_command(&self, cmd: Command) -> Result<()> {
        // 1. 生成事件
        let event = self.handle_command(cmd)?;

        // 2. 应用事件到快照
        let snapshot = self.apply_event(&event)?;

        // 3. 检查是否为终态
        if TERMINAL_EVENTS.contains(&event.event_type.as_str()) {
            // 归档流程
            self.archive_order(&snapshot, &events).await?;
        } else {
            // 普通流程：只写 redb
            self.persist_to_redb(&snapshot, &event).await?;
        }

        // 4. 广播事件
        self.broadcast(event).await?;
        Ok(())
    }

    async fn archive_order(&self, snapshot: &Order, events: &[OrderEvent]) -> Result<()> {
        let order_id = &snapshot.id;

        // 1. 获取 system_state 中的 last_order_hash
        let prev_hash = self.db.query("SELECT last_order_hash FROM system_state LIMIT 1")
            .await?;

        // 2. 计算 order hash
        let last_event_hash = events.last().map(|e| &e.curr_hash);
        let order_with_hash = snapshot.with_hash(prev_hash, last_event_hash);

        // 3. SurrealDB 事务
        self.db.query("BEGIN TRANSACTION").await?;

        let result = async {
            // 写入 order
            self.db.create("order", &order_with_hash).await?;

            // 写入 events + RELATE
            for event in events {
                self.db.create("order_event", event).await?;
                self.db.query("RELATE $order->has_event->$event")
                    .bind(("order", order_id))
                    .bind(("event", &event.id))
                    .await?;
            }

            // 更新 system_state
            self.db.query("UPDATE system_state SET last_order_hash = $hash")
                .bind(("hash", &order_with_hash.curr_hash))
                .await?;

            Ok::<_, Error>(())
        }.await;

        match result {
            Ok(_) => {
                self.db.query("COMMIT TRANSACTION").await?;
                // SurrealDB 成功，删除 redb
                self.redb.delete_order(order_id)?;
                self.redb.delete_events(order_id)?;
                Ok(())
            }
            Err(e) => {
                self.db.query("CANCEL TRANSACTION").await?;
                // 订单保留在 redb，返回错误
                Err(e)
            }
        }
    }
}
```

## 历史订单查询 API

```rust
// 列表查询
pub async fn fetch_order_list(
    State(state): State<ServerState>,
    Query(params): Query<OrderListParams>,
) -> Result<Json<Vec<OrderSummary>>> {
    let orders = state.db.query(r#"
        SELECT
            id,
            receipt_number,
            status,
            zone_name,
            table_name,
            total_amount,
            paid_amount,
            start_time,
            end_time,
            guest_count
        FROM order
        WHERE end_time >= $start_date
          AND end_time <= $end_date
        ORDER BY end_time DESC
        LIMIT $limit
    "#)
    .bind(("start_date", params.start_date))
    .bind(("end_date", params.end_date))
    .bind(("limit", params.limit.unwrap_or(100)))
    .await?;

    Ok(Json(orders))
}

// 详情查询（含事件）
pub async fn fetch_order_detail(
    State(state): State<ServerState>,
    Path(order_id): Path<String>,
) -> Result<Json<OrderDetail>> {
    let result = state.db.query(r#"
        SELECT *,
            ->has_event->order_event.* AS events
        FROM order
        WHERE id = $order_id
    "#)
    .bind(("order_id", order_id))
    .await?;

    Ok(Json(result))
}
```

## 错误处理

| 失败场景 | 处理方式 |
|----------|----------|
| SurrealDB 写入失败 | 事务回滚，订单留在 redb，可重试 |
| redb 删除失败 | 数据已在 SurrealDB，重启后检测并清理 |

## 实现步骤

1. 更新 SurrealDB schema (`edge-server/migrations/schemas/order.surql`)
2. 实现 order repository 的归档写入方法 (`edge-server/src/db/repository/order.rs`)
3. 实现 system_state 的 hash 链操作 (`edge-server/src/db/repository/system_state.rs`)
4. 修改 order_manager 添加归档逻辑 (`edge-server/src/orders/manager.rs`)
5. 更新 API 查询层 (`edge-server/src/api/orders/handler.rs`)
6. 测试归档流程

## 涉及文件

| 位置 | 修改内容 |
|------|----------|
| `edge-server/migrations/schemas/order.surql` | 更新 status 约束，添加 related_order_id、operator_id |
| `edge-server/src/orders/manager.rs` | 添加 archive_order 逻辑，终态检测 |
| `edge-server/src/db/repository/order.rs` | 新增 SurrealDB 订单写入方法 |
| `edge-server/src/db/repository/system_state.rs` | hash 链读写方法 |
| `edge-server/src/api/orders/handler.rs` | 更新历史订单查询，从 SurrealDB 读取 |
