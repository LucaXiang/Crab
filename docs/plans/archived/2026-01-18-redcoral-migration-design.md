# RedCoral 迁移到 Edge Server 设计方案

## 概述

将 red_coral POS 系统迁移到 edge-server 架构，支持两种运行模式：
- **Server 模式**: 本地运行 edge-server，使用 In-Process 通信
- **Client 模式**: 连接远程 edge-server，使用 mTLS 通信

## 设计决策

| 决策点 | 选择 |
|--------|------|
| 模式切换时机 | 运行时切换 |
| 前端通信方式 | Tauri Commands (mTLS 只能走 Rust) |
| API 设计策略 | 重新设计，基于 edge-server 数据模型 |
| 业务类型定义 | 在 shared crate 中定义 |
| 第一阶段目标 | 租户登录、员工登录、缓存机制 |
| 多租户支持 | 一台设备可存储多个租户证书 |
| 缓存策略 | JWT Token + 员工信息 + 离线登录 |
| 证书目录结构 | 按 tenant_id 分目录 |

## 整体架构

```
┌─────────────────────────────────────────────────────────────────────┐
│                         red_coral (Tauri App)                        │
├─────────────────────────────────────────────────────────────────────┤
│                        Frontend (React + TS)                         │
│   ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────┐               │
│   │ Login   │  │ Tenant  │  │ Settings│  │  POS    │               │
│   │ Screen  │  │ Select  │  │  Mode   │  │  Main   │               │
│   └────┬────┘  └────┬────┘  └────┬────┘  └────┬────┘               │
│        └────────────┴────────────┴────────────┘                     │
│                           │ Tauri Commands                          │
├───────────────────────────┼─────────────────────────────────────────┤
│                     Rust Backend (src-tauri)                         │
│   ┌─────────────────────────────────────────────────────────────┐   │
│   │                    ClientBridge                              │   │
│   │  ┌─────────────────────────────────────────────────────┐    │   │
│   │  │              TenantManager                           │    │   │
│   │  │  - 管理多租户证书目录                                  │    │   │
│   │  │  - 切换当前活跃租户                                    │    │   │
│   │  │  - 缓存员工登录状态                                    │    │   │
│   │  └─────────────────────────────────────────────────────┘    │   │
│   │                           │                                  │   │
│   │  ┌────────────────────────┴────────────────────────┐        │   │
│   │  │                  ClientMode (枚举)               │        │   │
│   │  ├─────────────────────┬───────────────────────────┤        │   │
│   │  │   Server 模式        │      Client 模式           │        │   │
│   │  │   (In-Process)      │      (Remote)             │        │   │
│   │  │                     │                           │        │   │
│   │  │  edge-server lib    │     crab-client           │        │   │
│   │  │  + CrabClient<Local>│     CrabClient<Remote>    │        │   │
│   │  │        ↓            │           ↓               │        │   │
│   │  │   本地 SurrealDB     │     远程 Edge Server       │        │   │
│   │  └─────────────────────┴───────────────────────────┘        │   │
│   └─────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────┘
```

## 多租户存储结构

```
~/.red_coral/
├── config.json                         # 全局配置
│   {
│     "current_tenant": "tenant_001",
│     "current_mode": "client",
│     "server_config": { ... },
│     "known_tenants": ["tenant_001", "tenant_002"]
│   }
│
├── tenants/
│   ├── tenant_001/
│   │   ├── credential.json             # 租户凭证
│   │   ├── entity.crt                  # 客户端证书
│   │   ├── entity.key                  # 客户端私钥
│   │   ├── tenant_ca.crt               # 租户 CA 证书
│   │   └── session_cache.json          # 员工会话缓存
│   │       {
│   │         "employees": {
│   │           "cashier1": {
│   │             "password_hash": "...",
│   │             "cached_token": "...",
│   │             "token_expires_at": 1234,
│   │             "user_info": { ... },
│   │             "last_online_login": 1234
│   │           }
│   │         }
│   │       }
│   │
│   └── tenant_002/
│       └── ...
│
└── server_data/
    └── surreal.db
```

## TenantManager API

