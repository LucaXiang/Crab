# Tauri + CrabClient + EdgeServer 集成实现计划

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 完善 Tauri 应用通过 CrabClient 对接 EdgeServer API 的完整集成，支持 Server 模式(InProcess) 和 Client 模式(Remote/mTLS)。

**Architecture:**
- Server 模式: Tauri 启动嵌入式 EdgeServer，通过 `CrabClient<Local>` 使用 Tower oneshot 进行同进程 HTTP 调用
- Client 模式: Tauri 通过 `CrabClient<Remote>` 使用 mTLS 连接远程 EdgeServer
- 两种模式通过 `ClientBridge` 统一管理，提供一致的 API 给前端

**Tech Stack:**
- Rust: Tauri 2.9, edge-server, crab-client (in-process feature), tokio
- Frontend: React, TypeScript, Zustand
- Protocol: HTTP/HTTPS (mTLS), TCP/TLS (Message Bus)

---

## 现状分析

### 已完成的工作

1. **ClientBridge 架构** (`red_coral/src-tauri/src/core/client_bridge.rs`)
   - ✅ Server/Client 两种模式的状态机定义
   - ✅ `start_server_mode()` - 启动本地 EdgeServer
   - ✅ `start_client_mode()` - 连接远程 EdgeServer
   - ✅ `login_employee()` / `logout_employee()` - 员工认证
   - ✅ GET/POST/PUT/DELETE 统一 HTTP API

2. **TenantManager** (`red_coral/src-tauri/src/core/tenant_manager.rs`)
   - ✅ 多租户证书管理
   - ✅ 设备激活 (从 Auth Server 获取证书)
   - ✅ 会话缓存 (支持离线登录)

3. **Tauri Commands** (`red_coral/src-tauri/src/commands/`)
   - ✅ 模式管理命令
   - ✅ 租户管理命令
   - ✅ 认证命令
   - ✅ 业务数据 CRUD 命令

4. **CrabClient** (`crab-client/src/`)
   - ✅ `CrabClient<Remote>` - mTLS HTTP + TCP Message Bus
   - ✅ `CrabClient<Local>` - Tower Oneshot HTTP + In-Memory Message Bus
   - ✅ Typestate pattern (Disconnected → Connected → Authenticated)

### 需要完善的工作

1. **Server 模式的 Message Bus 集成**
   - 当前: Server 模式仅使用 HTTP API
   - 目标: 添加 Message Bus 支持以接收服务器广播 (Notification, Sync)

2. **Client 模式的连接健壮性**
   - 当前: 简单的重连逻辑
   - 目标: 自动重连、健康检查、断线通知

3. **前端通知系统集成**
   - 当前: 前端无法接收服务器推送
   - 目标: 通过 Tauri Events 将 Message Bus 消息转发给前端

---

## Task 1: 完善 Server 模式的 In-Process Message Bus 订阅

**Files:**
- Modify: `red_coral/src-tauri/src/core/client_bridge.rs:467-507`

**Step 1: 阅读现有代码确认 Message Bus 创建位置**

在 `start_server_mode()` 中确认:
- `state_arc.message_bus()` 获取 MessageBusService
- `message_bus.sender_to_server()` 和 `message_bus.sender()` 用于创建 InMemoryMessageClient

**Step 2: 添加服务器消息订阅启动器**

在 `start_server_mode()` 完成后，启动一个后台任务订阅 server_rx:

```rust
// 在 ClientMode::Server 设置之后添加
// 启动消息订阅器 (用于将服务器广播转发给前端)
let server_rx = message_bus.subscribe();
let app_handle_clone = app_handle.clone(); // 需要从参数传入
tokio::spawn(async move {
    let mut rx = server_rx;
    loop {
        match rx.recv().await {
            Ok(msg) => {
                // 通过 Tauri Event 转发给前端
                let _ = app_handle_clone.emit("server-message", &msg);
            }
            Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                tracing::warn!("Server message receiver lagged {} messages", n);
            }
            Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                tracing::debug!("Server message channel closed");
                break;
            }
        }
    }
});
```

**Step 3: 修改 ClientBridge::new 以接受 AppHandle**

需要将 Tauri AppHandle 传入 ClientBridge 以便发送事件:

```rust
pub struct ClientBridge {
    // ... existing fields ...
    /// Tauri AppHandle for emitting events
    app_handle: Option<tauri::AppHandle>,
}
```

**Step 4: 运行测试验证**

