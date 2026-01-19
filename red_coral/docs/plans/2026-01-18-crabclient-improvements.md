# CrabClient Integration Improvements Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 完善 CrabClient 与 Tauri 的集成，实现自动重连、统一 API 调用、离线登录整合

**Architecture:** 基于现有 ClientBridge 架构，增强健壮性和功能完整性

**Tech Stack:** Rust, Tauri 2.9, CrabClient, tokio, mTLS

---

## 任务概览

| 任务 | 优先级 | 预估复杂度 |
|------|--------|------------|
| Task 0: 规范化工作目录结构 | 高 | 中 |
| Task 1: 实现 ConnectionMonitor 自动重连 | 高 | 中 |
| Task 2: 统一 Client 模式 API 调用 | 中 | 低 |
| Task 3: 整合离线登录到 ClientBridge | 中 | 中 |
| Task 4: 完善配置恢复逻辑 | 低 | 低 |
| Task 5: 添加连接状态 Hook | 低 | 低 |

---

## Task 0: 规范化工作目录结构

**问题分析:**

当前 edge-server 和 red_coral 的工作目录结构混乱，数据库、证书、配置文件散落在根目录，缺乏统一规范。

**当前结构 (edge-server):**
```
work_dir/
├── crab.db/           # SurrealDB RocksDB 目录 (直接在根目录)
├── auth_storage/      # 激活凭证
│   └── Credential.json
└── certs/             # 证书目录
    ├── root_ca.pem
    ├── tenant_ca.pem
    ├── edge_cert.pem
    └── edge_key.pem
```

**当前结构 (red_coral Tauri):**
```
~/Library/Application Support/com.xzy.pos/redcoral/
├── logs/              # 日志 (独立于 work_dir)
├── tenants/{tenant_id}/
│   ├── certs/
│   └── auth_storage/
└── config.json        # Tauri 配置
```

**目标结构:**
```
work_dir/
├── certs/             # 证书目录
│   ├── root_ca.pem
│   ├── tenant_ca.pem
│   ├── edge_cert.pem
│   └── edge_key.pem
├── database/          # 数据库目录
│   └── crab.db/       # SurrealDB RocksDB
├── logs/              # 日志目录
│   └── edge-server.log
├── auth_storage/      # 激活凭证
│   └── Credential.json
└── config/            # 配置目录 (可选)
    └── settings.json
```

**Files:**
- Modify: `edge-server/src/core/state.rs:100`
- Modify: `edge-server/src/db/mod.rs:27`
- Test: `cargo test -p edge-server`

### Step 1: 修改数据库路径

在 `edge-server/src/core/state.rs` 修改数据库路径:

```rust
// 修改 state.rs 中的 initialize 方法
pub async fn initialize(config: &Config) -> Self {
    // 1. Initialize DB
    // Use work_dir/database/crab.db for database path
    let db_dir = PathBuf::from(&config.work_dir).join("database");
    std::fs::create_dir_all(&db_dir).expect("Failed to create database directory");

    let db_path = db_dir.join("crab.db");
    let db_path_str = db_path.to_string_lossy();

    let db_service = DbService::new(&db_path_str)
        .await
        .expect("Failed to initialize database");
    // ... rest unchanged
}
```

### Step 2: 添加目录初始化辅助函数

在 `edge-server/src/core/config.rs` 添加:

```rust
impl Config {
    /// 确保工作目录结构存在
    pub fn ensure_work_dir_structure(&self) -> std::io::Result<()> {
        let base = PathBuf::from(&self.work_dir);
        std::fs::create_dir_all(base.join("certs"))?;
        std::fs::create_dir_all(base.join("database"))?;
        std::fs::create_dir_all(base.join("logs"))?;
        std::fs::create_dir_all(base.join("auth_storage"))?;
        Ok(())
    }

    /// 获取证书目录路径
    pub fn certs_dir(&self) -> PathBuf {
        PathBuf::from(&self.work_dir).join("certs")
    }

    /// 获取数据库目录路径
    pub fn database_dir(&self) -> PathBuf {
        PathBuf::from(&self.work_dir).join("database")
    }

    /// 获取日志目录路径
    pub fn logs_dir(&self) -> PathBuf {
        PathBuf::from(&self.work_dir).join("logs")
    }

    /// 获取认证存储目录路径
    pub fn auth_storage_dir(&self) -> PathBuf {
        PathBuf::from(&self.work_dir).join("auth_storage")
    }
}
```

### Step 3: 更新所有使用硬编码路径的位置

需要更新的文件:
- `edge-server/src/services/cert.rs` - 使用 `config.certs_dir()`
- `edge-server/src/services/activation.rs` - 使用 `config.auth_storage_dir()`
- `edge-server/src/services/tenant_binding.rs` - 使用 `config.auth_storage_dir()`

