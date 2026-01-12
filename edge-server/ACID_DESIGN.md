# Message Bus ACID 设计文档

## 🎯 设计目标

确保消息处理具备 **ACID** 特性和高**可维护性**：

- ✅ **Atomicity (原子性)**: 消息处理要么全部成功，要么全部失败
- ✅ **Consistency (一致性)**: 数据始终保持一致状态
- ✅ **Isolation (隔离性)**: 并发消息处理互不干扰
- ✅ **Durability (持久性)**: 处理结果持久化存储
- ✅ **Maintainability (可维护性)**: 代码清晰、可扩展、易测试

---

## 📐 架构设计

### 三层架构

```
┌─────────────────────────────────────────────────────────┐
│                    应用层 (Routes)                        │
│              /api/message/emit → MessageBus              │
└───────────────────────────┬─────────────────────────────┘
                            │
┌───────────────────────────▼─────────────────────────────┐
│                 消息总线层 (MessageBus)                   │
│           broadcast::channel(可配置容量)                  │
│         ┌─────────────┬──────────────┬────────────┐     │
│         │             │              │            │     │
│      服务端处理器   TCP客户端   Oneshot客户端  WebSocket │
└─────────┬─────────────────────────────────────────────┘
          │
┌─────────▼─────────────────────────────────────────────┐
│              处理层 (MessageHandler)                    │
│  ┌────────────────────────────────────────────────┐   │
│  │  1. 接收消息                                     │   │
│  │  2. 路由到对应的 Processor                       │   │
│  │  3. 自动重试（指数退避）                         │   │
│  │  4. 死信队列（DLQ）                              │   │
│  └────────────────────────────────────────────────┘   │
└─────────┬─────────────────────────────────────────────┘
          │
┌─────────▼─────────────────────────────────────────────┐
│            业务逻辑层 (MessageProcessor)                 │
│  ┌───────────────────────────────────────────────┐    │
│  │  NotificationProcessor (通知)                  │    │
│  │  TransactionProcessor (交易)                   │    │
│  │  BroadcastProcessor (广播)                     │    │
│  │  ... (可扩展)                                   │    │
│  └───────────────────────────────────────────────┘    │
│  每个 Processor 实现：                                  │
│  - process(): 处理逻辑 + ACID 事务                     │
│  - is_duplicate(): 幂等性检查                          │
│  - max_retries(): 重试次数                             │
│  - retry_delay_ms(): 重试延迟                          │
└─────────┬─────────────────────────────────────────────┘
          │
┌─────────▼─────────────────────────────────────────────┐
│                 数据层 (Database)                       │
│  - 事务管理 (Transactions)                              │
│  - 幂等性表 (processed_messages)                       │
│  - 死信队列表 (dead_letter_queue)                      │
│  - 业务数据表 (transactions, notifications, etc.)       │
└─────────────────────────────────────────────────────────┘
```

---

## 🔒 ACID 实现

### 1. Atomicity (原子性)

通过数据库事务确保操作的原子性：

```rust
async fn process(&self, msg: &BusMessage) -> Result<ProcessResult, AppError> {
    // 开启事务
    let mut tx = db.begin().await?;
    
    try {
        // 1. 检查幂等性
        if db.check_processed(msg_id, &tx).await? {
            return Ok(ProcessResult::Skipped { ... });
        }
        
        // 2. 执行业务逻辑
        db.insert_transaction(amount, terminal_id, &tx).await?;
        db.update_inventory(items, &tx).await?;
        db.add_loyalty_points(user_id, points, &tx).await?;
        
        // 3. 标记消息已处理
        db.mark_processed(msg_id, &tx).await?;
        
        // 4. 提交事务
        tx.commit().await?;
        
        Ok(ProcessResult::Success { ... })
    } catch (e) {
        // 事务自动回滚
        tx.rollback().await?;
        Err(e)
    }
}
```

### 2. Consistency (一致性)

保证数据始终处于一致状态：

1. **数据验证**: 在处理前验证消息格式
2. **约束检查**: 使用数据库约束（外键、唯一索引等）
3. **业务规则**: 在事务内执行所有相关操作

```rust
// 示例：交易处理
async fn process_transaction(&self, msg: &BusMessage) -> Result<ProcessResult, AppError> {
    let payload: TransactionPayload = msg.parse_payload()?;
    
    // 验证数据
    payload.validate()?;
    
    let mut tx = db.begin().await?;
    
    // 检查库存是否足够（一致性检查）
    if !db.check_inventory_sufficient(&payload.items, &tx).await? {
        return Ok(ProcessResult::Failed {
            reason: "Insufficient inventory".to_string(),
        });
    }
    
    // 执行所有相关操作
    db.save_transaction(&payload, &tx).await?;
    db.reduce_inventory(&payload.items, &tx).await?;
    db.add_points(payload.user_id, &tx).await?;
    
    tx.commit().await?;
    Ok(ProcessResult::Success { ... })
}
```