Run: `cd /Users/xzy/workspace/crab/red_coral && npm run tauri:dev`
Expected: 应用启动，Server 模式正常工作

**Step 5: Commit**

```bash
git add red_coral/src-tauri/src/core/client_bridge.rs
git commit -m "$(cat <<'EOF'
feat(tauri): add message bus subscription in server mode

Enable ServerState message bus subscription to forward server broadcasts
to the frontend via Tauri events.

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
EOF
)"
```

---

## Task 2: 创建 Tauri Event 定义和前端监听器

**Files:**
- Create: `red_coral/src-tauri/src/events.rs`
- Modify: `red_coral/src-tauri/src/lib.rs`
- Create: `red_coral/src/core/hooks/useServerMessages.ts`

**Step 1: 创建 Rust 事件定义**

```rust
// red_coral/src-tauri/src/events.rs
use serde::{Deserialize, Serialize};
use shared::message::BusMessage;

/// 服务器消息事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerMessageEvent {
    pub event_type: String,
    pub payload: serde_json::Value,
    pub correlation_id: Option<String>,
}

impl From<BusMessage> for ServerMessageEvent {
    fn from(msg: BusMessage) -> Self {
        Self {
            event_type: msg.event_type.to_string(),
            payload: msg.payload.clone(),
            correlation_id: msg.correlation_id.clone(),
        }
    }
}
```

**Step 2: 在 lib.rs 中导出 events 模块**

```rust
// 在 red_coral/src-tauri/src/lib.rs 中添加
pub mod events;
```

**Step 3: 创建前端 Hook**

```typescript
// red_coral/src/core/hooks/useServerMessages.ts
import { useEffect, useCallback } from 'react';
import { listen, UnlistenFn } from '@tauri-apps/api/event';

export interface ServerMessage {
  event_type: string;
  payload: unknown;
  correlation_id?: string;
}

export type MessageHandler = (message: ServerMessage) => void;

export function useServerMessages(handler: MessageHandler) {
  const stableHandler = useCallback(handler, [handler]);

  useEffect(() => {
    let unlisten: UnlistenFn | undefined;

    listen<ServerMessage>('server-message', (event) => {
      stableHandler(event.payload);
    }).then((fn) => {
      unlisten = fn;
    });

    return () => {
      unlisten?.();
    };
  }, [stableHandler]);
}
```

**Step 4: 运行 TypeScript 检查**

Run: `cd /Users/xzy/workspace/crab/red_coral && npx tsc --noEmit`
Expected: 无类型错误

**Step 5: Commit**

```bash
git add red_coral/src-tauri/src/events.rs red_coral/src-tauri/src/lib.rs red_coral/src/core/hooks/useServerMessages.ts
git commit -m "$(cat <<'EOF'
feat(tauri): add server message events and frontend hook

- Define ServerMessageEvent type for Tauri events
- Create useServerMessages hook for frontend message handling

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
EOF
)"
```

---

## Task 3: 重构 ClientBridge 支持 AppHandle 注入

**Files:**
- Modify: `red_coral/src-tauri/src/core/client_bridge.rs`
- Modify: `red_coral/src-tauri/src/lib.rs`

**Step 1: 修改 ClientBridge 结构体添加 AppHandle**

```rust
pub struct ClientBridge {
    tenant_manager: Arc<RwLock<TenantManager>>,
    mode: RwLock<ClientMode>,
    config: RwLock<AppConfig>,
    config_path: PathBuf,
    base_path: PathBuf,
    /// Tauri AppHandle for emitting events (optional for testing)
    app_handle: Option<tauri::AppHandle>,
}
```

**Step 2: 修改 new() 方法**

```rust
impl ClientBridge {
    pub fn new(base_path: impl Into<PathBuf>, client_name: &str) -> Result<Self, BridgeError> {
        Self::with_app_handle(base_path, client_name, None)
    }

    pub fn with_app_handle(
        base_path: impl Into<PathBuf>,
        client_name: &str,
        app_handle: Option<tauri::AppHandle>,
    ) -> Result<Self, BridgeError> {
        let base_path = base_path.into();
        std::fs::create_dir_all(&base_path)?;

        let config_path = base_path.join("config.json");
        let config = AppConfig::load(&config_path)?;

        let tenants_path = base_path.join("tenants");
        let mut tenant_manager = TenantManager::new(&tenants_path, client_name);
        tenant_manager.load_existing_tenants()?;

        Ok(Self {
            tenant_manager: Arc::new(RwLock::new(tenant_manager)),
            mode: RwLock::new(ClientMode::Disconnected),
            config: RwLock::new(config),
            config_path,
            base_path,
            app_handle,
        })
    }

    /// Set the app handle after initialization
    pub fn set_app_handle(&mut self, handle: tauri::AppHandle) {
        self.app_handle = Some(handle);
    }
}
```

