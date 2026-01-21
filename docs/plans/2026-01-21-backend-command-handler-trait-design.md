# 后端 OrderCommand 处理器 Trait 设计

**日期**: 2026-01-21
**目标**: 采用策略模式重构命令处理，类似前端 Timeline Renderer

---

## 一、当前问题分析

### 1.1 黑盒式命令处理

**当前代码** (`edge-server/src/orders/manager.rs`):

```rust
// 问题 1: 巨大的 match 语句（第 234-345 行）
let result = match &cmd.payload {
    OrderCommandPayload::ModifyItem {
        order_id, instance_id, affected_quantity,
        changes, authorizer_id, authorizer_name
    } => self.handle_modify_item(
        &txn, &cmd, order_id, instance_id,
        *affected_quantity, changes,
        authorizer_id.clone(), authorizer_name.clone() // 8 个参数！
    ),
    // ... 15 个其他命令
};

// 问题 2: 在 handler 内部又 match 一次！（第 371 行）
fn handle_modify_item(&self, txn, cmd, order_id, ...) {
    let (...) = match &cmd.payload {  // 重复提取字段
        OrderCommandPayload::ModifyItem { ... } => (...),
        _ => unreachable!(),  // 永远不会执行
    };
}
```

### 1.2 维护问题

| 问题 | 影响 |
|------|------|
| **双重 match** | 每个命令要模式匹配 2 次 |
| **参数冗余** | `handle_modify_item` 有 8 个参数 |
| **代码分散** | 新增命令要改 3 处（enum + match + handler） |
| **难以测试** | Handler 依赖 `self`，需要完整 OrderManager |
| **不可扩展** | 无法在外部添加新命令处理器 |
| **黑盒** | 看不出命令执行流程，调试困难 |

---

## 二、设计方案：CommandHandler Trait

### 2.1 核心思想

**类似前端 Timeline Renderer**：
- 每个命令类型有独立的 Handler struct
- Handler 实现 `CommandHandler` trait
- 通过注册表映射命令类型 → Handler

**架构对比**：

```
前端 Timeline:
  OrderEvent → EVENT_RENDERERS[event_type].render(event, payload)
              ↓
  返回 TimelineDisplayData (UI 数据)

后端 Command:
  OrderCommand → COMMAND_HANDLERS[command_type].handle(cmd, payload, ctx)
                ↓
  返回 (CommandResponse, Vec<OrderEvent>)
```

### 2.2 Trait 定义

**文件**: `edge-server/src/orders/handlers/mod.rs`

```rust
use super::storage::OrderStorage;
use crate::db::repository::DbConnection;
use shared::order::command::OrderCommand;
use shared::order::event::OrderEvent;
use shared::error::AppResult;

/// 命令处理上下文（替代 OrderManager self）
pub struct CommandContext<'a> {
    pub storage: &'a OrderStorage,
    pub db: &'a DbConnection,
    pub txn: &'a redb::WriteTransaction<'a>,
}

/// 命令处理器 Trait（类似前端 EventRenderer）
pub trait CommandHandler {
    /// 处理命令，返回响应和生成的事件
    fn handle(
        &self,
        cmd: &OrderCommand,
        ctx: CommandContext,
    ) -> AppResult<(CommandResponse, Vec<OrderEvent>)>;
}
```

### 2.3 Handler 实现示例

#### OpenTableHandler

**文件**: `edge-server/src/orders/handlers/open_table.rs`

```rust
use super::{CommandHandler, CommandContext};
use shared::order::command::{OrderCommand, OrderCommandPayload};
use shared::error::AppResult;

pub struct OpenTableHandler;

impl CommandHandler for OpenTableHandler {
    fn handle(
        &self,
        cmd: &OrderCommand,
        ctx: CommandContext,
    ) -> AppResult<(CommandResponse, Vec<OrderEvent>)> {
        // ✅ 直接解构 payload（只 match 一次）
        let OrderCommandPayload::OpenTable {
            table_id,
            table_name,
            zone_id,
            zone_name,
            guest_count,
            is_retail,
        } = &cmd.payload else {
            unreachable!("OpenTableHandler called with wrong payload type");
        };

        // 生成订单 ID
        let order_id = format!("order_{}", uuid::Uuid::new_v4());

        // 创建事件
        let event = OrderEvent {
            event_id: uuid::Uuid::new_v4().to_string(),
            sequence: 1,
            order_id: order_id.clone(),
            timestamp: chrono::Utc::now().timestamp_millis(),
            operator_id: cmd.operator_id.clone(),
            operator_name: cmd.operator_name.clone(),
            command_id: cmd.command_id.clone(),
            event_type: "TABLE_OPENED".to_string(),
            payload: TableOpenedPayload {
                table_id: table_id.clone(),
                table_name: table_name.clone(),
                zone_id: zone_id.clone(),
                zone_name: zone_name.clone(),
                guest_count: *guest_count,
                is_retail: *is_retail,
            },
        };

        // 存储事件
        ctx.storage.append_event(ctx.txn, &event)?;

        // 返回响应
        Ok((
            CommandResponse::success(order_id),
            vec![event],
        ))
    }
}
```

