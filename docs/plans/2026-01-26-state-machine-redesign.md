# 状态机重设计方案

## 1. 当前问题

### 1.1 三个状态源不同步
```
edge-server.ActivationService.credential_cache  → 激活状态
ClientBridge.mode.client (CrabClient)           → 认证状态
TenantManager.current_session                   → 会话缓存 (冗余!)
```

### 1.2 状态检测逻辑分散
- `get_app_state()` 每次调用都要组合多个状态源
- 检测逻辑 400+ 行，难以维护
- 重复的自检调用

### 1.3 状态转换不明确
- 没有明确的状态机定义
- 转换条件散布在各处
- 无法保证状态一致性

---

## 2. 新设计原则

### 2.1 单一事实来源 (Single Source of Truth)
- **edge-server** 是激活/订阅状态的权威
- **CrabClient** 是认证状态的权威
- **删除** TenantManager.current_session（只保留磁盘缓存用于恢复）

### 2.2 类型状态模式 (Typestate Pattern)
- 使用 Rust 类型系统保证状态转换安全
- 编译期防止非法状态转换
- 每个状态只暴露该状态下合法的操作

### 2.3 事件驱动 (Event-Driven)
- 状态变更通过事件通知
- 前端订阅状态变更，无需轮询
- 减少 `get_app_state()` 调用频率

---

## 3. 新状态机设计

### 3.1 Server 模式状态机

```
                    ┌─────────────────────┐
                    │     Unbound         │ ← 初始状态 / 自检失败
                    │  (无有效凭证)        │
                    └──────────┬──────────┘
                               │ activate()
                               ▼
                    ┌─────────────────────┐
                    │    Activating       │ ← 正在激活
                    │  (验证中)           │
                    └──────────┬──────────┘
                               │ self_check passed
                               ▼
                    ┌─────────────────────┐
                    │     Activated       │ ← 激活成功，服务就绪
                    │  (服务已启动)        │
                    └──────────┬──────────┘
                               │ employee_login()
                               ▼
                    ┌─────────────────────┐
                    │   Authenticated     │ ← 员工已登录
                    │  (可操作)           │
                    └─────────────────────┘
                               │ logout() / session_expired
                               ▼
                         (返回 Activated)
```

### 3.2 状态定义 (Rust Typestate)

```rust
// ===== 状态标记类型 =====
pub struct Unbound;
pub struct Activating;
pub struct Activated;
pub struct Authenticated;

// ===== 服务器状态机 =====
pub struct ServerMachine<S> {
    config: ServerConfig,
    _state: PhantomData<S>,
    inner: ServerInner,
}

struct ServerInner {
    edge_server: Option<EdgeServerHandle>,
    credential: Option<TenantBinding>,
    employee: Option<EmployeeInfo>,
}

// ===== 状态特有的实现 =====

impl ServerMachine<Unbound> {
    /// 只有 Unbound 状态可以激活
    pub async fn activate(
        self,
        auth_url: &str,
        username: &str,
        password: &str
    ) -> Result<ServerMachine<Activating>, ActivationError> {
        // ...
    }
}

impl ServerMachine<Activating> {
    /// 激活完成后转换到 Activated
    pub async fn complete_activation(
        self
    ) -> Result<ServerMachine<Activated>, ActivationError> {
        // 执行自检
        // 启动 edge-server
        // ...
    }

    /// 激活失败回到 Unbound
    pub fn fail(self, reason: ActivationError) -> ServerMachine<Unbound> {
        // ...
    }
}

impl ServerMachine<Activated> {
    /// 只有 Activated 状态可以登录
    pub async fn login(
        self,
        username: &str,
        password: &str
    ) -> Result<ServerMachine<Authenticated>, LoginError> {
        // ...
    }

    /// 获取健康状态
    pub fn health(&self) -> HealthStatus {
        // ...
    }
}

impl ServerMachine<Authenticated> {
    /// 登出回到 Activated
    pub async fn logout(self) -> ServerMachine<Activated> {
        // ...
    }

    /// 所有业务操作只在 Authenticated 状态可用
    pub fn client(&self) -> &AuthenticatedClient {
        // ...
    }
}
```

### 3.3 前端状态映射

```rust
/// 前端展示状态 (从 ServerMachine<S> 派生)
#[derive(Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AppState {
    /// 未绑定 - 需要激活
    Unbound {
        reason: UnboundReason,
        can_retry: bool,
    },

    /// 激活中
    Activating {
        progress: ActivationProgress,
    },

    /// 已激活 - 等待员工登录
    Ready {
        tenant_id: String,
        subscription: SubscriptionInfo,
    },

    /// 已认证 - 可操作
    Authenticated {
        tenant_id: String,
        employee: EmployeeInfo,
        subscription: SubscriptionInfo,
    },

    /// 订阅被阻止
    SubscriptionBlocked {
        info: SubscriptionBlockedInfo,
    },
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum UnboundReason {
    /// 首次使用
    FirstTimeSetup,
    /// 证书过期
    CertificateExpired { expired_at: String, days_overdue: i64 },
    /// 设备不匹配
    DeviceMismatch { expected: String, actual: String },
    /// 时钟篡改
    ClockTampering { direction: String, drift_seconds: i64 },
    /// 签名无效
    SignatureInvalid { error: String },
    /// 未知错误
    Unknown { error: String },
}
```

