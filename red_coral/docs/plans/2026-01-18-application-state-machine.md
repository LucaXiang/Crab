# 应用状态机设计方案

> **参考**:
> - Server 模式: `edge-server/src/core/server.rs` + `examples/interactive_demo.rs`
> - Client 模式: `crab-client/examples/message_client.rs`

## 核心原则

1. **Server 模式 = 完整生命周期管理** (激活、订阅验证、证书、员工认证)
2. **Client 模式 = 遥控器** (连接、员工认证；订阅由远程 Server 验证)
3. **订阅无效 = 阻止使用** (Server 模式下)

---

## 一、Server 模式状态机

### 状态定义

```
┌─────────────────────────────────────────────────────────────────┐
│                     Server Mode State Machine                    │
└─────────────────────────────────────────────────────────────────┘

    ┌───────────┐
    │  Inactive │  ← 初始状态 (无租户/无证书)
    └─────┬─────┘
          │ 检测到租户目录 + 证书
          ▼
    ┌────────────────┐
    │ NeedActivation │  ← 有证书但未激活/自检失败
    └───────┬────────┘
            │ /activate <user> <pass>
            │ ProvisioningService.activate()
            ▼
    ┌────────────────┐
    │   Activating   │  ← 正在激活 (下载证书、验证签名)
    └───────┬────────┘
            │ 激活成功
            ▼
    ┌────────────────┐
    │   Activated    │  ← 证书有效、硬件绑定通过
    └───────┬────────┘
            │ 验证订阅状态
            ▼
    ┌────────────────────────────────┐
    │        SubscriptionCheck       │
    │  Active/Trial → continue       │
    │  Canceled/Unpaid → ❌ BLOCKED  │
    └───────┬────────────────────────┘
            │ 订阅有效
            ▼
    ┌──────────────┐
    │    Ready     │  ← 服务器运行中，等待员工登录
    └──────┬───────┘
           │ /login <user> <pass>
           ▼
    ┌─────────────────────┐
    │ EmployeeAuthenticated│  ← 可以使用 POS 功能
    └─────────────────────┘
```

### 状态说明

| 状态 | 条件 | 前端路由 |
|------|------|----------|
| `Inactive` | 无租户或无证书 | `/setup` (租户激活) |
| `NeedActivation` | 证书存在但自检失败 | `/setup` (重新激活) |
| `Activating` | 正在下载证书/激活 | `/setup` (显示进度) |
| `Activated` | 证书有效，待验证订阅 | `/setup` (显示状态) |
| `SubscriptionBlocked` | 订阅无效 (Canceled/Unpaid) | `/blocked` (提示续费) |
| `Ready` | 服务器就绪，等待员工 | `/login` |
| `EmployeeAuthenticated` | 员工已登录 | `/pos` |

### 关键代码映射

```rust
// edge-server/src/core/server.rs - Server::run()

loop {
    // 1. 等待激活 (阻塞直到 is_activated == true)
    state.wait_for_activation().await;  // 包含完整自检

    // 2. 加载 TLS
    let tls_config = state.load_tls_config()?;

    // 3. 启动服务
    // ... HTTP + TCP
}
```

```rust
// wait_for_activation 内部 (ActivationService)
// - 加载 credential.json
// - 验证证书链
// - 验证硬件绑定 (device_id == generate_hardware_id())
// - 验证签名 (binding.signature)
// - 检查订阅状态 (subscription.status)
```

---

## 二、Client 模式状态机

### 状态定义

```
┌─────────────────────────────────────────────────────────────────┐
│                     Client Mode State Machine                    │
│                         (遥控器模式)                             │
└─────────────────────────────────────────────────────────────────┘

    ┌──────────────┐
    │ Disconnected │  ← 初始状态
    └──────┬───────┘
           │
           ├─── 无缓存证书 ──→ /setup <user> <pass>
           │                   CrabClient::setup()
           │                   ↓
           │              下载证书到本地
           │                   ↓
           │              连接 Message Bus
           │
           ├─── 有缓存证书 ──→ /reconnect
           │                   CrabClient::reconnect()
           │                   ↓
           │              self_check() 验证
           │                   ↓
           │              连接 Message Bus
           │
           ▼
    ┌───────────┐
    │ Connected │  ← mTLS 连接建立
    └─────┬─────┘
          │ /login <user> <pass>
          │ CrabClient::login()
          ▼
    ┌───────────────┐
    │ Authenticated │  ← 获取 JWT Token
    └───────────────┘
          │
          │ 可以使用 HTTP API
          ▼
    ┌─────────────────────┐
    │ EmployeeAuthenticated│  ← 可以使用 POS 功能
    └─────────────────────┘
```