#### ModifyItemHandler

**文件**: `edge-server/src/orders/handlers/modify_item.rs`

```rust
pub struct ModifyItemHandler;

impl CommandHandler for ModifyItemHandler {
    fn handle(
        &self,
        cmd: &OrderCommand,
        ctx: CommandContext,
    ) -> AppResult<(CommandResponse, Vec<OrderEvent>)> {
        // ✅ 解构 payload（从 8 个参数 → 0 个参数）
        let OrderCommandPayload::ModifyItem {
            order_id,
            instance_id,
            affected_quantity,
            changes,
            authorizer_id,
            authorizer_name,
        } = &cmd.payload else {
            unreachable!();
        };

        // 验证订单存在
        let snapshot = ctx.storage.get_snapshot(ctx.txn, order_id)?;

        // 查找要修改的商品
        let item = snapshot.items.iter()
            .find(|i| &i.instance_id == instance_id)
            .ok_or_else(|| AppError::not_found("Item not found"))?;

        // 创建修改事件
        let event = OrderEvent {
            event_id: uuid::Uuid::new_v4().to_string(),
            sequence: snapshot.last_sequence + 1,
            order_id: order_id.clone(),
            timestamp: chrono::Utc::now().timestamp_millis(),
            operator_id: cmd.operator_id.clone(),
            operator_name: cmd.operator_name.clone(),
            command_id: cmd.command_id.clone(),
            event_type: "ITEM_MODIFIED".to_string(),
            payload: ItemModifiedPayload {
                source: item.clone(),
                changes: changes.clone(),
                previous_values: extract_previous_values(item, changes),
                affected_quantity: *affected_quantity,
            },
        };

        ctx.storage.append_event(ctx.txn, &event)?;

        Ok((CommandResponse::success_empty(), vec![event]))
    }
}
```

### 2.4 Handler 注册表

**文件**: `edge-server/src/orders/handlers/registry.rs`

```rust
use std::collections::HashMap;
use once_cell::sync::Lazy;
use super::*;

/// 命令类型（字符串）
type CommandType = &'static str;

/// Handler 注册表（类似前端 EVENT_RENDERERS）
pub static COMMAND_HANDLERS: Lazy<HashMap<CommandType, Box<dyn CommandHandler + Send + Sync>>> = Lazy::new(|| {
    let mut handlers: HashMap<CommandType, Box<dyn CommandHandler + Send + Sync>> = HashMap::new();

    // 注册所有 handler
    handlers.insert("OPEN_TABLE", Box::new(OpenTableHandler));
    handlers.insert("COMPLETE_ORDER", Box::new(CompleteOrderHandler));
    handlers.insert("VOID_ORDER", Box::new(VoidOrderHandler));
    handlers.insert("ADD_ITEMS", Box::new(AddItemsHandler));
    handlers.insert("MODIFY_ITEM", Box::new(ModifyItemHandler));
    handlers.insert("REMOVE_ITEM", Box::new(RemoveItemHandler));
    handlers.insert("ADD_PAYMENT", Box::new(AddPaymentHandler));
    handlers.insert("CANCEL_PAYMENT", Box::new(CancelPaymentHandler));
    handlers.insert("SPLIT_ORDER", Box::new(SplitOrderHandler));
    handlers.insert("MOVE_ORDER", Box::new(MoveOrderHandler));
    handlers.insert("MERGE_ORDERS", Box::new(MergeOrdersHandler));
    handlers.insert("UPDATE_ORDER_INFO", Box::new(UpdateOrderInfoHandler));

    handlers
});

/// 获取命令类型字符串
pub fn get_command_type(payload: &OrderCommandPayload) -> &'static str {
    match payload {
        OrderCommandPayload::OpenTable { .. } => "OPEN_TABLE",
        OrderCommandPayload::CompleteOrder { .. } => "COMPLETE_ORDER",
        OrderCommandPayload::VoidOrder { .. } => "VOID_ORDER",
        OrderCommandPayload::AddItems { .. } => "ADD_ITEMS",
        OrderCommandPayload::ModifyItem { .. } => "MODIFY_ITEM",
        OrderCommandPayload::RemoveItem { .. } => "REMOVE_ITEM",
        OrderCommandPayload::AddPayment { .. } => "ADD_PAYMENT",
        OrderCommandPayload::CancelPayment { .. } => "CANCEL_PAYMENT",
        OrderCommandPayload::SplitOrder { .. } => "SPLIT_ORDER",
        OrderCommandPayload::MoveOrder { .. } => "MOVE_ORDER",
        OrderCommandPayload::MergeOrders { .. } => "MERGE_ORDERS",
        OrderCommandPayload::UpdateOrderInfo { .. } => "UPDATE_ORDER_INFO",
        _ => "UNKNOWN",
    }
}
```

