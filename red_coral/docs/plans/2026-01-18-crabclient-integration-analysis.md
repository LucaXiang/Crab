# CrabClient Integration Analysis Report

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 分析现有 CrabClient 与 EdgeServer 的 API 对接现状，评估租户登录、员工登录、证书管理、启动流程和看门狗等功能

**Architecture:** Tauri 应用通过 ClientBridge 统一管理 Server/Client 双模式，使用 CrabClient typestate 模式确保编译期安全

**Tech Stack:** Rust, Tauri 2.9, CrabClient, EdgeServer, mTLS, SurrealDB

---

## 一、现有架构总览

### 1.1 目录结构

```
red_coral/src-tauri/src/
├── lib.rs                    # Tauri 入口，命令注册，启动流程
├── events.rs                 # Tauri 事件定义 (ServerMessageEvent)
├── core/
│   ├── mod.rs               # 核心模块导出
│   ├── client_bridge.rs     # ClientBridge - 双模式统一桥接层
│   ├── tenant_manager.rs    # TenantManager - 多租户证书管理
│   ├── session_cache.rs     # SessionCache - 员工离线登录缓存
│   └── connection_monitor.rs # ConnectionMonitor - 连接状态监控
└── commands/
    ├── mod.rs               # 命令模块导出
    ├── mode.rs              # 模式管理命令 (start_server_mode, start_client_mode 等)
    ├── auth.rs              # 认证命令 (login_employee, logout_employee 等)
    ├── tenant.rs            # 租户管理命令 (activate_tenant, switch_tenant 等)
    ├── api.rs               # 通用 API 代理命令 (api_get, api_post 等)
    └── ...                  # 业务命令 (data, orders, location, system)
```

### 1.2 双模式架构

```
┌─────────────────────────────────────────────────────────────┐
│                    Tauri Frontend (React)                    │
│   useBridgeStore → invoke("command") → Tauri Commands       │
└────────────────────────────┬────────────────────────────────┘
                             │
┌────────────────────────────▼────────────────────────────────┐
│                      ClientBridge                            │
│   Arc<RwLock<ClientBridge>> managed by Tauri State          │
└─────────────┬───────────────────────────────┬───────────────┘
              │                               │
     ┌────────▼────────┐             ┌────────▼────────┐
     │   Server Mode   │             │   Client Mode   │
     │                 │             │                 │
     │ CrabClient<Local>│           │CrabClient<Remote>│
     │  + ServerState  │             │   + mTLS TCP    │
     │  + In-Process   │             │   + Message Bus │
     └─────────────────┘             └─────────────────┘
              │                               │
     ┌────────▼────────────────────────────────▼──────────┐
     │                  EdgeServer                        │
     │  HTTP API (Axum) + Message Bus (TCP/TLS)          │
     │  + SurrealDB + JWT Auth + Background Tasks        │
     └────────────────────────────────────────────────────┘
```

---

## 二、功能分析

### 2.1 租户登录（设备激活）

**位置:** `tenant_manager.rs:146` - `activate_device()`

**流程:**
1. 生成 Hardware ID (`crab_cert::generate_hardware_id()`)
2. 调用 Auth Server `/api/server/activate` 接口
3. 解析响应获取证书链 (root_ca, tenant_ca, entity_cert, entity_key)
4. 保存证书到租户目录 `tenants/{tenant_id}/certs/`
5. 保存 Credential.json 到 `tenants/{tenant_id}/auth_storage/`
6. 自动切换到该租户

**现状评估:**
- ✅ 证书获取和保存逻辑完整
- ✅ 支持多租户目录结构
- ⚠️ TenantManager 使用 reqwest 直接请求，未使用 CrabClient
- ⚠️ 激活后未自动启动 Server 模式

**Tauri 命令:**
- `activate_tenant(auth_url, username, password)` → 调用 `ClientBridge::handle_activation()`

---

### 2.2 员工登录

**位置:** `client_bridge.rs:707` - `login_employee()`

**流程:**

**Server 模式:**
1. 从 `LocalClientState` 取出 `CrabClient<Local, Connected>`
2. 调用 `connected.login(username, password).await`
3. 获取 `user_info` 和 `token`
4. 状态转换为 `LocalClientState::Authenticated`

**Client 模式:**
1. 从 `RemoteClientState` 取出 `CrabClient<Remote, Connected>`
2. 调用 `connected.login(username, password).await` (使用 mTLS HTTPS)
3. 获取 `user_info` 和 `token`
4. 状态转换为 `RemoteClientState::Authenticated`