### 状态说明

| 状态 | 条件 | 前端路由 |
|------|------|----------|
| `Disconnected` | 未连接 | `/connect` (选择服务器) |
| `NeedSetup` | 无缓存证书 | `/setup` (输入凭证) |
| `Connected` | mTLS 连接建立 | `/login` |
| `Authenticated` | 员工已登录 | `/pos` |

### 关键代码映射

```rust
// crab-client/examples/message_client.rs

// 首次设置
let client = CrabClient::remote()
    .auth_server(AUTH_SERVER)
    .edge_server(EDGE_HTTPS)
    .cert_path(CERT_PATH)
    .client_name(CLIENT_NAME)
    .build()?;

// 方式1: 首次设置 (下载证书)
let connected = client.setup(username, password, MESSAGE_ADDR).await?;

// 方式2: 重连 (使用缓存证书 + self_check)
let connected = client.reconnect(MESSAGE_ADDR).await?;

// 员工登录
let authenticated = connected.login(username, password).await?;
```

---

## 三、ClientBridge 统一状态机

### 状态枚举设计

```rust
/// 应用状态 (统一 Server/Client 模式)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AppState {
    // === 通用状态 ===
    /// 未初始化
    Uninitialized,

    // === Server 模式专属 ===
    /// Server: 无租户
    ServerNoTenant,
    /// Server: 需要激活 (有证书目录但自检失败)
    ServerNeedActivation,
    /// Server: 正在激活
    ServerActivating,
    /// Server: 已激活，验证订阅中
    ServerCheckingSubscription,
    /// Server: 订阅无效，阻止使用
    ServerSubscriptionBlocked { reason: String },
    /// Server: 服务器就绪
    ServerReady,
    /// Server: 员工已登录
    ServerAuthenticated,

    // === Client 模式专属 ===
    /// Client: 未连接
    ClientDisconnected,
    /// Client: 需要设置 (无缓存证书)
    ClientNeedSetup,
    /// Client: 正在连接
    ClientConnecting,
    /// Client: 已连接
    ClientConnected,
    /// Client: 员工已登录
    ClientAuthenticated,
}
```

### 状态转换图

```
                    ┌───────────────────┐
                    │   Uninitialized   │
                    └─────────┬─────────┘
                              │
                    ┌─────────┴─────────┐
                    │                   │
                    ▼                   ▼
┌───────────────────────────┐   ┌───────────────────────────┐
│     SERVER MODE PATH      │   │     CLIENT MODE PATH      │
└───────────────────────────┘   └───────────────────────────┘
            │                               │
            ▼                               ▼
    ┌───────────────┐               ┌─────────────────┐
    │ ServerNoTenant│               │ClientDisconnected│
    └───────┬───────┘               └────────┬────────┘
            │ 激活租户                         │
            ▼                               │
    ┌────────────────────┐                  │
    │ServerNeedActivation│                  ▼
    └────────┬───────────┘          ┌───────────────┐
             │ activate()           │ClientNeedSetup│ ← 无证书
             ▼                      └───────┬───────┘
    ┌────────────────┐                      │ setup()
    │ServerActivating│                      │
    └────────┬───────┘                      │
             │ 成功                          │
             ▼                              │
    ┌──────────────────────────┐            │
    │ServerCheckingSubscription│            │
    └────────┬─────────────────┘            │
             │                              │
     ┌───────┴───────┐                      │
     │               │                      │
     ▼               ▼                      ▼
┌─────────────┐ ┌───────────────────────┐ ┌───────────────┐
│ ServerReady │ │ServerSubscriptionBlocked│ │ClientConnected│
└──────┬──────┘ └───────────────────────┘ └───────┬───────┘
       │                                          │
       │ login()                                  │ login()
       ▼                                          ▼
┌─────────────────────┐               ┌─────────────────────┐
│ ServerAuthenticated │               │ ClientAuthenticated │
└─────────────────────┘               └─────────────────────┘
```