```rust
pub struct TenantManager {
    base_path: PathBuf,
    current_tenant: Option<String>,
    cert_managers: HashMap<String, CertManager>,
    session_caches: HashMap<String, SessionCache>,
}

impl TenantManager {
    // 租户管理
    pub fn list_tenants(&self) -> Vec<TenantInfo>;
    pub async fn activate_tenant(&mut self, auth_url: &str, username: &str, password: &str) -> Result<String, TenantError>;
    pub fn switch_tenant(&mut self, tenant_id: &str) -> Result<(), TenantError>;
    pub fn remove_tenant(&mut self, tenant_id: &str) -> Result<(), TenantError>;

    // 员工登录
    pub async fn login_online(&mut self, username: &str, password: &str) -> Result<EmployeeSession, TenantError>;
    pub fn login_offline(&mut self, username: &str, password: &str) -> Result<EmployeeSession, TenantError>;
    pub async fn login_auto(&mut self, username: &str, password: &str) -> Result<EmployeeSession, TenantError>;
    pub fn logout(&mut self) -> Result<(), TenantError>;

    // 状态查询
    pub fn current_tenant_id(&self) -> Option<&str>;
    pub fn current_session(&self) -> Option<&EmployeeSession>;
    pub fn has_offline_cache(&self, username: &str) -> bool;
}

pub struct EmployeeSession {
    pub username: String,
    pub token: String,
    pub user_info: UserInfo,
    pub login_mode: LoginMode,  // Online | Offline
    pub expires_at: Option<u64>,
}
```

## ClientBridge 与模式切换

```rust
pub enum ClientMode {
    Server {
        server_state: Arc<ServerState>,
        client: CrabClient<Local, Authenticated>,
    },
    Client {
        client: CrabClient<Remote, Authenticated>,
        edge_url: String,
    },
}

pub struct ClientBridge {
    tenant_manager: TenantManager,
    mode: Option<ClientMode>,
    config: AppConfig,
}

impl ClientBridge {
    // 模式管理
    pub async fn start_server_mode(&mut self) -> Result<(), BridgeError>;
    pub async fn start_client_mode(&mut self, edge_url: &str, message_addr: &str) -> Result<(), BridgeError>;
    pub async fn switch_mode(&mut self, new_mode: ModeType) -> Result<(), BridgeError>;
    pub fn current_mode(&self) -> Option<ModeType>;

    // 统一业务 API
    pub async fn get_products(&self) -> Result<Vec<Product>, BridgeError>;
    pub async fn create_order(&self, order: NewOrder) -> Result<Order, BridgeError>;
}
```

## 启动流程

```
App Start
    │
    ▼
┌───────────────┐
│ 加载 config   │
└───────┬───────┘
        │
        ▼
┌───────────────────┐    无租户     ┌──────────────────┐
│ 检查已激活租户数量 │─────────────→│  设备激活页面     │
└───────┬───────────┘              └────────┬─────────┘
        │ 有租户                            │
        ▼                                   │
┌───────────────────┐                       │
│  租户选择页面      │←──────────────────────┘
└───────┬───────────┘
        │
        ▼
┌───────────────────┐
│  模式选择         │
│  Server / Client  │
└───────┬───────────┘
        │
┌───────┴───────┐
│               │
▼               ▼
Server        Client
模式           模式
│               │
└───────┬───────┘
        │
        ▼
┌───────────────────┐
│   员工登录页面     │
└───────┬───────────┘
        │
        ▼
┌───────────────────┐
│    POS 主界面     │
└───────────────────┘
```

## 文件结构

**Rust 端：**
```
red_coral/src-tauri/src/
├── core/
│   ├── mod.rs
│   ├── tenant_manager.rs
│   ├── client_bridge.rs
│   ├── session_cache.rs
│   └── config.rs
├── api/
│   ├── mod.rs
│   ├── tenant.rs
│   ├── auth.rs
│   └── mode.rs
└── lib.rs
```

**TypeScript 端：**
```
red_coral/src/
├── screens/
│   ├── Activate/
│   ├── TenantSelect/
│   └── Login/
├── infrastructure/
│   └── api/
│       ├── tenant.ts
│       ├── auth.ts
│       └── mode.ts
└── core/
    └── stores/
        └── app/
            └── useAppStore.ts
```

## 依赖配置

```toml
[dependencies]
shared = { path = "../../shared" }
crab-client = { path = "../../crab-client", features = ["in-process"] }
edge-server = { path = "../../edge-server" }
```

## 第一阶段实现步骤

| 步骤 | 任务 | 涉及文件 |
|------|------|----------|
| 1 | 将 red_coral 加入 crab workspace | `Cargo.toml` |
| 2 | 创建 TenantManager 基础结构 | `src-tauri/src/core/tenant_manager.rs` |
| 3 | 创建 SessionCache 结构 | `src-tauri/src/core/session_cache.rs` |
| 4 | 创建 ClientBridge 骨架 | `src-tauri/src/core/client_bridge.rs` |
| 5 | 实现租户激活流程 | `api/tenant.rs` + 前端 |
| 6 | 实现员工登录 (在线/离线) | `api/auth.rs` + 前端 |
| 7 | 实现模式切换 | `api/mode.rs` + 前端 |
| 8 | 前端页面: 激活、租户选择、登录 | `screens/` |

## 验收标准

- [ ] 可以激活设备到某个租户
- [ ] 可以在多个租户间切换
- [ ] 可以用员工账号登录（在线）
- [ ] 断网后可以离线登录（使用缓存）
- [ ] 可以在 Server/Client 模式间切换