**现状评估:**
- ✅ 使用 CrabClient typestate 模式，编译期安全
- ✅ Server/Client 双模式统一接口
- ✅ 会话信息正确保存到 `EmployeeSession`
- ⚠️ 离线登录 (`login_offline`) 仅在 TenantManager 中实现，未整合到 ClientBridge

**Tauri 命令:**
- `login_employee(username, password)` → 统一入口，自动选择模式
- `login_online/offline/auto()` → TenantManager 方法（旧接口）

---

### 2.3 证书管理

**位置:** `crab-client/src/cert/manager.rs` - `CertManager`

**功能:**
- `has_local_certificates()` - 检查本地证书是否存在
- `load_local_certificates()` - 加载本地证书 (cert.pem, key.pem, ca.pem)
- `save_certificates()` - 保存证书
- `build_mtls_http_client()` - 构建 mTLS HTTP 客户端
- `self_check()` - 自检（证书链、硬件绑定、时钟篡改）
- `refresh_credential_timestamp()` - 刷新凭证时间戳

**现状评估:**
- ✅ 证书管理功能完整
- ✅ 支持 mTLS 客户端构建
- ✅ 支持自检和时钟篡改检测
- ✅ TenantManager 正确使用 CertManager

---

### 2.4 启动流程

**位置:** `lib.rs:39` - `run()`

**流程:**
1. 初始化 rustls crypto provider
2. 初始化日志系统 (file + stdout)
3. 设置 panic hook
4. 创建数据目录 `~/Library/Application Support/com.xzy.pos/redcoral/`
5. 创建 ClientBridge (with AppHandle for events)
6. 异步恢复上次会话 (`restore_last_session()`)
7. 启动 ConnectionMonitor 后台任务
8. 注册 Tauri 命令

**`restore_last_session()` 流程 (client_bridge.rs:317):**
1. 读取 `config.json` 获取 `current_mode` 和 `current_tenant`
2. 恢复租户选择 (`switch_tenant`)
3. 根据模式启动:
   - Server 模式: `start_server_mode()`
   - Client 模式: `start_client_mode(edge_url, message_addr)`

**现状评估:**
- ✅ 启动流程完整，自动恢复会话
- ✅ 后台任务正确启动
- ⚠️ 首次启动时未自动进入 Setup 流程
- ⚠️ 恢复失败时未通知前端

---

### 2.5 看门狗 (ConnectionMonitor)

**位置:** `connection_monitor.rs:17` - `ConnectionMonitor`

**功能:**
- 每 30 秒检查一次连接状态
- 仅在 Client 模式下执行检查
- 发送 `connection-status` Tauri 事件

**现状评估:**
- ✅ 基础监控逻辑实现
- ⚠️ 仅检查状态，未实现自动重连
- ⚠️ TODO 注释表明重连逻辑待实现

---

### 2.6 消息总线订阅

**位置:** `client_bridge.rs:497-522` (Server) / `client_bridge.rs:609-637` (Client)

**功能:**
- Server 模式: 订阅 `message_bus.subscribe()` 并转发到 Tauri 事件
- Client 模式: 订阅 `NetworkMessageClient.subscribe()` 并转发到 Tauri 事件
- 事件名: `server-message`
- 事件格式: `ServerMessageEvent { event_type, payload, correlation_id }`

**现状评估:**
- ✅ 消息转发逻辑完整
- ✅ 前端可通过 `listen("server-message")` 接收
- ✅ 支持 Notification, Sync, ServerCommand 等类型

---

## 三、API 对接现状

### 3.1 统一 API 方法

**位置:** `client_bridge.rs:925-1159`

ClientBridge 提供了统一的 HTTP 方法:
- `get<T>(path)` - GET 请求
- `post<T, B>(path, body)` - POST 请求
- `put<T, B>(path, body)` - PUT 请求
- `delete<T>(path)` - DELETE 请求
- `delete_with_body<T, B>(path, body)` - DELETE with body

**实现逻辑:**
- Server 模式: 调用 `CrabClient<Local, Authenticated>` 的方法
- Client 模式: 使用 `edge_http_client` (mTLS reqwest) 发送请求

**现状评估:**
- ✅ 统一接口，屏蔽模式差异
- ✅ 自动附加 Authorization header
- ⚠️ Client 模式未使用 CrabClient 的 HTTP 方法（直接用 reqwest）