---

## 四、前端路由守卫

### 路由映射

```typescript
// App.tsx 路由配置

const routes = {
  // 通用
  '/':       'InitialRoute',      // 检查状态，重定向

  // Setup 流程
  '/setup':  'SetupScreen',       // 激活/设置
  '/blocked': 'BlockedScreen',    // 订阅阻止

  // 认证流程
  '/login':  'LoginScreen',       // 员工登录

  // 主功能
  '/pos':    'POSScreen',         // POS 主界面 (需要认证)
};
```

### 状态 → 路由映射

```typescript
function getRouteForState(state: AppState): string {
  switch (state) {
    // Server Mode
    case 'ServerNoTenant':
    case 'ServerNeedActivation':
    case 'ServerActivating':
    case 'ServerCheckingSubscription':
      return '/setup';

    case 'ServerSubscriptionBlocked':
      return '/blocked';

    case 'ServerReady':
      return '/login';

    case 'ServerAuthenticated':
      return '/pos';

    // Client Mode
    case 'ClientDisconnected':
    case 'ClientNeedSetup':
    case 'ClientConnecting':
      return '/setup';

    case 'ClientConnected':
      return '/login';

    case 'ClientAuthenticated':
      return '/pos';

    default:
      return '/setup';
  }
}
```

### 路由守卫实现

```typescript
// ProtectedRoute.tsx
function ProtectedRoute({ children, requiredState }) {
  const { appState, isLoading } = useBridgeStore();

  if (isLoading) {
    return <LoadingScreen />;
  }

  const allowedStates = Array.isArray(requiredState)
    ? requiredState
    : [requiredState];

  if (!allowedStates.includes(appState)) {
    const redirectTo = getRouteForState(appState);
    return <Navigate to={redirectTo} replace />;
  }

  return children;
}

// 使用示例
<Route
  path="/pos"
  element={
    <ProtectedRoute requiredState={['ServerAuthenticated', 'ClientAuthenticated']}>
      <POSScreen />
    </ProtectedRoute>
  }
/>
```

---

## 五、Tauri Command 设计

### 状态查询

```rust
#[tauri::command]
pub async fn get_app_state(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
) -> Result<AppState, String> {
    let bridge = bridge.read().await;
    bridge.get_app_state().await.map_err(|e| e.to_string())
}
```

### 状态转换命令

```rust
// Server Mode
#[tauri::command]
pub async fn activate_tenant(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    auth_url: String,
    username: String,
    password: String,
) -> Result<String, String>;

#[tauri::command]
pub async fn start_server_mode(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
) -> Result<(), String>;

// Client Mode
#[tauri::command]
pub async fn setup_client(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    auth_url: String,
    edge_url: String,
    message_addr: String,
    username: String,
    password: String,
) -> Result<(), String>;

#[tauri::command]
pub async fn reconnect_client(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
) -> Result<(), String>;

// 通用
#[tauri::command]
pub async fn login_employee(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    username: String,
    password: String,
) -> Result<EmployeeSession, String>;
```

---

## 六、实现优先级

### Phase 1: 基础状态机
1. 定义 `AppState` 枚举
2. 实现 `ClientBridge::get_app_state()`
3. 添加 `get_app_state` Tauri command

### Phase 2: Server 模式完整流程
1. 集成 `wait_for_activation()` 逻辑
2. 添加订阅验证 (SubscriptionBlocked 状态)
3. 前端 Setup/Blocked 页面

### Phase 3: Client 模式完整流程
1. 实现 `setup_client` (首次设置)
2. 实现 `reconnect_client` (重连)
3. 前端 Connect 页面

### Phase 4: 前端路由守卫
1. 实现 `ProtectedRoute` 组件
2. 实现 `InitialRoute` 自动重定向
3. 状态变化监听 + 自动导航

---

## 七、测试场景

| 场景 | 预期行为 |
|------|----------|
| 首次启动 (Server) | → /setup → 激活 → /login |
| 首次启动 (Client) | → /setup → setup_client → /login |
| 订阅过期 (Server) | → /blocked |
| 证书损坏 (Server) | → /setup (NeedActivation) |
| 断网重连 (Client) | reconnect → /login |
| Token 过期 | → /login (自动 logout) |
