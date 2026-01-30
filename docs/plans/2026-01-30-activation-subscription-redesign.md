# 激活流程 + 订阅检测 重构设计

## 问题总结

### Pain Point 1: 激活流程卡住

`Server::run()` → `wait_for_tls()` → `wait_for_activation()` 形成不可取消的无限循环：

```
wait_for_activation():
  loop {
    if !is_activated() {
      self.notify.notified().await  ← 无限阻塞，不响应 shutdown
    }
    if self_check fails {
      enter_unbound_state()         ← 清理一切，回到循环
    }                                  ← 没人再发 notify → 永久卡住
  }

wait_for_tls():
  loop {
    wait_for_activation().await      ← 上面卡住，这里也卡住
    match load_tls_config() {
      Ok(None) => enter_unbound_state()  ← 证书丢失 → 清理 → 回循环
    }                                       ← 又进 wait_for_activation → 永久卡住
  }
```

**结果**：Bridge 拿到了 `server_state`（router/message_bus 已初始化），但 Server 永远不启动 HTTPS。用户看到"已连接"实际无法使用。

### Pain Point 2: 订阅检测不准

8 个 Bug：

| # | 严重度 | 问题 |
|---|--------|------|
| 1 | HIGH | `sync_subscription()` 网络失败时静默降级，`Active` 缓存永不失效 |
| 2 | HIGH | `is_subscription_blocked()` 不检查签名过期 `signature_valid_until` |
| 3 | MEDIUM-HIGH | Bridge `get_app_state()` 和 edge-server `is_subscription_blocked()` 重复实现阻止逻辑 |
| 4 | MEDIUM | `sync_subscription()` 与 `get_app_state()` 读写竞争 |
| 5 | MEDIUM | Phase 4 循环固定 60s，无退避 |
| 6 | MEDIUM | `last_checked_at` 存在但从未用于陈旧性检查 |
| 7 | MEDIUM | `is_signature_expired()` → `Warning` 但不触发刷新 |
| 8 | LOW-MEDIUM | `fetch_subscription_from_auth_server` 响应校验不足 |

### Pain Point 3: 切换租户 Panic

已修复（CancellationToken graceful shutdown），不在本次范围。

---

## 设计方案

### 改动 1: `wait_for_activation()` 可取消

**文件**: `edge-server/src/services/activation.rs`

当前签名：
```rust
pub async fn wait_for_activation(&self, cert_service: &CertService)
```

改为：
```rust
pub async fn wait_for_activation(
    &self,
    cert_service: &CertService,
    cancel: &CancellationToken,
) -> Result<(), Cancelled>
```

实现：
```rust
pub async fn wait_for_activation(
    &self,
    cert_service: &CertService,
    cancel: &CancellationToken,
) -> Result<(), Cancelled> {
    loop {
        if !self.is_activated().await {
            tracing::info!("⏳ Server not activated. Waiting...");
            tokio::select! {
                _ = cancel.cancelled() => return Err(Cancelled),
                _ = self.notify.notified() => {
                    tracing::info!("📡 Activation signal received!");
                }
            }
        }

        // Self-check
        let cached = self.credential_cache.read().await.clone();
        match cert_service.self_check_with_binding(cached.as_ref()).await {
            Ok(()) => {
                self.update_last_verified_at().await;
                break;
            }
            Err(e) => {
                tracing::error!("❌ Self-check failed: {}", e);
                self.enter_unbound_state(cert_service).await;
                // 回到循环顶部，重新等待 notify
            }
        }
    }

    self.sync_subscription().await;
    Ok(())
}
```

### 改动 2: `wait_for_tls()` 可取消

**文件**: `edge-server/src/core/server.rs`

```rust
async fn wait_for_tls(&self, state: &ServerState) -> Option<Arc<rustls::ServerConfig>> {
    loop {
        if state.wait_for_activation(&self.shutdown_token).await.is_err() {
            return None; // shutdown requested
        }

        match state.load_tls_config() {
            Ok(Some(cfg)) => return Some(cfg),
            Ok(None) => {
                tracing::error!("❌ TLS certificates not found after activation!");
                state.enter_unbound_state().await;
            }
            Err(e) => {
                tracing::error!("❌ Failed to load TLS: {}. Entering unbound state.", e);
                state.enter_unbound_state().await;
            }
        }
    }
}
```