### 3.2 Tauri 命令对接

**位置:** `commands/api.rs`

提供通用 API 代理:
- `api_get(path)` → `bridge.get(path)`
- `api_post(path, body)` → `bridge.post(path, body)`
- `api_put(path, body)` → `bridge.put(path, body)`
- `api_delete(path)` → `bridge.delete(path)`

---

## 四、待完善事项

### 4.1 高优先级

| 编号 | 问题 | 位置 | 影响 |
|------|------|------|------|
| H1 | ConnectionMonitor 未实现自动重连 | connection_monitor.rs:67 | Client 模式断线后无法自动恢复 |
| H2 | Client 模式使用 reqwest 而非 CrabClient | client_bridge.rs:942-963 | 代码重复，不统一 |
| H3 | 离线登录未整合到 ClientBridge | auth.rs | 需要单独调用 TenantManager |

### 4.2 中优先级

| 编号 | 问题 | 位置 | 影响 |
|------|------|------|------|
| M1 | 恢复会话失败时未通知前端 | lib.rs:114 | 用户无感知 |
| M2 | 激活后未自动启动 Server 模式 | tenant.rs | 需要手动操作 |
| M3 | get_app_config 返回默认值 | mode.rs:137 | 前端显示不准确 |

### 4.3 低优先级

| 编号 | 问题 | 位置 | 影响 |
|------|------|------|------|
| L1 | TenantInfo 缺少 tenant_name | tenant_manager.rs:132 | 前端显示不友好 |
| L2 | SessionCache 未持久化到磁盘 | session_cache.rs | 应用重启后离线缓存丢失 |

---

## 五、CrabClient 接口参考

### 5.1 Remote 模式 Typestate 流程

```
CrabClient<Remote, Disconnected>
    │
    ├─ setup(tenant_user, tenant_pass, message_addr)
    │   → Downloads certificates from Auth Server
    │   → Connects to Message Bus via mTLS
    │
    └─ reconnect(message_addr)
        → Uses cached certificates
        → Runs self_check()
        → Connects to Message Bus via mTLS
              │
              ▼
CrabClient<Remote, Connected>
    │
    ├─ request(&BusMessage) → RPC (no login needed)
    ├─ message_client() → &NetworkMessageClient
    ├─ edge_http_client() → &reqwest::Client (mTLS)
    │
    └─ login(username, password)
              │
              ▼
CrabClient<Remote, Authenticated>
    │
    ├─ me() → &UserInfo
    ├─ token() → &str
    ├─ request(&BusMessage) → RPC
    │
    ├─ logout() → CrabClient<Remote, Connected>
    └─ disconnect() → CrabClient<Remote, Disconnected>
```

### 5.2 Local 模式 Typestate 流程

```
CrabClient<Local, Disconnected>
    │
    └─ connect()
        → Verifies router and message channels configured
              │
              ▼
CrabClient<Local, Connected>
    │
    ├─ subscribe() → Receiver<BusMessage>
    │
    └─ login(username, password)
        → In-Process HTTP call to /api/auth/login
              │
              ▼
CrabClient<Local, Authenticated>
    │
    ├─ me() → &UserInfo
    ├─ token() → &str
    ├─ get/post/put/delete() → In-Process HTTP
    ├─ request(&BusMessage) → In-Process RPC
    │
    ├─ logout() → CrabClient<Local, Connected>
    └─ disconnect() → CrabClient<Local, Disconnected>
```

---

## 六、总结

### 现状

1. **架构设计良好**: ClientBridge 正确抽象了 Server/Client 双模式
2. **CrabClient 集成完整**: typestate 模式确保编译期安全
3. **租户管理完善**: 多租户证书隔离存储
4. **员工认证完整**: 支持在线/离线登录
5. **消息总线对接**: Tauri 事件正确转发

### 主要差距

1. **自动重连未实现**: ConnectionMonitor 仅监控，不恢复
2. **API 实现不统一**: Client 模式部分直接用 reqwest
3. **配置恢复不完整**: 部分字段返回默认值
4. **离线登录未统一**: 需要单独调用 TenantManager

### 建议优先级

1. 实现 ConnectionMonitor 自动重连
2. 统一 Client 模式 API 调用
3. 完善配置恢复逻辑
4. 整合离线登录到 ClientBridge