### 2.5 简化的 OrderManager

**文件**: `edge-server/src/orders/manager.rs` (重构后)

```rust
impl OrderManager {
    pub fn execute_command(&self, cmd: OrderCommand) -> AppResult<(CommandResponse, Vec<OrderEvent>)> {
        // 1. 幂等性检查
        if self.storage.is_command_processed(&cmd.command_id)? {
            return Ok((CommandResponse::already_processed(), vec![]));
        }

        // 2. 开启事务
        let txn = self.storage.db.begin_write()?;

        // 3. 查找 Handler（✅ 无 match 语句！）
        let command_type = handlers::get_command_type(&cmd.payload);
        let handler = handlers::COMMAND_HANDLERS
            .get(command_type)
            .ok_or_else(|| AppError::invalid_input("Unknown command type"))?;

        // 4. 创建上下文
        let ctx = CommandContext {
            storage: &self.storage,
            db: &self.db,
            txn: &txn,
        };

        // 5. 委托给 Handler（✅ 策略模式！）
        let result = handler.handle(&cmd, ctx)?;

        // 6. 标记命令已处理
        self.storage.mark_command_processed(&txn, &cmd.command_id)?;

        // 7. 提交事务
        txn.commit()?;

        Ok(result)
    }
}
```

**对比**：
- **旧代码**: 110+ 行巨大 match 语句
- **新代码**: 20 行，无 match，策略模式

---

## 三、优势分析

### 3.1 代码质量

| 指标 | 旧架构 | 新架构 | 改进 |
|------|--------|--------|------|
| **match 次数** | 2 次/命令 | 0 次 | ✅ 消除重复 |
| **handler 参数** | 8 个 | 0 个（通过 ctx） | ✅ 简化签名 |
| **新增命令改动点** | 3 处 | 1 处 | ✅ 易扩展 |
| **测试复杂度** | 需要完整 Manager | 可独立测试 Handler | ✅ 易测试 |
| **代码行数** | ~1500 行 | ~800 行 | ✅ 减少 47% |

### 3.2 可维护性

**新增命令流程**：

```rust
// 旧架构：改 3 处
1. shared/src/order/command.rs - 添加 enum variant
2. edge-server/src/orders/manager.rs - 添加 match arm
3. edge-server/src/orders/manager.rs - 添加 handler 函数

// 新架构：改 1 处
1. edge-server/src/orders/handlers/my_command.rs - 创建 Handler
2. 在 registry.rs 注册（1 行代码）
```

### 3.3 可扩展性

**外部扩展**：

```rust
// 可以在外部模块添加自定义 Handler
pub struct CustomCommandHandler;

impl CommandHandler for CustomCommandHandler {
    fn handle(&self, cmd, ctx) -> AppResult<...> {
        // 自定义逻辑
    }
}

// 注册到系统
COMMAND_HANDLERS.insert("CUSTOM_COMMAND", Box::new(CustomCommandHandler));
```

### 3.4 调试能力

**旧架构**：
```rust
// 黑盒：不知道哪个 handler 被调用
execute_command(cmd) → ??? → Result
```

**新架构**：
```rust
// 透明：可以打印 handler 名称
let handler = COMMAND_HANDLERS.get(command_type)?;
info!("Executing handler: {}", command_type);
handler.handle(cmd, ctx)?;
```

---

## 四、实施步骤

### Phase 1: 基础设施（1 天）