### Step 4: 更新 red_coral Tauri 对应目录

在 `red_coral/src-tauri/src/core/client_bridge.rs` 确保:
- Server 模式启动时使用规范化路径
- 日志目录与数据目录分离

### Step 5: 添加迁移逻辑 (可选)

如果需要支持从旧目录结构迁移:

```rust
/// 检查并迁移旧目录结构
pub fn migrate_legacy_structure(work_dir: &Path) -> std::io::Result<()> {
    let legacy_db = work_dir.join("crab.db");
    let new_db_dir = work_dir.join("database");

    if legacy_db.exists() && !new_db_dir.exists() {
        tracing::info!("Migrating legacy database location...");
        std::fs::create_dir_all(&new_db_dir)?;
        std::fs::rename(&legacy_db, new_db_dir.join("crab.db"))?;
        tracing::info!("Database migration complete");
    }

    Ok(())
}
```

### Step 6: 运行测试

```bash
cargo test -p edge-server
cargo build -p edge-server
```

### Step 7: Commit

```bash
git add edge-server/src/core/state.rs edge-server/src/core/config.rs edge-server/src/services/cert.rs
git commit -m "refactor: normalize work_dir structure

- Move database to work_dir/database/
- Add Config helper methods for directory paths
- Add ensure_work_dir_structure() initialization
- Prepare for legacy structure migration

Directory structure now:
  work_dir/
  ├── certs/
  ├── database/
  ├── logs/
  └── auth_storage/

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

## Task 1: 实现 ConnectionMonitor 自动重连

**Files:**
- Modify: `red_coral/src-tauri/src/core/connection_monitor.rs`
- Modify: `red_coral/src-tauri/src/core/client_bridge.rs`
- Test: Manual testing with network disconnect

### Step 1: 在 ClientBridge 添加 reconnect_client_mode 方法

在 `client_bridge.rs` 中添加内部重连方法:

```rust
/// 内部重连方法 (仅用于 ConnectionMonitor)
///
/// 与 start_client_mode 不同，此方法假设已经是 Client 模式，
/// 仅重建连接而不检查当前模式
pub(crate) async fn reconnect_client(&self) -> Result<(), BridgeError> {
    let mut mode_guard = self.mode.write().await;

    // 确保是 Client 模式
    let (edge_url, message_addr) = match &*mode_guard {
        ClientMode::Client { edge_url, message_addr, .. } => {
            (edge_url.clone(), message_addr.clone())
        }
        _ => return Err(BridgeError::NotImplemented("Not in client mode".into())),
    };

    // 获取证书管理器
    let tenant_manager = self.tenant_manager.read().await;
    let cert_manager = tenant_manager
        .current_cert_manager()
        .ok_or(TenantError::NoTenantSelected)?;

    let config = self.config.read().await;
    let auth_url = config
        .client_config
        .as_ref()
        .map(|c| c.auth_url.clone())
        .unwrap_or_else(|| "https://auth.example.com".to_string());
    drop(config);

    // 重建 CrabClient<Remote>
    let client = CrabClient::remote()
        .auth_server(&auth_url)
        .edge_server(&edge_url)
        .cert_path(cert_manager.cert_path())
        .client_name(tenant_manager.client_name())
        .build()?;

    // 重连
    let connected_client = client.reconnect(&message_addr).await?;

    tracing::info!("Client mode reconnected via monitor");

    // 启动新的消息监听器
    if let Some(handle) = &self.app_handle {
        if let Some(mc) = connected_client.message_client() {
            let mut rx = mc.subscribe();
            let handle_clone = handle.clone();

            tokio::spawn(async move {
                loop {
                    match rx.recv().await {
                        Ok(msg) => {
                            let event = crate::events::ServerMessageEvent::from(msg);
                            let _ = handle_clone.emit("server-message", &event);
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                        Err(_) => continue,
                    }
                }
            });
        }
    }

    *mode_guard = ClientMode::Client {
        client: Some(RemoteClientState::Connected(connected_client)),
        edge_url,
        message_addr,
    };

    Ok(())
}
```

### Step 2: 更新 ConnectionMonitor 实现自动重连

修改 `connection_monitor.rs`:

```rust
//! Connection Monitor - 连接状态监控和自动重连

use std::sync::Arc;
use std::time::Duration;
use tauri::Emitter;
use tokio::sync::RwLock;
use tokio::time::interval;

use super::client_bridge::{ClientBridge, ModeType};

/// 重连配置
const MAX_RETRY_ATTEMPTS: u32 = 3;
const RETRY_DELAY_MS: u64 = 5000;

/// 连接监控器
pub struct ConnectionMonitor {
    bridge: Arc<RwLock<ClientBridge>>,
    check_interval: Duration,
}

impl ConnectionMonitor {
    pub fn new(bridge: Arc<RwLock<ClientBridge>>, check_interval: Duration) -> Self {
        Self {
            bridge,
            check_interval,
        }
    }

    pub async fn start(self, app_handle: tauri::AppHandle) {
        let mut ticker = interval(self.check_interval);
        let mut consecutive_failures = 0u32;

        loop {
            ticker.tick().await;

            let mode_info = {
                let bridge = self.bridge.read().await;
                bridge.get_mode_info().await
            };

            // 只在 Client 模式下进行检查
            if mode_info.mode != ModeType::Client {
                consecutive_failures = 0;
                continue;
            }

            if !mode_info.is_connected {
                consecutive_failures += 1;
                tracing::warn!(
                    "Connection lost (attempt {}/{})",
                    consecutive_failures,
                    MAX_RETRY_ATTEMPTS
                );

                // 发送断开事件
                let _ = app_handle.emit(
                    "connection-status",
                    serde_json::json!({
                        "connected": false,
                        "reconnecting": true,
                        "attempt": consecutive_failures,
                    }),
                );

                if consecutive_failures <= MAX_RETRY_ATTEMPTS {
                    // 尝试重连
                    tokio::time::sleep(Duration::from_millis(RETRY_DELAY_MS)).await;

                    let bridge = self.bridge.read().await;
                    match bridge.reconnect_client().await {
                        Ok(()) => {
                            tracing::info!("Reconnected successfully");
                            consecutive_failures = 0;
                            let _ = app_handle.emit(
                                "connection-status",
                                serde_json::json!({
                                    "connected": true,
                                    "reconnecting": false,
                                }),
                            );
                        }
                        Err(e) => {
                            tracing::error!("Reconnect failed: {}", e);
                        }
                    }
                } else {
                    // 超过重试次数，通知前端
                    let _ = app_handle.emit(
                        "connection-status",
                        serde_json::json!({
                            "connected": false,
                            "reconnecting": false,
                            "error": "Max retry attempts exceeded",
                        }),
                    );
                }
            } else {
                // 连接正常，重置计数器
                if consecutive_failures > 0 {
                    consecutive_failures = 0;
                    let _ = app_handle.emit(
                        "connection-status",
                        serde_json::json!({
                            "connected": true,
                            "reconnecting": false,
                        }),
                    );
                }
            }
        }
    }
}
```

### Step 3: 运行测试验证

```bash
cargo build -p red-coral
cargo run -p red-coral -- tauri:dev
```

### Step 4: Commit

```bash
git add src-tauri/src/core/connection_monitor.rs src-tauri/src/core/client_bridge.rs
git commit -m "feat(tauri): implement auto-reconnect in ConnectionMonitor

- Add reconnect_client() method to ClientBridge
- Implement retry logic with max 3 attempts
- Emit connection-status events for frontend feedback

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

## Task 2: 统一 Client 模式 API 调用

**Files:**
- Modify: `red_coral/src-tauri/src/core/client_bridge.rs`

### Step 1: 重构 get 方法使用 CrabClient

当前 Client 模式直接使用 reqwest，应该复用 CrabClient 的 HTTP 方法。

但是 CrabClient<Remote, Authenticated> 没有内置 get/post 方法（与 Local 不同），
所以需要保持现有实现，但可以提取公共逻辑。

**决策:** 保持现有实现，因为 CrabClient<Remote> 设计上依赖 edge_http_client()
返回的 reqwest::Client。这是合理的设计，不需要修改。

### Step 2: Commit (if changes made)

无需修改，现有设计合理。

---

## Task 3: 整合离线登录到 ClientBridge

**Files:**
- Modify: `red_coral/src-tauri/src/core/client_bridge.rs`
- Modify: `red_coral/src-tauri/src/commands/auth.rs`

### Step 1: 在 ClientBridge 添加 login_employee_with_fallback 方法

```rust
/// 员工登录 (支持离线回退)
///
/// 优先尝试在线登录，如果失败则尝试离线登录。
/// 仅在 Server 模式下支持离线登录。
pub async fn login_employee_with_fallback(
    &self,
    username: &str,
    password: &str,
) -> Result<EmployeeSession, BridgeError> {
    // 先尝试在线登录
    match self.login_employee(username, password).await {
        Ok(session) => Ok(session),
        Err(e) => {
            tracing::warn!("Online login failed: {}, trying offline...", e);

            // 仅在 Server 模式下尝试离线登录
            let mode_guard = self.mode.read().await;
            if !matches!(&*mode_guard, ClientMode::Server { .. }) {
                return Err(e); // Client 模式不支持离线登录
            }
            drop(mode_guard);

            // 尝试离线登录
            let mut tenant_manager = self.tenant_manager.write().await;
            match tenant_manager.login_offline(username, password) {
                Ok(session) => {
                    tracing::info!("Offline login successful for: {}", username);
                    Ok(session)
                }
                Err(offline_err) => {
                    tracing::error!("Offline login also failed: {}", offline_err);
                    Err(e) // 返回原始在线登录错误
                }
            }
        }
    }
}
```

### Step 2: 添加新的 Tauri 命令

在 `commands/auth.rs` 添加:

```rust
/// 员工登录 (带离线回退)
#[tauri::command]
pub async fn login_employee_auto(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
    username: String,
    password: String,
) -> Result<LoginResponse, String> {
    let bridge = bridge.read().await;

    match bridge.login_employee_with_fallback(&username, &password).await {
        Ok(session) => Ok(LoginResponse {
            success: true,
            mode: session.login_mode,
            session: Some(session),
            error: None,
        }),
        Err(e) => Ok(LoginResponse {
            success: false,
            session: None,
            error: Some(e.to_string()),
            mode: LoginMode::Offline,
        }),
    }
}
```

### Step 3: 注册命令

在 `lib.rs` 的 `invoke_handler` 中添加:

```rust
commands::login_employee_auto,
```

### Step 4: 运行测试

```bash
cargo build -p red-coral
```

### Step 5: Commit

```bash
git add src-tauri/src/core/client_bridge.rs src-tauri/src/commands/auth.rs src-tauri/src/lib.rs
git commit -m "feat(auth): add login_employee_auto with offline fallback

- Add login_employee_with_fallback() to ClientBridge
- Create login_employee_auto Tauri command
- Fallback to offline login only in Server mode

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

## Task 4: 完善配置恢复逻辑

**Files:**
- Modify: `red_coral/src-tauri/src/commands/mode.rs`

### Step 1: 修复 get_app_config 返回实际配置

```rust
#[tauri::command]
pub async fn get_app_config(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
) -> Result<AppConfigResponse, String> {
    let bridge = bridge.read().await;
    let info = bridge.get_mode_info().await;
    let tenant_manager = bridge.tenant_manager().read().await;

    // 获取实际配置
    let server_config = bridge.get_server_config().await;
    let client_config = bridge.get_client_config().await;

    Ok(AppConfigResponse {
        current_mode: info.mode,
        current_tenant: info.tenant_id,
        server_config,
        client_config,
        known_tenants: tenant_manager
            .list_tenants()
            .into_iter()
            .map(|t| t.tenant_id)
            .collect(),
    })
}
```

### Step 2: 运行测试

```bash
cargo build -p red-coral
```

### Step 3: Commit

```bash
git add src-tauri/src/commands/mode.rs
git commit -m "fix(mode): return actual config in get_app_config

- Use bridge.get_server_config() instead of default
- Use bridge.get_client_config() for client settings

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

## Task 5: 添加连接状态 Hook

**Files:**
- Create: `red_coral/src/core/hooks/useConnectionStatus.ts`
- Modify: `red_coral/src/core/stores/bridge/useBridgeStore.ts`

### Step 1: 创建 useConnectionStatus hook

```typescript
// src/core/hooks/useConnectionStatus.ts
import { useEffect, useState } from 'react';
import { listen } from '@tauri-apps/api/event';

interface ConnectionStatus {
  connected: boolean;
  reconnecting: boolean;
  attempt?: number;
  error?: string;
}

export function useConnectionStatus() {
  const [status, setStatus] = useState<ConnectionStatus>({
    connected: true,
    reconnecting: false,
  });

  useEffect(() => {
    const unlisten = listen<ConnectionStatus>('connection-status', (event) => {
      setStatus(event.payload);
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  return status;
}
```

### Step 2: 在 useBridgeStore 集成

```typescript
// 在 useBridgeStore 中添加
import { useConnectionStatus } from '../hooks/useConnectionStatus';

// 在 store 中添加连接状态
connectionStatus: {
  connected: true,
  reconnecting: false,
},

// 或者直接在需要的组件中使用 hook
```

### Step 3: Commit

```bash
git add src/core/hooks/useConnectionStatus.ts
git commit -m "feat(frontend): add useConnectionStatus hook

- Listen to connection-status Tauri events
- Expose connected, reconnecting, attempt, error states

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

## 验收标准

1. **Task 1**: Client 模式断网后能自动重连（最多3次）
2. **Task 2**: N/A (保持现有设计)
3. **Task 3**: login_employee_auto 命令能自动回退到离线登录
4. **Task 4**: get_app_config 返回实际配置而非默认值
5. **Task 5**: 前端能感知连接状态变化

---

## 执行方式

Plan complete and saved to `docs/plans/2026-01-18-crabclient-improvements.md`. Two execution options:

**1. Subagent-Driven (this session)** - I dispatch fresh subagent per task, review between tasks, fast iteration

**2. Parallel Session (separate)** - Open new session with executing-plans, batch execution with checkpoints

**Which approach?**