**Step 3: 修改 lib.rs 中的初始化代码**

```rust
// 在 setup 闭包中
let bridge = ClientBridge::with_app_handle(&work_dir, &client_name, Some(app.handle().clone()))
    .map_err(|e| format!("Failed to initialize ClientBridge: {}", e))?;
```

**Step 4: 运行 cargo check**

Run: `cd /Users/xzy/workspace/crab && cargo check -p red_coral`
Expected: 编译通过

**Step 5: Commit**

```bash
git add red_coral/src-tauri/src/core/client_bridge.rs red_coral/src-tauri/src/lib.rs
git commit -m "$(cat <<'EOF'
refactor(tauri): inject AppHandle into ClientBridge

Allow ClientBridge to emit Tauri events by injecting the AppHandle
during initialization.

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
EOF
)"
```

---

## Task 4: 在 Server 模式启动时订阅 Message Bus

**Files:**
- Modify: `red_coral/src-tauri/src/core/client_bridge.rs:410-507`

**Step 1: 在 start_server_mode() 中添加消息订阅**

在创建 `ClientMode::Server` 之前，启动消息监听任务:

```rust
// 启动消息广播订阅 (转发给前端)
if let Some(handle) = &self.app_handle {
    let mut server_rx = message_bus.subscribe();
    let handle_clone = handle.clone();

    tokio::spawn(async move {
        loop {
            match server_rx.recv().await {
                Ok(msg) => {
                    let event = crate::events::ServerMessageEvent::from(msg);
                    if let Err(e) = handle_clone.emit("server-message", &event) {
                        tracing::warn!("Failed to emit server message: {}", e);
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!("Server message listener lagged {} messages", n);
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                    tracing::debug!("Server message channel closed");
                    break;
                }
            }
        }
    });

    tracing::info!("Server message listener started");
}
```

**Step 2: 运行 cargo check**

Run: `cd /Users/xzy/workspace/crab && cargo check -p red_coral`
Expected: 编译通过

**Step 3: 测试 Server 模式启动**

Run: `cd /Users/xzy/workspace/crab/red_coral && npm run tauri:dev`
Expected: 应用启动，日志显示 "Server message listener started"

**Step 4: Commit**

```bash
git add red_coral/src-tauri/src/core/client_bridge.rs
git commit -m "$(cat <<'EOF'
feat(tauri): subscribe to message bus in server mode

Start a background task to forward server broadcasts to the frontend
via Tauri events when server mode is started.

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
EOF
)"
```

---

## Task 5: 在 Client 模式启动时订阅 Message Bus

**Files:**
- Modify: `red_coral/src-tauri/src/core/client_bridge.rs:509-582`

**Step 1: 在 start_client_mode() 中添加消息订阅**

Client 模式使用 `NetworkMessageClient.subscribe()`:

```rust
// 在获取 connected_client 之后，保存到 ClientMode 之前

// 启动消息广播订阅 (转发给前端)
if let Some(handle) = &self.app_handle {
    if let Some(mc) = connected_client.message_client() {
        let mut rx = mc.subscribe();
        let handle_clone = handle.clone();

        tokio::spawn(async move {
            loop {
                match rx.recv().await {
                    Ok(msg) => {
                        // NetworkMessageClient 返回 shared::message::Message
                        // 需要转换为 BusMessage
                        let event = crate::events::ServerMessageEvent {
                            event_type: msg.event_type.to_string(),
                            payload: msg.payload.clone(),
                            correlation_id: msg.correlation_id.clone(),
                        };
                        if let Err(e) = handle_clone.emit("server-message", &event) {
                            tracing::warn!("Failed to emit server message: {}", e);
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        tracing::warn!("Client message listener lagged {} messages", n);
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                        tracing::debug!("Client message channel closed");
                        break;
                    }
                }
            }
        });

        tracing::info!("Client message listener started");
    }
}
```

**Step 2: 运行 cargo check**

Run: `cd /Users/xzy/workspace/crab && cargo check -p red_coral`
Expected: 编译通过