`run()` 中使用：
```rust
// Phase 3
let tls_config = match self.wait_for_tls(&state).await {
    Some(cfg) => cfg,
    None => {
        tracing::info!("Shutdown during activation wait");
        background_tasks.shutdown().await;
        return Ok(());
    }
};
```

### 改动 3: `is_subscription_blocked()` 增加签名过期检查

**文件**: `edge-server/src/services/activation.rs`

当前实现只检查 `status.is_blocked()`。改为同时检查签名有效期：

```rust
/// 检查订阅是否被阻止
///
/// 阻止条件 (任一满足):
/// 1. status 为 Inactive/Expired/Canceled/Unpaid
/// 2. 签名已过期且超过宽限期 (签名过期 + 3 天)
pub async fn is_subscription_blocked(&self) -> bool {
    let cache = self.credential_cache.read().await;
    let sub = match cache.as_ref().and_then(|c| c.subscription.as_ref()) {
        Some(s) => s,
        None => return false, // 无订阅数据 = 首次激活，不阻止
    };

    // 1. 状态阻止
    if sub.status.is_blocked() {
        return true;
    }

    // 2. 签名过期宽限检查
    //    签名有效期 7 天，过期后宽限 3 天 (共 10 天离线容忍)
    //    超过宽限期 → 必须联网刷新
    if sub.is_signature_stale() {
        tracing::warn!(
            "Subscription signature stale (expired + grace period exceeded). Blocking."
        );
        return true;
    }

    false
}
```

### 改动 4: `Subscription` 增加陈旧性检查方法

**文件**: `edge-server/src/services/tenant_binding.rs`

```rust
impl Subscription {
    /// 签名过期宽限期 (3 天)
    const SIGNATURE_GRACE_PERIOD_MS: i64 = 3 * 24 * 60 * 60 * 1000;

    /// 检查签名是否过期 (需要刷新)
    pub fn is_signature_expired(&self) -> bool {
        match self.signature_valid_until {
            Some(valid_until) => shared::util::now_millis() > valid_until,
            None => true,
        }
    }

    /// 检查签名是否陈旧 (过期 + 宽限期也已过)
    ///
    /// 签名有效期 7 天 + 宽限期 3 天 = 最多 10 天离线容忍。
    /// 超过此限制必须联网刷新，否则阻止使用。
    pub fn is_signature_stale(&self) -> bool {
        match self.signature_valid_until {
            Some(valid_until) => {
                shared::util::now_millis() > valid_until + Self::SIGNATURE_GRACE_PERIOD_MS
            }
            None => true,
        }
    }
}
```

### 改动 5: Bridge 去重 — 使用 edge-server 的统一判断

**文件**: `red_coral/src-tauri/src/core/bridge/mod.rs`

当前 `get_app_state()` 中 Bridge 自己实现了一套 `subscription_blocked` 判断（`matches!` on status）。
改为调用 `server_state.is_subscription_blocked()`：

```rust
// 替换 Bridge 中的重复逻辑
let subscription_blocked = server_state.is_subscription_blocked().await;

if subscription_blocked {
    // 构建 SubscriptionBlockedInfo 的逻辑保留
    // 但阻止判断统一来自 edge-server
}
```

同时新增 `get_subscription_blocked_info()` 方法到 `ActivationService`，将 info 构建也集中到 edge-server：

```rust
/// 获取订阅阻止信息 (供 Bridge 使用)
///
/// 返回 None 表示未阻止
pub async fn get_subscription_blocked_info(&self) -> Option<SubscriptionBlockedInfo> {
    let cache = self.credential_cache.read().await;
    let sub = cache.as_ref()?.subscription.as_ref()?;

    if !sub.status.is_blocked() && !sub.is_signature_stale() {
        return None;
    }

    // 构建 SubscriptionBlockedInfo
    let status = sub.status.to_shared();
    let plan = sub.plan.to_shared();

    let (user_message, expired_at) = if sub.is_signature_stale() && !sub.status.is_blocked() {
        ("subscription_signature_stale".to_string(), None)
    } else {
        let msg = match sub.status {
            SubscriptionStatus::Inactive => "subscription_inactive",
            SubscriptionStatus::Expired => "subscription_expired",
            SubscriptionStatus::Canceled => "subscription_canceled",
            SubscriptionStatus::Unpaid => "subscription_unpaid",
            _ => "subscription_blocked",
        };
        let expired_at = match sub.status {
            SubscriptionStatus::Inactive | SubscriptionStatus::Unpaid => None,
            _ => sub.expires_at,
        };
        (msg.to_string(), expired_at)
    };

    Some(SubscriptionBlockedInfo {
        status,
        plan,
        max_stores: sub.max_stores,
        expired_at,
        grace_period_days: None,
        grace_period_ends_at: None,
        in_grace_period: false,
        support_url: Some("https://support.example.com".to_string()),
        renewal_url: Some("https://billing.example.com/renew".to_string()),
        user_message,
    })
}
```