---

## 4. 实现计划

### Phase 1: 定义新类型 (shared crate)
- [ ] 定义 `AppState` 新枚举
- [ ] 定义 `UnboundReason`
- [ ] 定义 `SubscriptionInfo`
- [ ] 删除旧的 `ActivationRequiredReason`

### Phase 2: 实现 ServerMachine (red_coral)
- [ ] 创建 `core/state_machine.rs`
- [ ] 实现 `ServerMachine<Unbound>`
- [ ] 实现 `ServerMachine<Activating>`
- [ ] 实现 `ServerMachine<Activated>`
- [ ] 实现 `ServerMachine<Authenticated>`

### Phase 3: 重构 ClientBridge
- [ ] 替换 `ClientMode` 为 `ServerMachine<S>`
- [ ] 删除 `get_app_state()` 复杂逻辑
- [ ] 实现 `current_state() -> AppState` (简单映射)
- [ ] 删除 TenantManager.current_session

### Phase 4: 事件通知
- [ ] 定义 `StateChangedEvent`
- [ ] 在状态转换时发送事件
- [ ] 前端订阅事件更新 UI

### Phase 5: 清理
- [ ] 删除冗余代码
- [ ] 更新前端类型定义
- [ ] 更新测试

---

## 5. 迁移策略

### 5.1 渐进式迁移
1. 新旧状态机并存
2. 新代码使用新状态机
3. 逐步删除旧代码
4. 最后移除兼容层

### 5.2 回滚方案
- 保留旧 `get_app_state()` 实现（标记为 deprecated）
- 添加 feature flag 切换新旧实现
- 验证通过后删除旧代码

---

## 6. 类型安全保证

### 6.1 编译期检查
```rust
// ❌ 编译错误：Unbound 状态没有 login 方法
let unbound: ServerMachine<Unbound> = ...;
unbound.login("admin", "password"); // compile error!

// ✅ 正确：必须先激活
let activated = unbound.activate(...).await?.complete_activation().await?;
let authenticated = activated.login("admin", "password").await?;
```

### 6.2 运行时状态
由于 Tauri 命令需要 `&self`，我们使用枚举包装：

```rust
pub enum ServerState {
    Unbound(ServerMachine<Unbound>),
    Activating(ServerMachine<Activating>),
    Activated(ServerMachine<Activated>),
    Authenticated(ServerMachine<Authenticated>),
}

impl ServerState {
    pub fn current_app_state(&self) -> AppState {
        match self {
            ServerState::Unbound(m) => m.to_app_state(),
            ServerState::Activating(m) => m.to_app_state(),
            ServerState::Activated(m) => m.to_app_state(),
            ServerState::Authenticated(m) => m.to_app_state(),
        }
    }
}
```

---

## 7. 与 edge-server 的集成

### 7.1 激活流程简化

```rust
impl ServerMachine<Activating> {
    pub async fn complete_activation(self) -> Result<ServerMachine<Activated>, ActivationError> {
        // 1. 启动 edge-server (它内部会执行 wait_for_activation)
        let edge_handle = EdgeServer::start(&self.config).await?;

        // 2. 等待 edge-server 就绪 (通过 health check)
        edge_handle.wait_ready().await?;

        // 3. 创建 CrabClient (Local mode)
        let client = CrabClient::local()
            .with_router(edge_handle.router())
            .build()?
            .connect()
            .await?;

        // 4. 转换状态
        Ok(ServerMachine {
            config: self.config,
            _state: PhantomData,
            inner: ServerInner {
                edge_server: Some(edge_handle),
                credential: self.inner.credential,
                employee: None,
                client: Some(client),
            },
        })
    }
}
```

### 7.2 edge-server 简化

删除 edge-server 的 `is_activated()` 检查，改为：
- 启动时执行一次自检
- 自检失败则返回错误，不进入等待循环
- 由调用方 (red_coral) 决定如何处理失败

---

## 8. 预期收益

1. **类型安全**：编译期防止非法状态转换
2. **单一事实来源**：不再有状态不同步问题
3. **代码简化**：删除 400+ 行复杂检测逻辑
4. **可测试性**：每个状态独立测试
5. **可维护性**：状态转换清晰可见