**Step 3: Commit**

```bash
git add red_coral/src-tauri/src/core/client_bridge.rs
git commit -m "$(cat <<'EOF'
feat(tauri): subscribe to message bus in client mode

Start a background task to forward remote server broadcasts to the
frontend via Tauri events when client mode is started.

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
EOF
)"
```

---

## Task 6: 创建通知展示组件

**Files:**
- Create: `red_coral/src/presentation/components/notifications/NotificationToast.tsx`
- Create: `red_coral/src/presentation/components/notifications/NotificationProvider.tsx`
- Modify: `red_coral/src/App.tsx`

**Step 1: 创建 NotificationToast 组件**

```typescript
// red_coral/src/presentation/components/notifications/NotificationToast.tsx
import React from 'react';

export interface Notification {
  id: string;
  level: 'info' | 'warning' | 'error' | 'success';
  title: string;
  message: string;
  timestamp: number;
}

interface Props {
  notification: Notification;
  onDismiss: (id: string) => void;
}

export function NotificationToast({ notification, onDismiss }: Props) {
  const levelStyles = {
    info: 'bg-blue-100 border-blue-500 text-blue-700',
    warning: 'bg-yellow-100 border-yellow-500 text-yellow-700',
    error: 'bg-red-100 border-red-500 text-red-700',
    success: 'bg-green-100 border-green-500 text-green-700',
  };

  return (
    <div
      className={`border-l-4 p-4 mb-2 rounded shadow-lg ${levelStyles[notification.level]}`}
      role="alert"
    >
      <div className="flex justify-between items-start">
        <div>
          <p className="font-bold">{notification.title}</p>
          <p className="text-sm">{notification.message}</p>
        </div>
        <button
          onClick={() => onDismiss(notification.id)}
          className="ml-4 text-lg font-semibold"
        >
          ×
        </button>
      </div>
    </div>
  );
}
```

**Step 2: 创建 NotificationProvider**

```typescript
// red_coral/src/presentation/components/notifications/NotificationProvider.tsx
import React, { createContext, useContext, useState, useCallback, useEffect } from 'react';
import { useServerMessages, ServerMessage } from '@/core/hooks/useServerMessages';
import { NotificationToast, Notification } from './NotificationToast';

interface NotificationContextValue {
  notifications: Notification[];
  addNotification: (notification: Omit<Notification, 'id' | 'timestamp'>) => void;
  dismissNotification: (id: string) => void;
}

const NotificationContext = createContext<NotificationContextValue | null>(null);

export function useNotifications() {
  const ctx = useContext(NotificationContext);
  if (!ctx) throw new Error('useNotifications must be used within NotificationProvider');
  return ctx;
}

export function NotificationProvider({ children }: { children: React.ReactNode }) {
  const [notifications, setNotifications] = useState<Notification[]>([]);

  const addNotification = useCallback((notif: Omit<Notification, 'id' | 'timestamp'>) => {
    const id = crypto.randomUUID();
    setNotifications((prev) => [...prev, { ...notif, id, timestamp: Date.now() }]);

    // Auto dismiss after 5 seconds
    setTimeout(() => {
      setNotifications((prev) => prev.filter((n) => n.id !== id));
    }, 5000);
  }, []);

  const dismissNotification = useCallback((id: string) => {
    setNotifications((prev) => prev.filter((n) => n.id !== id));
  }, []);

  // Handle server messages
  useServerMessages(useCallback((msg: ServerMessage) => {
    if (msg.event_type === 'Notification') {
      const payload = msg.payload as { level?: string; title?: string; message?: string };
      addNotification({
        level: (payload.level as Notification['level']) || 'info',
        title: payload.title || 'Notification',
        message: payload.message || '',
      });
    }
  }, [addNotification]));

  return (
    <NotificationContext.Provider value={{ notifications, addNotification, dismissNotification }}>
      {children}
      {/* Notification container */}
      <div className="fixed top-4 right-4 z-50 w-80">
        {notifications.map((n) => (
          <NotificationToast key={n.id} notification={n} onDismiss={dismissNotification} />
        ))}
      </div>
    </NotificationContext.Provider>
  );
}
```

**Step 3: 在 App.tsx 中添加 NotificationProvider**

```typescript
// 在 App.tsx 中
import { NotificationProvider } from '@/presentation/components/notifications/NotificationProvider';

// 在返回的 JSX 中包裹
return (
  <NotificationProvider>
    {/* existing router content */}
  </NotificationProvider>
);
```