### 改动 6: Phase 4 指数退避

**文件**: `edge-server/src/core/server.rs`

```rust
// Phase 4: Subscription check with exponential backoff
let mut retry_delay = std::time::Duration::from_secs(10); // 首次 10s
const MAX_DELAY: std::time::Duration = std::time::Duration::from_secs(300); // 最大 5min

while state.is_subscription_blocked().await {
    state.print_subscription_blocked_banner().await;

    tokio::select! {
        _ = self.shutdown_token.cancelled() => {
            tracing::info!("Shutdown requested during subscription check");
            background_tasks.shutdown().await;
            return Ok(());
        }
        _ = tokio::time::sleep(retry_delay) => {}
    }

    state.sync_subscription().await;
    tracing::info!("🔄 Re-checked subscription (next retry in {:?})", retry_delay);

    // 指数退避: 10s → 20s → 40s → 80s → 160s → 300s
    retry_delay = (retry_delay * 2).min(MAX_DELAY);
}
```

### 改动 7: `sync_subscription()` 网络失败时标记陈旧

**文件**: `edge-server/src/services/activation.rs`

当前网络失败只打 warn 不做任何操作。改为：网络失败时，如果签名已过期，更新 `last_checked_at` 标记失败。

```rust
pub async fn sync_subscription(&self) {
    tracing::info!("Running subscription synchronization...");

    let mut credential = match self.get_credential().await {
        Ok(Some(c)) => c,
        _ => {
            tracing::debug!("Server not activated, skipping subscription sync");
            return;
        }
    };

    if let Some(sub) = self
        .fetch_subscription_from_auth_server(&credential.binding.tenant_id)
        .await
    {
        tracing::info!(
            "Subscription sync successful for tenant {}: {:?}",
            credential.binding.tenant_id,
            sub.status
        );
        credential.subscription = Some(sub);

        if let Err(e) = credential.save(&self.cert_dir) {
            tracing::error!("Failed to save subscription: {}", e);
        }
        let mut cache = self.credential_cache.write().await;
        *cache = Some(credential);
    } else {
        // 网络失败 → 检查签名是否过期
        if let Some(sub) = &credential.subscription {
            if sub.is_signature_expired() {
                tracing::warn!(
                    "Subscription sync failed AND signature expired! \
                     Offline grace period applies."
                );
            } else {
                tracing::info!(
                    "Subscription sync failed but signature still valid \
                     (expires in {}h). Using cached data.",
                    sub.signature_valid_until
                        .map(|v| (v - shared::util::now_millis()) / 3_600_000)
                        .unwrap_or(0)
                );
            }
        }
    }
}
```

---

## 改动汇总

| # | 文件 | 改动 | 解决 |
|---|------|------|------|
| 1 | `activation.rs` | `wait_for_activation()` 接受 `CancellationToken`，`select!` | Pain 1: 卡住 |
| 2 | `server.rs` | `wait_for_tls()` 返回 `Option`，`run()` 处理 shutdown | Pain 1: 卡住 |
| 3 | `activation.rs` | `is_subscription_blocked()` 增加签名过期检查 | Bug #2, #6 |
| 4 | `tenant_binding.rs` | `Subscription` 增加 `is_signature_stale()` 方法 | Bug #2 基础 |
| 5 | `bridge/mod.rs` | `get_app_state()` 调用 edge-server 判断，去重 | Bug #3 |
| 5b | `activation.rs` | 新增 `get_subscription_blocked_info()` | Bug #3 |
| 6 | `server.rs` | Phase 4 指数退避 10s→300s | Bug #5 |
| 7 | `activation.rs` | `sync_subscription()` 网络失败时日志增强 | Bug #1, #7 |

## 不在本次范围

- Bug #4 (读写竞争): 当前 `RwLock` 粒度已足够，极端情况下读到旧数据无实际危害
- Bug #8 (响应校验): 当前 serde 解析已有基本校验，改进优先级低

## 验证

```bash
cargo check -p edge-server
cargo check -p red-coral
cargo test -p edge-server --lib
```
