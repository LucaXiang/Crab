# Message Bus 使用指南

## 📋 架构概览

```
消息流转图
═══════════════════════════════════════════════════════════

HTTP 请求
   │
   ▼
/api/message/emit?body=Hello
   │
   ▼
MessageBus::publish(msg)
   │
   ├──────────────┬─────────────┬──────────────┐
   │              │             │              │
   ▼              ▼             ▼              ▼
服务端处理器   TCP 客户端   Oneshot 客户端  WebSocket
   │
   ▼
业务逻辑处理
 - 保存到数据库
 - 日志记录
 - 触发其他操作
```

## 🎯 消息处理的三个层次

### 1. **服务端处理器** (Server-side Handler)

**位置**: `edge-server/src/message/handler.rs`

**作用**: 服务器内部的业务逻辑处理

**处理内容**:
- 📝 记录到数据库
- 📊 更新统计数据
- 🔔 触发推送通知
- 📦 更新库存
- 💰 处理交易逻辑
- 等等...

**启动位置**: `ServerState::new()` 自动启动

```rust
// 服务器启动时自动订阅并处理消息
let handler = MessageHandler::new(receiver, shutdown_token);
tokio::spawn(async move {
    handler.run().await;
});
```

### 2. **TCP 客户端订阅者** (外部进程)

**位置**: `edge-server/examples/message_subscriber.rs`

**作用**: 跨进程接收消息（例如其他收银机、监控系统）

**启动方式**:
```bash
cargo run --example message_subscriber
```

### 3. **Oneshot 订阅者** (同进程)

**位置**: `edge-server/examples/oneshot_subscriber.rs`

**作用**: 同进程内零开销接收消息

**启动方式**:
```bash
cargo run --example oneshot_subscriber
```

---

## 🧪 完整测试流程

### 测试场景：模拟完整的消息流转

#### **终端 1**: 启动服务器
```bash
cd edge-server
cargo run --bin edge-server
```

你会看到：
```
🎯 Message handler started             ← 服务器处理器已启动
📡 Message bus TCP server listening...  ← TCP 服务器已启动
🚀 Server running on 0.0.0.0:8080
```

#### **终端 2**: 启动 TCP 订阅者
```bash
cd edge-server
cargo run --example message_subscriber
```

你会看到：
```
✅ Connected successfully!
🎧 Listening for messages...
```

#### **终端 3**: 发送测试消息
```bash
# 测试 1: 通知消息
curl "http://localhost:8080/api/message/emit?body=Hello%20World"

# 测试 2: 交易完成
curl "http://localhost:8080/api/message/emit?type=transaction&body=test"

# 测试 3: 广播消息
curl "http://localhost:8080/api/message/emit?type=broadcast&body=系统更新"
```

---

## 📊 观察结果

### **终端 1 (服务器日志)** 会显示：

```
INFO edge_server::message::handler: Received notification
    event="notification"
    title="Notification"
    body="Hello World"

INFO edge_server::routes::message: Message emitted: notification - Hello World

INFO edge_server::message::handler: Transaction completed
    event="transaction_complete"
    amount=1000
    terminal_id="terminal_test"
```

### **终端 2 (TCP 订阅者)** 会显示：

```
[14:23:45] 📢 NOTIFICATION
   Title: Notification
   Body:  Hello World

[14:23:50] 💰 TRANSACTION
   Amount:   $10.00
   Terminal: terminal_test

[14:23:55] 📣 BROADCAST
   From:    server
   Message: 系统更新
```

---

## 🔧 添加自定义业务逻辑

在 `edge-server/src/message/handler.rs` 中修改对应的处理方法：

```rust
/// 处理交易完成消息
async fn handle_transaction(&self, msg: &BusMessage) -> Result<(), Box<dyn std::error::Error>> {
    let payload: serde_json::Value = msg.parse_payload()?;
    let amount = payload["amount"].as_u64().unwrap_or(0);
    let terminal_id = payload["terminal_id"].as_str().unwrap_or("unknown");

    // ✅ 添加你的业务逻辑
    
    // 1. 保存到数据库
    // db.save_transaction(amount, terminal_id).await?;
    
    // 2. 更新库存
    // inventory.update_stock(transaction_items).await?;
    
    // 3. 生成收据
    // receipt_service.generate(transaction_id).await?;
    
    // 4. 发送通知
    // notification_service.send(user_id, "交易完成").await?;
    
    // 5. 触发积分计算
    // loyalty_service.add_points(user_id, amount).await?;

    Ok(())
}
```

---

## 🎨 支持的消息类型

| 类型 | 用途 | 示例 |
|------|------|------|
| **Notification** | 系统通知 | 系统更新、警告 |
| **TransactionComplete** | 交易完成 | 收银机交易完成 |
| **Broadcast** | 广播消息 | 群发消息、公告 |
| **PriceUpdate** | 价格更新 | 商品价格变动 |
| **StockUpdate** | 库存更新 | 库存变化通知 |
| **UserEvent** | 用户事件 | 登录、登出 |

---

## 🚀 性能特性

### 同进程通信 (Oneshot)
- ✅ **零开销**: 直接内存共享
- ✅ **无序列化**: 不需要 JSON 序列化
- ✅ **最快速度**: 纳秒级延迟

### 跨进程通信 (TCP)
- ✅ **读写分离**: 并发读写无阻塞
- ✅ **优雅关闭**: 支持平滑重启
- ✅ **自动重连**: 客户端断线重连
- ✅ **可配置容量**: 默认 1024 消息缓冲

---

## 🛠️ 常见问题

### Q: 消息丢失了怎么办？
A: 检查 channel 容量配置，增大 `channel_capacity`

### Q: 如何添加新的消息类型？
A: 
1. 在 `EventType` 枚举中添加新类型
2. 在 `MessageHandler` 中添加对应的处理方法
3. 更新 `handle_message()` 的 match 分支

### Q: 如何保证消息可靠性？
A: 
- 服务端处理器使用 Result 返回，记录失败日志
- 关键业务使用数据库事务保证一致性
- 考虑添加消息持久化层（如 Redis）

### Q: 如何进行性能监控？
A: 
- 监控 `RecvError::Lagged` 日志（消息滞后）
- 统计各类型消息的处理时间
- 监控 channel 使用率

---

## 📚 相关代码文件

- `edge-server/src/message/mod.rs` - MessageBus 核心
- `edge-server/src/message/handler.rs` - 服务端处理器
- `edge-server/src/message/types.rs` - 消息类型定义
- `edge-server/src/routes/message.rs` - 测试路由
- `edge-server/src/server/state.rs` - 启动配置
- `edge-server/examples/message_subscriber.rs` - TCP 订阅者示例
- `edge-server/examples/oneshot_subscriber.rs` - Oneshot 订阅者示例