**Step 4: 运行 TypeScript 检查**

Run: `cd /Users/xzy/workspace/crab/red_coral && npx tsc --noEmit`
Expected: 无类型错误

**Step 5: Commit**

```bash
git add red_coral/src/presentation/components/notifications/ red_coral/src/App.tsx
git commit -m "$(cat <<'EOF'
feat(frontend): add notification system for server messages

- NotificationToast component for displaying individual notifications
- NotificationProvider for managing notification state and server messages
- Integration with useServerMessages hook

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
EOF
)"
```

---

## Task 7: 添加连接状态监控和自动重连

**Files:**
- Create: `red_coral/src-tauri/src/core/connection_monitor.rs`
- Modify: `red_coral/src-tauri/src/core/mod.rs`
- Modify: `red_coral/src-tauri/src/core/client_bridge.rs`

**Step 1: 创建连接监控模块**

```rust
// red_coral/src-tauri/src/core/connection_monitor.rs
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::interval;

use super::client_bridge::{ClientBridge, ModeType};

/// 连接监控器
/// 定期检查连接状态，在 Client 模式下自动重连
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

    /// 启动监控循环
    pub async fn start(self, app_handle: tauri::AppHandle) {
        let mut ticker = interval(self.check_interval);

        loop {
            ticker.tick().await;

            let bridge = self.bridge.read().await;
            let mode_info = bridge.get_mode_info().await;

            // 只在 Client 模式下进行检查
            if mode_info.mode == ModeType::Client && !mode_info.is_connected {
                tracing::warn!("Connection lost in client mode, attempting reconnect...");

                // Emit disconnected event
                let _ = app_handle.emit("connection-status", serde_json::json!({
                    "connected": false,
                    "reconnecting": true,
                }));

                // 尝试重连
                drop(bridge);
                let bridge = self.bridge.read().await;

                // TODO: 实现重连逻辑
                // 这需要重新调用 start_client_mode，但目前的设计需要先 stop

                let _ = app_handle.emit("connection-status", serde_json::json!({
                    "connected": mode_info.is_connected,
                    "reconnecting": false,
                }));
            }
        }
    }
}
```

**Step 2: 在 mod.rs 中导出**

```rust
// red_coral/src-tauri/src/core/mod.rs
pub mod connection_monitor;
pub use connection_monitor::ConnectionMonitor;
```

**Step 3: 在 lib.rs 中启动监控**

```rust
// 在 setup 闭包中，创建 bridge 之后
let bridge_for_monitor = bridge.clone();
let handle_for_monitor = app.handle().clone();
tauri::async_runtime::spawn(async move {
    let monitor = ConnectionMonitor::new(bridge_for_monitor, Duration::from_secs(30));
    monitor.start(handle_for_monitor).await;
});
```

**Step 4: 运行 cargo check**

Run: `cd /Users/xzy/workspace/crab && cargo check -p red_coral`
Expected: 编译通过

**Step 5: Commit**

```bash
git add red_coral/src-tauri/src/core/connection_monitor.rs red_coral/src-tauri/src/core/mod.rs red_coral/src-tauri/src/lib.rs
git commit -m "$(cat <<'EOF'
feat(tauri): add connection monitor for client mode

Add ConnectionMonitor to periodically check connection status and emit
events when disconnected. Foundation for auto-reconnect feature.

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
EOF
)"
```

---

## Task 8: 创建前端连接状态 Hook

**Files:**
- Create: `red_coral/src/core/hooks/useConnectionStatus.ts`
- Modify: `red_coral/src/core/stores/bridge/useBridgeStore.ts`

**Step 1: 创建连接状态 Hook**

```typescript
// red_coral/src/core/hooks/useConnectionStatus.ts
import { useEffect, useState } from 'react';
import { listen, UnlistenFn } from '@tauri-apps/api/event';

export interface ConnectionStatus {
  connected: boolean;
  reconnecting: boolean;
}

export function useConnectionStatus() {
  const [status, setStatus] = useState<ConnectionStatus>({
    connected: true,
    reconnecting: false,
  });

  useEffect(() => {
    let unlisten: UnlistenFn | undefined;

    listen<ConnectionStatus>('connection-status', (event) => {
      setStatus(event.payload);
    }).then((fn) => {
      unlisten = fn;
    });

    return () => {
      unlisten?.();
    };
  }, []);

  return status;
}
```