### 3. Isolation (隔离性)

防止并发消息处理互相干扰：

1. **数据库隔离级别**: 使用适当的事务隔离级别
2. **悲观锁**: 关键资源使用 `SELECT FOR UPDATE`
3. **乐观锁**: 使用版本号检测冲突

```rust
// 示例：库存更新（使用悲观锁）
async fn update_inventory(items: &[Item], tx: &mut Transaction) -> Result<(), DbError> {
    for item in items {
        // SELECT FOR UPDATE 锁定行
        let inventory = db.query_one(
            "SELECT * FROM inventory WHERE product_id = $1 FOR UPDATE",
            &[item.product_id]
        ).await?;
        
        // 更新库存
        db.execute(
            "UPDATE inventory SET quantity = quantity - $1 WHERE product_id = $2",
            &[item.quantity, item.product_id]
        ).await?;
    }
    Ok(())
}
```

### 4. Durability (持久性)

确保处理结果持久化：

1. **事务提交**: 只有在 `tx.commit()` 成功后才返回 Success
2. **WAL (Write-Ahead Logging)**: 数据库自动支持
3. **消息标记**: 记录已处理的消息 ID

```rust
// 幂等性表结构
CREATE TABLE processed_messages (
    message_id TEXT PRIMARY KEY,
    event_type TEXT NOT NULL,
    processed_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    result JSON,
    INDEX idx_event_type (event_type),
    INDEX idx_processed_at (processed_at)
);

// 标记消息已处理
async fn mark_processed(
    msg_id: &str,
    result: &serde_json::Value,
    tx: &mut Transaction
) -> Result<(), DbError> {
    db.execute(
        "INSERT INTO processed_messages (message_id, event_type, result) 
         VALUES ($1, $2, $3)",
        &[msg_id, event_type, result]
    ).await?;
    Ok(())
}
```

---

## 🔄 幂等性设计

### 幂等性检查策略

1. **消息 ID**: 每条消息唯一 ID（推荐）
2. **业务 ID**: 例如 `(terminal_id, timestamp, amount)` 组合
3. **内容哈希**: 消息内容的 SHA256

### 实现示例

```rust
#[async_trait]
impl MessageProcessor for TransactionProcessor {
    async fn is_duplicate(&self, msg: &BusMessage) -> Result<bool, AppError> {
        let payload: Value = msg.parse_payload()?;
        let terminal_id = payload["terminal_id"].as_str().unwrap();
        let timestamp = payload["timestamp"].as_str().unwrap();
        let amount = payload["amount"].as_u64().unwrap();
        
        // 检查是否已经处理过这个交易
        let exists = db.query_one(
            "SELECT EXISTS(
                SELECT 1 FROM transactions 
                WHERE terminal_id = $1 
                  AND timestamp = $2 
                  AND amount = $3
            )",
            &[terminal_id, timestamp, amount]
        ).await?;
        
        Ok(exists)
    }
}
```

---

## 🔁 重试机制

### 指数退避重试

```rust
// 配置
max_retries: 5
base_delay: 1000ms

// 重试延迟计算
delay = base_delay * 2^(retry_count - 1)

// 重试序列
Attempt 1: 立即处理
Attempt 2: 1000ms 后重试
Attempt 3: 2000ms 后重试
Attempt 4: 4000ms 后重试
Attempt 5: 8000ms 后重试
```

### 何时重试

```rust
pub enum ProcessResult {
    Success { message: String },          // ✅ 成功，不重试
    Skipped { reason: String },          // ⏭️  跳过，不重试
    Retry { reason: String },            // 🔄 失败，需要重试
    Failed { reason: String },           // ❌ 永久失败，不重试
}
```

### 重试示例

```rust
// 网络错误 → 重试
Err(NetworkError) => ProcessResult::Retry { 
    reason: "Network timeout".to_string() 
}

// 数据库死锁 → 重试
Err(DeadlockDetected) => ProcessResult::Retry { 
    reason: "Database deadlock".to_string() 
}

// 数据验证错误 → 永久失败
Err(ValidationError) => ProcessResult::Failed { 
    reason: "Invalid data format".to_string() 
}
```

---

## 💀 死信队列 (DLQ)

### 何时发送到 DLQ

1. 超过最大重试次数
2. 返回 `ProcessResult::Failed`
3. 处理抛出无法恢复的异常

### DLQ 表结构

```sql
CREATE TABLE dead_letter_queue (
    id SERIAL PRIMARY KEY,
    message_id TEXT,
    event_type TEXT NOT NULL,
    payload BYTEA NOT NULL,
    failure_reason TEXT NOT NULL,
    retry_count INT NOT NULL,
    first_attempt_at TIMESTAMP NOT NULL,
    failed_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    INDEX idx_event_type (event_type),
    INDEX idx_failed_at (failed_at)
);
```