**任务**:
1. 创建 `handlers/` 目录结构
2. 定义 `CommandHandler` trait
3. 创建 `CommandContext` struct
4. 实现 `COMMAND_HANDLERS` 注册表

**文件**:
- `edge-server/src/orders/handlers/mod.rs`
- `edge-server/src/orders/handlers/registry.rs`

### Phase 2: 迁移 Handlers（2-3 天）

**任务**: 逐个迁移现有命令

**优先级**:
1. **高频命令** (先迁移，影响大):
   - `AddItems` - 加菜
   - `AddPayment` - 支付
   - `CompleteOrder` - 结账

2. **中频命令**:
   - `ModifyItem` - 修改商品
   - `RemoveItem` - 删除商品
   - `VoidOrder` - 作废

3. **低频命令**:
   - `MoveOrder`, `MergeOrders` - 桌台操作
   - `SplitOrder` - 分账
   - `UpdateOrderInfo` - 更新信息

**每个 Handler 的工作量**: 30-60 分钟

### Phase 3: 重构 OrderManager（1 天）

**任务**:
1. 删除巨大 match 语句
2. 使用 Handler 注册表
3. 删除旧 `handle_xxx` 函数
4. 更新测试

### Phase 4: 测试验证（1 天）

**任务**:
1. 单元测试每个 Handler
2. 集成测试命令流程
3. 端到端测试（Tauri 调用）
4. 性能测试（确保无退化）

**总工期**: 5-6 天

---

## 五、风险与缓解

### 5.1 向后兼容性

**风险**: 修改核心命令处理逻辑，可能破坏现有功能

**缓解**:
- 增量迁移：每迁移一个 Handler 就测试
- 保留旧代码：在新 Handler 稳定前，旧 `handle_xxx` 保留为 fallback
- Feature flag：通过配置切换新旧实现

### 5.2 性能影响

**风险**: 动态分发（trait object）可能影响性能

**缓解**:
- Benchmark 测试：对比新旧实现性能
- `Lazy` 静态初始化：注册表只初始化一次
- 内联优化：关键路径使用 `#[inline]`

**预期**: 性能影响 <5%（可接受）

### 5.3 测试覆盖

**风险**: 重构可能引入新 bug

**缓解**:
- TDD：先写测试，再实现 Handler
- 覆盖率要求：每个 Handler 测试覆盖 >90%
- 端到端测试：模拟真实订单流程

---

## 六、验收标准

### 6.1 代码质量

- [ ] 删除 `manager.rs` 中的 match 语句
- [ ] 每个命令有独立 Handler struct
- [ ] Handler 可独立测试（无需 Manager）
- [ ] 无 `unreachable!()` 代码

### 6.2 功能完整

- [ ] 所有 12 个命令成功迁移
- [ ] 端到端测试通过
- [ ] 性能无明显退化（<5%）

### 6.3 可维护性

- [ ] 新增命令只需 1 个文件
- [ ] Handler 测试覆盖 >90%
- [ ] 文档说明如何添加新 Handler

---

## 七、长期收益

### 7.1 开发效率

**新增命令时间**:
- 旧架构: 2-3 小时（理解 match 逻辑 + 修改多处）
- 新架构: 30-60 分钟（创建 Handler + 注册）

**节省**: 60-70% 开发时间

### 7.2 Bug 减少

**常见 Bug**:
- 忘记在某个 match 分支添加新命令
- 参数传递错误（8 个参数容易出错）
- `unreachable!()` 被意外触发

**新架构**: 编译时保证类型安全，运行时自动分发

### 7.3 团队协作

**旧架构**: 多人同时修改 `manager.rs` 容易冲突

**新架构**: 每个 Handler 独立文件，无冲突

---

## 八、总结

### 核心改进

```
旧架构（黑盒）:
  OrderCommand → 巨大 match → handle_xxx(8 个参数) → 内部再 match
                  ↓
          难以理解、难以测试、难以扩展

新架构（策略模式）:
  OrderCommand → COMMAND_HANDLERS[type] → handler.handle(cmd, ctx)
                  ↓
          清晰、可测试、易扩展
```

### 预期成果

- ✅ 删除 700+ 行重复代码
- ✅ 每个命令独立、可测试
- ✅ 新增命令工作量减少 60%
- ✅ 类似前端 Renderer 的优雅架构
- ✅ 消除"黑盒"，代码透明可调试

**推荐优先级**: 高 - 基础架构改进，长期收益巨大