**Step 2: 在 useBridgeStore 中集成连接状态**

在 store 中添加连接状态字段和更新方法，以便其他组件可以访问。

**Step 3: 运行 TypeScript 检查**

Run: `cd /Users/xzy/workspace/crab/red_coral && npx tsc --noEmit`
Expected: 无类型错误

**Step 4: Commit**

```bash
git add red_coral/src/core/hooks/useConnectionStatus.ts red_coral/src/core/stores/bridge/useBridgeStore.ts
git commit -m "$(cat <<'EOF'
feat(frontend): add connection status hook

Create useConnectionStatus hook to track connection state from backend
monitor and integrate with bridge store.

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
EOF
)"
```

---

## Task 9: 添加 Tauri Command 用于手动重连

**Files:**
- Modify: `red_coral/src-tauri/src/commands/mode.rs`
- Modify: `red_coral/src-tauri/src/lib.rs`

**Step 1: 在 mode.rs 中添加 reconnect 命令**

```rust
/// 重新连接 (仅 Client 模式)
#[tauri::command]
pub async fn reconnect(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
) -> Result<(), String> {
    let bridge = bridge.read().await;
    let mode_info = bridge.get_mode_info().await;

    if mode_info.mode != ModeType::Client {
        return Err("Reconnect is only available in Client mode".into());
    }

    // 获取当前配置
    let config = bridge.config.read().await;
    let client_config = config.client_config.clone()
        .ok_or("No client configuration found")?;
    drop(config);
    drop(bridge);

    // 停止并重新启动
    let bridge = bridge.write().await;
    bridge.stop().await.map_err(|e| e.to_string())?;
    bridge.start_client_mode(&client_config.edge_url, &client_config.message_addr)
        .await
        .map_err(|e| e.to_string())
}
```

**Step 2: 在 lib.rs 中注册命令**

```rust
// 在 invoke_handler 中添加
commands::reconnect,
```

**Step 3: 运行 cargo check**

Run: `cd /Users/xzy/workspace/crab && cargo check -p red_coral`
Expected: 编译通过

**Step 4: Commit**

```bash
git add red_coral/src-tauri/src/commands/mode.rs red_coral/src-tauri/src/lib.rs
git commit -m "$(cat <<'EOF'
feat(tauri): add reconnect command for client mode

Allow frontend to trigger manual reconnection to remote edge server.

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
EOF
)"
```

---

## Task 10: 集成测试和验证

**Files:**
- No new files, testing existing functionality

**Step 1: 启动 Auth Server**

Run: `cd /Users/xzy/workspace/crab && cargo run -p crab-auth`
Expected: Auth Server 在 3001 端口启动

**Step 2: 启动应用并测试 Server 模式**

Run: `cd /Users/xzy/workspace/crab/red_coral && npm run tauri:dev`
Expected:
- 应用启动
- 能够激活租户
- Server 模式启动成功
- 员工登录成功
- 日志显示 "Server message listener started"

**Step 3: 测试通知接收**

在 interactive_demo (另一个 edge-server 实例) 中发送通知:
```
/notify TestTitle TestMessage
```

Expected: 前端显示通知 Toast

**Step 4: 测试 Client 模式**

1. 先启动一个独立的 edge-server: `cargo run --example interactive_demo -p edge-server`
2. 在应用中切换到 Client 模式
3. 验证连接成功
4. 验证能接收远程服务器的通知

**Step 5: Commit 最终状态**

```bash
git add -A
git commit -m "$(cat <<'EOF'
test: verify tauri + crabclient + edgeserver integration

All integration tests pass:
- Server mode with message bus subscription
- Client mode with mTLS connection
- Notification forwarding to frontend
- Connection monitoring

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
EOF
)"
```

---

## 总结

本计划包含 10 个任务，涵盖：

1. **Server 模式 Message Bus 集成** (Tasks 1, 4)
2. **Tauri Events 系统** (Tasks 2, 3)
3. **Client 模式 Message Bus 集成** (Task 5)
4. **前端通知系统** (Task 6)
5. **连接状态监控** (Tasks 7, 8, 9)
6. **集成测试** (Task 10)

每个任务都是独立的、可测试的单元，遵循 TDD 原则。执行时请确保：
- 每完成一个步骤就运行相应的测试/检查
- 及时提交代码保留工作进度
- 遇到问题时回退到上一个稳定状态