### 处理 DLQ 消息

```bash
# 1. 查询 DLQ
SELECT * FROM dead_letter_queue 
WHERE event_type = 'transaction_complete' 
ORDER BY failed_at DESC 
LIMIT 100;

# 2. 修复数据后重新发送
curl -X POST http://localhost:8080/api/message/emit \
  -d '{"message_type": "transaction", "body": "..."}'

# 3. 清理已处理的 DLQ 消息
DELETE FROM dead_letter_queue WHERE id IN (...);
```

---

## 🧩 可扩展性

### 添加新的消息处理器

```rust
// 1. 定义新的 Processor
pub struct PriceUpdateProcessor;

#[async_trait]
impl MessageProcessor for PriceUpdateProcessor {
    fn event_type(&self) -> EventType {
        EventType::PriceUpdate
    }

    async fn process(&self, msg: &BusMessage) -> Result<ProcessResult, AppError> {
        // 实现处理逻辑
        let mut tx = db.begin().await?;
        
        // 更新价格
        db.update_price(product_id, new_price, &tx).await?;
        
        // 记录历史
        db.insert_price_history(product_id, old_price, new_price, &tx).await?;
        
        // 标记已处理
        db.mark_processed(msg_id, &tx).await?;
        
        tx.commit().await?;
        Ok(ProcessResult::Success { ... })
    }

    fn max_retries(&self) -> u32 {
        3 // 价格更新重试 3 次
    }
}

// 2. 注册 Processor
let handler = MessageHandler::new(receiver, shutdown_token)
    .register_processor(Arc::new(NotificationProcessor))
    .register_processor(Arc::new(TransactionProcessor))
    .register_processor(Arc::new(PriceUpdateProcessor)); // 新增
```

---

## 🧪 测试

### 单元测试

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_transaction_processor_success() {
        let processor = TransactionProcessor::new(mock_db());
        let msg = BusMessage::transaction_complete(1000, "terminal_a");
        
        let result = processor.process(&msg).await.unwrap();
        assert!(result.is_success());
    }

    #[tokio::test]
    async fn test_idempotency() {
        let processor = TransactionProcessor::new(mock_db());
        let msg = BusMessage::transaction_complete(1000, "terminal_a");
        
        // 第一次处理
        processor.process(&msg).await.unwrap();
        
        // 第二次处理 - 应该被跳过
        let result = processor.process(&msg).await.unwrap();
        assert!(matches!(result, ProcessResult::Skipped { .. }));
    }

    #[tokio::test]
    async fn test_retry_on_network_error() {
        let processor = TransactionProcessor::new(failing_db());
        let msg = BusMessage::transaction_complete(1000, "terminal_a");
        
        let result = processor.process(&msg).await.unwrap();
        assert!(result.should_retry());
    }
}
```

### 集成测试

```bash
# 启动服务器
cargo run --bin edge-server

# 启动订阅者
cargo run --example message_subscriber

# 发送测试消息
curl "http://localhost:8080/api/message/emit?type=transaction&body=test"

# 观察日志
# - 服务器处理日志
# - 订阅者接收日志
# - 重试日志（如果有错误）
```

---

## 📊 监控指标

### 关键指标

1. **处理成功率**: `success_count / total_count`
2. **平均处理时间**: `avg(processing_time)`
3. **重试率**: `retry_count / total_count`
4. **DLQ 消息数**: `count(dead_letter_queue)`
5. **消息滞后**: `RecvError::Lagged` 次数

### 日志示例

```
INFO  Message processed successfully
      event_type=TransactionComplete
      processing_time=23ms
      retry_count=0

WARN  Retrying message processing
      event_type=TransactionComplete
      retry_count=2
      delay_ms=2000
      reason="Database connection timeout"

ERROR Sending message to dead letter queue
      event_type=TransactionComplete
      reason="Max retries exceeded"
      payload_len=256
```

---

## 📚 最佳实践

### DO ✅

1. **使用事务包裹所有相关操作**
2. **实现幂等性检查**
3. **合理设置重试次数和延迟**
4. **记录详细的处理日志**
5. **监控 DLQ 并及时处理**
6. **对关键资源使用锁**
7. **验证输入数据**

### DON'T ❌

1. **不要在事务外执行副作用操作**（如发送邮件、HTTP 请求）
2. **不要忽略幂等性**
3. **不要无限重试**
4. **不要在 processor 中直接调用外部服务**（应该通过发布新消息）
5. **不要使用过长的事务**
6. **不要忽略死信队列**

---

## 🔗 相关文件

- `edge-server/src/message/processor.rs` - Processor trait 定义
- `edge-server/src/message/handler.rs` - 消息处理器实现
- `edge-server/src/message/mod.rs` - MessageBus 核心
- `edge-server/MESSAGE_BUS_GUIDE.md` - 使用指南
- `edge-server/ACID_DESIGN.md` - 本文档
