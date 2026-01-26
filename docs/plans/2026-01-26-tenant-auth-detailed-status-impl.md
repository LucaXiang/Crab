# Tenant 认证状态详细信息 - 实现计划

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 让 edge-server 能够告诉前端详细的认证状态，而不是简单跳转到 /setup

**Architecture:** 
1. 在 `shared` crate 定义状态类型 (Rust)
2. 在 `red_coral/src-tauri` 修改 `get_app_state` 返回详细原因
3. 在 `red_coral/src` 更新 TypeScript 类型和路由逻辑

**Tech Stack:** Rust (serde), TypeScript, Tauri

---

## Task 1: 定义状态类型 (shared crate)

**Files:**
- Create: `shared/src/app_state.rs`
- Modify: `shared/src/lib.rs`

**Step 1: 创建 app_state.rs 文件**

```rust
// shared/src/app_state.rs
//! 应用状态类型定义
//!
//! 统一 Server/Client 模式的应用状态，供前端路由守卫使用。

use serde::{Deserialize, Serialize};

use crate::activation::{PlanType, SubscriptionStatus};

// =============================================================================
// 激活失败原因
// =============================================================================

/// 时钟偏移方向
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClockDirection {
    /// 时钟回拨
    Backward,
    /// 时钟前跳
    Forward,
}

/// 需要激活的原因
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "code", content = "details")]
pub enum ActivationRequiredReason {
    /// 首次激活
    FirstTimeSetup,

    /// 证书过期
    CertificateExpired {
        expired_at: String,
        days_overdue: i64,
    },

    /// 证书即将过期 (警告)
    CertificateExpiringSoon {
        expires_at: String,
        days_remaining: i64,
    },

    /// 证书无效
    CertificateInvalid { error: String },

    /// 签名验证失败
    SignatureInvalid { component: String, error: String },

    /// 硬件 ID 不匹配
    DeviceMismatch { expected: String, actual: String },

    /// 时钟篡改
    ClockTampering {
        direction: ClockDirection,
        drift_seconds: i64,
        last_verified_at: String,
    },

    /// Binding 无效
    BindingInvalid { error: String },

    /// Token 过期
    TokenExpired { expired_at: String },

    /// 网络错误
    NetworkError {
        error: String,
        can_continue_offline: bool,
    },

    /// 已被吊销
    Revoked { revoked_at: String, reason: String },
}

impl ActivationRequiredReason {
    /// 获取恢复建议
    pub fn recovery_hint(&self) -> &'static str {
        match self {
            Self::FirstTimeSetup => "输入管理员提供的凭据完成激活",
            Self::CertificateExpired { .. } => "请重新激活设备以更新证书",
            Self::CertificateExpiringSoon { .. } => "建议尽快重新激活以更新证书",
            Self::CertificateInvalid { .. } => "证书文件损坏，请重新激活",
            Self::SignatureInvalid { .. } => "安全验证失败，请重新激活",
            Self::DeviceMismatch { .. } => "如果更换了设备，请联系管理员重新激活",
            Self::ClockTampering { .. } => "请检查系统时间设置是否正确",
            Self::BindingInvalid { .. } => "设备绑定无效，请重新激活",
            Self::TokenExpired { .. } => "凭据已过期，请重新激活",
            Self::NetworkError { can_continue_offline: true, .. } => "可以离线继续使用，联网后将自动同步",
            Self::NetworkError { can_continue_offline: false, .. } => "请检查网络连接后重试",
            Self::Revoked { .. } => "请联系管理员了解详情",
        }
    }

    /// 是否可以自动恢复
    pub fn can_auto_recover(&self) -> bool {
        matches!(
            self,
            Self::CertificateExpiringSoon { .. }
                | Self::NetworkError { can_continue_offline: true, .. }
        )
    }
}

// =============================================================================
// 订阅阻止信息
// =============================================================================

/// 订阅阻止详细信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionBlockedInfo {
    pub status: SubscriptionStatus,
    pub plan: PlanType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expired_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grace_period_days: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grace_period_ends_at: Option<String>,
    pub in_grace_period: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub support_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub renewal_url: Option<String>,
    pub user_message: String,
}

// =============================================================================
// 激活进度
// =============================================================================

/// 激活步骤
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActivationStep {
    Authenticating,
    DownloadingCertificates,
    VerifyingBinding,
    CheckingSubscription,
    StartingServer,
    Complete,
}

impl ActivationStep {
    pub fn message_zh(&self) -> &'static str {
        match self {
            Self::Authenticating => "正在验证凭据...",
            Self::DownloadingCertificates => "正在下载证书...",
            Self::VerifyingBinding => "正在验证设备绑定...",
            Self::CheckingSubscription => "正在检查订阅状态...",
            Self::StartingServer => "正在启动服务...",
            Self::Complete => "激活完成",
        }
    }

    pub fn step_number(&self) -> u8 {
        match self {
            Self::Authenticating => 1,
            Self::DownloadingCertificates => 2,
            Self::VerifyingBinding => 3,
            Self::CheckingSubscription => 4,
            Self::StartingServer => 5,
            Self::Complete => 6,
        }
    }

    pub const TOTAL_STEPS: u8 = 6;
}

/// 激活进度
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivationProgress {
    pub step: ActivationStep,
    pub total_steps: u8,
    pub current_step: u8,
    pub message: String,
    pub started_at: String,
}

impl ActivationProgress {
    pub fn new(step: ActivationStep) -> Self {
        Self {
            step,
            total_steps: ActivationStep::TOTAL_STEPS,
            current_step: step.step_number(),
            message: step.message_zh().to_string(),
            started_at: chrono::Utc::now().to_rfc3339(),
        }
    }
}

// =============================================================================
// 健康检查
// =============================================================================

/// 健康级别
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HealthLevel {
    Healthy,
    Warning,
    Critical,
    Unknown,
}

/// 证书健康状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CertificateHealth {
    pub status: HealthLevel,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub days_remaining: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fingerprint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub issuer: Option<String>,
}

/// 订阅健康状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionHealth {
    pub status: HealthLevel,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plan: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subscription_status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature_valid_until: Option<String>,
    pub needs_refresh: bool,
}

/// 网络健康状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkHealth {
    pub status: HealthLevel,
    pub auth_server_reachable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_connected_at: Option<String>,
}

/// 数据库健康状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseHealth {
    pub status: HealthLevel,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_write_at: Option<String>,
}

/// 组件健康状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentsHealth {
    pub certificate: CertificateHealth,
    pub subscription: SubscriptionHealth,
    pub network: NetworkHealth,
    pub database: DatabaseHealth,
}

/// 设备信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub device_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tenant_id: Option<String>,
}

/// 健康检查响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    pub overall: HealthLevel,
    pub components: ComponentsHealth,
    pub checked_at: String,
    pub device_info: DeviceInfo,
}
```

**Step 2: 在 lib.rs 中导出模块**

修改 `shared/src/lib.rs`，在模块列表中添加：

```rust
pub mod app_state;
```

并在 re-exports 部分添加：

```rust
// App state re-exports
pub use app_state::{
    ActivationProgress, ActivationRequiredReason, ActivationStep, CertificateHealth,
    ClockDirection, ComponentsHealth, DatabaseHealth, DeviceInfo, HealthLevel, HealthStatus,
    NetworkHealth, SubscriptionBlockedInfo, SubscriptionHealth,
};
```

**Step 3: 验证编译**

Run: `cargo check -p shared`
Expected: 编译通过

**Step 4: Commit**

```bash
git add shared/src/app_state.rs shared/src/lib.rs
git commit -m "feat(shared): add detailed app state types for tenant auth"
```

---

## Task 2: 更新 Tauri AppState 类型

**Files:**
- Modify: `red_coral/src-tauri/src/core/bridge/types.rs`

**Step 1: 更新 AppState 枚举**

替换 `red_coral/src-tauri/src/core/bridge/types.rs` 中的 `AppState` 枚举：

```rust
//! Bridge type definitions

use serde::{Deserialize, Serialize};

use crab_client::{Authenticated, Connected, CrabClient, Local, Remote};
use shared::app_state::{ActivationProgress, ActivationRequiredReason, SubscriptionBlockedInfo};

/// 运行模式类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModeType {
    Server,
    Client,
    Disconnected,
}

impl std::fmt::Display for ModeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ModeType::Server => write!(f, "server"),
            ModeType::Client => write!(f, "client"),
            ModeType::Disconnected => write!(f, "disconnected"),
        }
    }
}

/// 应用状态 (统一 Server/Client 模式)
///
/// 用于前端路由守卫和状态展示。
/// 参考设计文档: `docs/plans/2026-01-26-tenant-auth-detailed-status-design.md`
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum AppState {
    // === 通用状态 ===
    /// 未初始化
    Uninitialized,

    // === Server 模式专属 ===
    /// Server: 无租户
    ServerNoTenant,

    /// Server: 需要激活 - 携带详细原因
    ServerNeedActivation {
        reason: ActivationRequiredReason,
        can_auto_recover: bool,
        recovery_hint: String,
    },

    /// Server: 正在激活 - 携带进度
    ServerActivating { progress: ActivationProgress },

    /// Server: 已激活，验证订阅中
    ServerCheckingSubscription,

    /// Server: 订阅无效 - 携带详细信息
    ServerSubscriptionBlocked { info: SubscriptionBlockedInfo },

    /// Server: 服务器就绪，等待员工登录
    ServerReady,

    /// Server: 员工已登录
    ServerAuthenticated,

    // === Client 模式专属 ===
    /// Client: 未连接
    ClientDisconnected,

    /// Client: 需要设置 (无缓存证书)
    ClientNeedSetup,

    /// Client: 正在连接
    ClientConnecting { server_url: String },

    /// Client: 已连接，等待员工登录
    ClientConnected,

    /// Client: 员工已登录
    ClientAuthenticated,
}

impl AppState {
    /// 是否可以访问 POS 主界面
    pub fn can_access_pos(&self) -> bool {
        matches!(
            self,
            AppState::ServerAuthenticated | AppState::ClientAuthenticated
        )
    }

    /// 是否需要员工登录
    pub fn needs_employee_login(&self) -> bool {
        matches!(self, AppState::ServerReady | AppState::ClientConnected)
    }

    /// 是否需要设置/激活
    pub fn needs_setup(&self) -> bool {
        matches!(
            self,
            AppState::Uninitialized
                | AppState::ServerNoTenant
                | AppState::ServerNeedActivation { .. }
                | AppState::ClientDisconnected
                | AppState::ClientNeedSetup
        )
    }

    /// 是否被订阅阻止
    pub fn is_subscription_blocked(&self) -> bool {
        matches!(self, AppState::ServerSubscriptionBlocked { .. })
    }
}

// ... 保留 ModeInfo 和其他类型不变 ...
```

**Step 2: 验证编译**

Run: `cd red_coral && cargo check -p red_coral-tauri`
Expected: 编译错误 (get_app_state 函数需要更新)

**Step 3: Commit (WIP)**

```bash
git add red_coral/src-tauri/src/core/bridge/types.rs
git commit -m "wip: update AppState with detailed reason types"
```

---

## Task 3: 更新 get_app_state 逻辑

**Files:**
- Modify: `red_coral/src-tauri/src/core/bridge/mod.rs`

**Step 1: 导入新类型**

在文件顶部添加导入：

```rust
use shared::app_state::{
    ActivationProgress, ActivationRequiredReason, ActivationStep, ClockDirection,
    SubscriptionBlockedInfo,
};
use shared::activation::{PlanType, SubscriptionStatus};
```

**Step 2: 创建辅助函数检测激活原因**

在 `impl ClientBridge` 中添加：

```rust
/// 检测需要激活的具体原因
async fn detect_activation_reason(
    &self,
    tenant_manager: &TenantManager,
) -> ActivationRequiredReason {
    // 1. 检查是否有证书
    let cert_manager = match tenant_manager.current_cert_manager() {
        Some(cm) => cm,
        None => return ActivationRequiredReason::FirstTimeSetup,
    };

    // 2. 检查证书是否存在
    if !cert_manager.has_local_certificates() {
        return ActivationRequiredReason::FirstTimeSetup;
    }

    // 3. 执行自检
    match cert_manager.self_check() {
        Ok(()) => {
            // 证书有效，检查是否快过期
            if let Ok((cert_pem, _, _)) = cert_manager.load_local_certificates() {
                if let Ok(metadata) = crab_cert::CertMetadata::from_pem(&cert_pem) {
                    let days_remaining = metadata.days_until_expiry();
                    if days_remaining <= 30 && days_remaining > 0 {
                        return ActivationRequiredReason::CertificateExpiringSoon {
                            expires_at: metadata.not_after.clone(),
                            days_remaining,
                        };
                    }
                }
            }
            // 证书有效且不快过期，可能是其他原因
            ActivationRequiredReason::FirstTimeSetup
        }
        Err(e) => {
            let error_str = e.to_string();
            
            // 解析具体错误类型
            if error_str.contains("expired") || error_str.contains("Expired") {
                // 尝试获取过期信息
                if let Ok((cert_pem, _, _)) = cert_manager.load_local_certificates() {
                    if let Ok(metadata) = crab_cert::CertMetadata::from_pem(&cert_pem) {
                        let days_overdue = -metadata.days_until_expiry();
                        return ActivationRequiredReason::CertificateExpired {
                            expired_at: metadata.not_after.clone(),
                            days_overdue,
                        };
                    }
                }
                ActivationRequiredReason::CertificateExpired {
                    expired_at: "unknown".to_string(),
                    days_overdue: 0,
                }
            } else if error_str.contains("Device ID mismatch") || error_str.contains("device_id") {
                let parts: Vec<&str> = error_str.split("expected ").collect();
                let (expected, actual) = if parts.len() > 1 {
                    let rest = parts[1];
                    let parts2: Vec<&str> = rest.split(", got ").collect();
                    if parts2.len() == 2 {
                        (parts2[0].to_string(), parts2[1].to_string())
                    } else {
                        ("***".to_string(), "***".to_string())
                    }
                } else {
                    ("***".to_string(), "***".to_string())
                };
                ActivationRequiredReason::DeviceMismatch { expected, actual }
            } else if error_str.contains("Clock tampering") || error_str.contains("clock") {
                let direction = if error_str.contains("backward") {
                    ClockDirection::Backward
                } else {
                    ClockDirection::Forward
                };
                // 提取秒数
                let drift_seconds = error_str
                    .split_whitespace()
                    .find_map(|s| s.parse::<i64>().ok())
                    .unwrap_or(0);
                ActivationRequiredReason::ClockTampering {
                    direction,
                    drift_seconds,
                    last_verified_at: "unknown".to_string(),
                }
            } else if error_str.contains("signature") || error_str.contains("Signature") {
                ActivationRequiredReason::SignatureInvalid {
                    component: "certificate".to_string(),
                    error: error_str,
                }
            } else {
                ActivationRequiredReason::CertificateInvalid { error: error_str }
            }
        }
    }
}
```

**Step 3: 更新 get_app_state 函数**

修改 `get_app_state` 函数，在返回 `ServerNeedActivation` 的地方改为携带详细原因：

```rust
pub async fn get_app_state(&self) -> AppState {
    let mode_guard = self.mode.read().await;
    let tenant_manager = self.tenant_manager.read().await;

    match &*mode_guard {
        ClientMode::Disconnected => {
            if tenant_manager.current_tenant_id().is_none() {
                AppState::ServerNoTenant
            } else {
                let has_certs = tenant_manager
                    .current_cert_manager()
                    .map(|cm| cm.has_local_certificates())
                    .unwrap_or(false);

                if has_certs {
                    AppState::Uninitialized
                } else {
                    let reason = ActivationRequiredReason::FirstTimeSetup;
                    AppState::ServerNeedActivation {
                        can_auto_recover: reason.can_auto_recover(),
                        recovery_hint: reason.recovery_hint().to_string(),
                        reason,
                    }
                }
            }
        }

        ClientMode::Server {
            server_state,
            client,
            ..
        } => {
            let is_activated = server_state.is_activated().await;

            if !is_activated {
                // 检测具体原因
                drop(mode_guard); // 释放锁以避免死锁
                let reason = self.detect_activation_reason(&tenant_manager).await;
                return AppState::ServerNeedActivation {
                    can_auto_recover: reason.can_auto_recover(),
                    recovery_hint: reason.recovery_hint().to_string(),
                    reason,
                };
            }

            let credential = server_state
                .activation_service()
                .get_credential()
                .await
                .ok()
                .flatten();

            if let Some(cred) = credential {
                let subscription_blocked = cred.subscription.as_ref().is_some_and(|sub| {
                    matches!(
                        sub.status,
                        SubscriptionStatus::Canceled | SubscriptionStatus::Unpaid
                    )
                });

                if subscription_blocked {
                    let sub = cred.subscription.as_ref().unwrap();
                    let info = SubscriptionBlockedInfo {
                        status: sub.status.clone(),
                        plan: sub.plan.clone(),
                        expired_at: sub.expires_at.clone(),
                        grace_period_days: None,
                        grace_period_ends_at: None,
                        in_grace_period: false,
                        support_url: Some("https://support.example.com".to_string()),
                        renewal_url: Some("https://billing.example.com/renew".to_string()),
                        user_message: match sub.status {
                            SubscriptionStatus::Canceled => "订阅已取消".to_string(),
                            SubscriptionStatus::Unpaid => "订阅欠费".to_string(),
                            _ => format!("订阅状态异常: {:?}", sub.status),
                        },
                    };
                    AppState::ServerSubscriptionBlocked { info }
                } else {
                    // ... 保持原有的 client/session 检查逻辑 ...
                    match client {
                        Some(LocalClientState::Authenticated(_)) => {
                            AppState::ServerAuthenticated
                        }
                        _ => {
                            if let Some(session) = tenant_manager.current_session() {
                                let expires_at = session.expires_at.or_else(|| {
                                    super::session_cache::EmployeeSession::parse_jwt_exp(
                                        &session.token,
                                    )
                                });

                                if let Some(exp) = expires_at {
                                    let now = std::time::SystemTime::now()
                                        .duration_since(std::time::UNIX_EPOCH)
                                        .map(|d| d.as_secs())
                                        .unwrap_or(0);
                                    if now >= exp {
                                        return AppState::ServerReady;
                                    }
                                }
                                AppState::ServerAuthenticated
                            } else {
                                AppState::ServerReady
                            }
                        }
                    }
                }
            } else {
                let reason = self.detect_activation_reason(&tenant_manager).await;
                AppState::ServerNeedActivation {
                    can_auto_recover: reason.can_auto_recover(),
                    recovery_hint: reason.recovery_hint().to_string(),
                    reason,
                }
            }
        }

        ClientMode::Client { client, .. } => {
            // ... 保持原有逻辑 ...
            match client {
                Some(RemoteClientState::Authenticated(_)) => AppState::ClientAuthenticated,
                Some(RemoteClientState::Connected(_)) => AppState::ClientConnected,
                None => {
                    let has_certs = tenant_manager
                        .current_cert_manager()
                        .map(|cm| cm.has_local_certificates())
                        .unwrap_or(false);

                    if has_certs {
                        AppState::ClientDisconnected
                    } else {
                        AppState::ClientNeedSetup
                    }
                }
            }
        }
    }
}
```

**Step 4: 验证编译**

Run: `cd red_coral && cargo check -p red_coral-tauri`
Expected: 编译通过

**Step 5: Commit**

```bash
git add red_coral/src-tauri/src/core/bridge/mod.rs
git commit -m "feat(tauri): return detailed activation reasons in get_app_state"
```

---

## Task 4: 添加健康检查命令

**Files:**
- Create: `red_coral/src-tauri/src/commands/health.rs`
- Modify: `red_coral/src-tauri/src/commands/mod.rs`
- Modify: `red_coral/src-tauri/src/lib.rs`

**Step 1: 创建 health.rs**

```rust
// red_coral/src-tauri/src/commands/health.rs
//! 健康检查命令

use std::sync::Arc;
use tauri::State;
use tokio::sync::RwLock;

use crate::core::bridge::ClientBridge;
use crate::core::response::ApiResponse;
use shared::app_state::{
    CertificateHealth, ComponentsHealth, DatabaseHealth, DeviceInfo, HealthLevel, HealthStatus,
    NetworkHealth, SubscriptionHealth,
};

/// 获取系统健康状态
#[tauri::command]
pub async fn get_health_status(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
) -> Result<ApiResponse<HealthStatus>, String> {
    let bridge = bridge.read().await;
    let tenant_manager = bridge.tenant_manager().read().await;

    // 获取设备信息
    let device_id = crab_cert::generate_hardware_id();
    let tenant_id = tenant_manager.current_tenant_id().map(|s| s.to_string());

    // 检查证书健康状态
    let certificate = if let Some(cm) = tenant_manager.current_cert_manager() {
        if cm.has_local_certificates() {
            match cm.load_local_certificates() {
                Ok((cert_pem, _, _)) => {
                    match crab_cert::CertMetadata::from_pem(&cert_pem) {
                        Ok(metadata) => {
                            let days_remaining = metadata.days_until_expiry();
                            let status = if days_remaining < 0 {
                                HealthLevel::Critical
                            } else if days_remaining <= 30 {
                                HealthLevel::Warning
                            } else {
                                HealthLevel::Healthy
                            };
                            CertificateHealth {
                                status,
                                expires_at: Some(metadata.not_after.clone()),
                                days_remaining: Some(days_remaining),
                                fingerprint: Some(metadata.fingerprint.clone()),
                                issuer: Some(metadata.issuer.clone()),
                            }
                        }
                        Err(_) => CertificateHealth {
                            status: HealthLevel::Critical,
                            expires_at: None,
                            days_remaining: None,
                            fingerprint: None,
                            issuer: None,
                        },
                    }
                }
                Err(_) => CertificateHealth {
                    status: HealthLevel::Critical,
                    expires_at: None,
                    days_remaining: None,
                    fingerprint: None,
                    issuer: None,
                },
            }
        } else {
            CertificateHealth {
                status: HealthLevel::Unknown,
                expires_at: None,
                days_remaining: None,
                fingerprint: None,
                issuer: None,
            }
        }
    } else {
        CertificateHealth {
            status: HealthLevel::Unknown,
            expires_at: None,
            days_remaining: None,
            fingerprint: None,
            issuer: None,
        }
    };

    // 检查订阅健康状态 (简化)
    let subscription = SubscriptionHealth {
        status: HealthLevel::Unknown,
        plan: None,
        subscription_status: None,
        signature_valid_until: None,
        needs_refresh: false,
    };

    // 检查网络健康状态 (简化)
    let network = NetworkHealth {
        status: HealthLevel::Unknown,
        auth_server_reachable: false,
        last_connected_at: None,
    };

    // 检查数据库健康状态 (简化)
    let database = DatabaseHealth {
        status: HealthLevel::Healthy,
        size_bytes: None,
        last_write_at: None,
    };

    // 计算整体健康状态
    let overall = if certificate.status == HealthLevel::Critical {
        HealthLevel::Critical
    } else if certificate.status == HealthLevel::Warning {
        HealthLevel::Warning
    } else {
        HealthLevel::Healthy
    };

    let health = HealthStatus {
        overall,
        components: ComponentsHealth {
            certificate,
            subscription,
            network,
            database,
        },
        checked_at: chrono::Utc::now().to_rfc3339(),
        device_info: DeviceInfo {
            device_id: format!("{}...", &device_id[..8]),
            entity_id: None,
            tenant_id,
        },
    };

    Ok(ApiResponse::success(health))
}
```

**Step 2: 在 mod.rs 中导出**

在 `red_coral/src-tauri/src/commands/mod.rs` 添加：

```rust
pub mod health;
pub use health::*;
```

**Step 3: 在 lib.rs 中注册命令**

在 `red_coral/src-tauri/src/lib.rs` 的 `invoke_handler` 中添加 `get_health_status`。

**Step 4: 验证编译**

Run: `cd red_coral && cargo check -p red_coral-tauri`
Expected: 编译通过

**Step 5: Commit**

```bash
git add red_coral/src-tauri/src/commands/health.rs red_coral/src-tauri/src/commands/mod.rs red_coral/src-tauri/src/lib.rs
git commit -m "feat(tauri): add get_health_status command"
```

---

## Task 5: 更新前端 TypeScript 类型

**Files:**
- Create: `red_coral/src/core/domain/types/appState.ts`
- Modify: `red_coral/src/core/domain/types/index.ts`
- Modify: `red_coral/src/core/stores/bridge/useBridgeStore.ts`

**Step 1: 创建 appState.ts**

```typescript
// red_coral/src/core/domain/types/appState.ts
/**
 * 应用状态类型定义
 * 
 * 与 Rust 定义保持一致: shared/src/app_state.rs
 */

// =============================================================================
// 激活失败原因
// =============================================================================

export type ClockDirection = 'backward' | 'forward';

export type ActivationRequiredReason =
  | { code: 'FirstTimeSetup' }
  | { code: 'CertificateExpired'; details: { expired_at: string; days_overdue: number } }
  | { code: 'CertificateExpiringSoon'; details: { expires_at: string; days_remaining: number } }
  | { code: 'CertificateInvalid'; details: { error: string } }
  | { code: 'SignatureInvalid'; details: { component: string; error: string } }
  | { code: 'DeviceMismatch'; details: { expected: string; actual: string } }
  | { code: 'ClockTampering'; details: { direction: ClockDirection; drift_seconds: number; last_verified_at: string } }
  | { code: 'BindingInvalid'; details: { error: string } }
  | { code: 'TokenExpired'; details: { expired_at: string } }
  | { code: 'NetworkError'; details: { error: string; can_continue_offline: boolean } }
  | { code: 'Revoked'; details: { revoked_at: string; reason: string } };

// =============================================================================
// 订阅阻止信息
// =============================================================================

export type SubscriptionStatus = 'active' | 'trial' | 'past_due' | 'canceled' | 'unpaid';
export type PlanType = 'free' | 'pro' | 'enterprise';

export interface SubscriptionBlockedInfo {
  status: SubscriptionStatus;
  plan: PlanType;
  expired_at?: string;
  grace_period_days?: number;
  grace_period_ends_at?: string;
  in_grace_period: boolean;
  support_url?: string;
  renewal_url?: string;
  user_message: string;
}

// =============================================================================
// 激活进度
// =============================================================================

export type ActivationStep =
  | 'authenticating'
  | 'downloading_certificates'
  | 'verifying_binding'
  | 'checking_subscription'
  | 'starting_server'
  | 'complete';

export interface ActivationProgress {
  step: ActivationStep;
  total_steps: number;
  current_step: number;
  message: string;
  started_at: string;
}

// =============================================================================
// 健康检查
// =============================================================================

export type HealthLevel = 'healthy' | 'warning' | 'critical' | 'unknown';

export interface CertificateHealth {
  status: HealthLevel;
  expires_at?: string;
  days_remaining?: number;
  fingerprint?: string;
  issuer?: string;
}

export interface SubscriptionHealth {
  status: HealthLevel;
  plan?: string;
  subscription_status?: string;
  signature_valid_until?: string;
  needs_refresh: boolean;
}

export interface NetworkHealth {
  status: HealthLevel;
  auth_server_reachable: boolean;
  last_connected_at?: string;
}

export interface DatabaseHealth {
  status: HealthLevel;
  size_bytes?: number;
  last_write_at?: string;
}

export interface ComponentsHealth {
  certificate: CertificateHealth;
  subscription: SubscriptionHealth;
  network: NetworkHealth;
  database: DatabaseHealth;
}

export interface DeviceInfo {
  device_id: string;
  entity_id?: string;
  tenant_id?: string;
}

export interface HealthStatus {
  overall: HealthLevel;
  components: ComponentsHealth;
  checked_at: string;
  device_info: DeviceInfo;
}

// =============================================================================
// AppState
// =============================================================================

export type AppState =
  // 通用状态
  | { type: 'Uninitialized' }
  // Server 模式
  | { type: 'ServerNoTenant' }
  | { type: 'ServerNeedActivation'; data: { reason: ActivationRequiredReason; can_auto_recover: boolean; recovery_hint: string } }
  | { type: 'ServerActivating'; data: { progress: ActivationProgress } }
  | { type: 'ServerCheckingSubscription' }
  | { type: 'ServerSubscriptionBlocked'; data: { info: SubscriptionBlockedInfo } }
  | { type: 'ServerReady' }
  | { type: 'ServerAuthenticated' }
  // Client 模式
  | { type: 'ClientDisconnected' }
  | { type: 'ClientNeedSetup' }
  | { type: 'ClientConnecting'; data: { server_url: string } }
  | { type: 'ClientConnected' }
  | { type: 'ClientAuthenticated' };

// =============================================================================
// 辅助函数
// =============================================================================

/** 获取激活原因的用户友好消息 */
export function getActivationReasonMessage(reason: ActivationRequiredReason): string {
  switch (reason.code) {
    case 'FirstTimeSetup':
      return '欢迎！请激活您的设备';
    case 'CertificateExpired':
      return `设备证书已过期 ${reason.details.days_overdue} 天`;
    case 'CertificateExpiringSoon':
      return `证书将在 ${reason.details.days_remaining} 天后过期`;
    case 'CertificateInvalid':
      return '证书文件损坏或无效';
    case 'SignatureInvalid':
      return '安全验证失败';
    case 'DeviceMismatch':
      return '检测到硬件变更';
    case 'ClockTampering':
      return reason.details.direction === 'backward'
        ? `系统时间异常：回拨了 ${Math.floor(reason.details.drift_seconds / 3600)} 小时`
        : `系统时间异常：前跳了 ${Math.floor(reason.details.drift_seconds / 86400)} 天`;
    case 'BindingInvalid':
      return '设备绑定无效';
    case 'TokenExpired':
      return '凭据已过期';
    case 'NetworkError':
      return '无法连接服务器';
    case 'Revoked':
      return '此设备已被停用';
    default:
      return '需要重新激活';
  }
}
```

**Step 2: 更新 index.ts**

在 `red_coral/src/core/domain/types/index.ts` 添加导出：

```typescript
export * from './appState';
```

**Step 3: 更新 useBridgeStore.ts**

修改 `red_coral/src/core/stores/bridge/useBridgeStore.ts`：

1. 导入新类型：
```typescript
import type { AppState, HealthStatus, ActivationRequiredReason } from '@/core/domain/types';
import { getActivationReasonMessage } from '@/core/domain/types';
```

2. 删除原有的 `AppState` 类型定义

3. 更新 `getRouteForState` 函数：
```typescript
getRouteForState: (state: AppState | null): string => {
  if (!state) return '/setup';

  switch (state.type) {
    // 首次设置
    case 'ServerNoTenant':
      return '/setup';

    // 需要激活 - 显示具体原因
    case 'ServerNeedActivation':
      return '/status/activation-required';

    // 激活中 - 显示进度
    case 'ServerActivating':
      return '/status/activating';

    // 检查订阅
    case 'ServerCheckingSubscription':
      return '/status/checking';

    // 订阅阻止
    case 'ServerSubscriptionBlocked':
      return '/status/subscription-blocked';

    // 未初始化
    case 'Uninitialized':
    case 'ClientDisconnected':
    case 'ClientNeedSetup':
    case 'ClientConnecting':
      return '/setup';

    // 需要登录
    case 'ServerReady':
    case 'ClientConnected':
      return '/login';

    // 可以进入 POS
    case 'ServerAuthenticated':
    case 'ClientAuthenticated':
      return '/pos';

    default:
      return '/setup';
  }
},
```

4. 添加 `fetchHealthStatus` action：
```typescript
fetchHealthStatus: async () => {
  try {
    return await invokeApi<HealthStatus>('get_health_status');
  } catch (error) {
    console.error('Failed to fetch health status:', error);
    return null;
  }
},
```

**Step 4: 验证 TypeScript 编译**

Run: `cd red_coral && npx tsc --noEmit`
Expected: 编译通过

**Step 5: Commit**

```bash
git add red_coral/src/core/domain/types/appState.ts red_coral/src/core/domain/types/index.ts red_coral/src/core/stores/bridge/useBridgeStore.ts
git commit -m "feat(frontend): add detailed AppState types and update routing"
```

---

## Task 6: 创建状态显示组件

**Files:**
- Create: `red_coral/src/screens/Status/ActivationRequiredScreen.tsx`
- Create: `red_coral/src/screens/Status/index.ts`
- Modify: `red_coral/src/App.tsx`

**Step 1: 创建 ActivationRequiredScreen**

```tsx
// red_coral/src/screens/Status/ActivationRequiredScreen.tsx
import React from 'react';
import { useNavigate } from 'react-router-dom';
import { AlertTriangle, RefreshCw, Settings, Clock, Shield, Wifi } from 'lucide-react';
import { useBridgeStore, useAppState } from '@/core/stores/bridge';
import { getActivationReasonMessage } from '@/core/domain/types';

export const ActivationRequiredScreen: React.FC = () => {
  const navigate = useNavigate();
  const appState = useAppState();
  const { isLoading } = useBridgeStore();

  if (appState?.type !== 'ServerNeedActivation') {
    return null;
  }

  const { reason, can_auto_recover, recovery_hint } = appState.data;
  const message = getActivationReasonMessage(reason);

  const getIcon = () => {
    switch (reason.code) {
      case 'ClockTampering':
        return <Clock className="text-yellow-500" size={48} />;
      case 'DeviceMismatch':
      case 'SignatureInvalid':
        return <Shield className="text-red-500" size={48} />;
      case 'NetworkError':
        return <Wifi className="text-orange-500" size={48} />;
      default:
        return <AlertTriangle className="text-yellow-500" size={48} />;
    }
  };

  const handleReactivate = () => {
    navigate('/setup', { replace: true });
  };

  return (
    <div className="min-h-screen w-full flex items-center justify-center p-8 bg-gray-50">
      <div className="max-w-md w-full bg-white rounded-2xl shadow-lg p-8">
        <div className="text-center mb-6">
          <div className="inline-flex items-center justify-center w-20 h-20 bg-gray-100 rounded-full mb-4">
            {getIcon()}
          </div>
          <h1 className="text-2xl font-bold text-gray-900 mb-2">
            需要重新激活
          </h1>
          <p className="text-lg text-gray-600">
            {message}
          </p>
        </div>

        <div className="bg-gray-50 rounded-xl p-4 mb-6">
          <p className="text-sm text-gray-600">
            <strong>建议操作：</strong> {recovery_hint}
          </p>
        </div>

        {reason.code === 'CertificateExpired' && 'details' in reason && (
          <div className="bg-red-50 border border-red-100 rounded-xl p-4 mb-6">
            <p className="text-sm text-red-600">
              证书过期时间：{reason.details.expired_at}
            </p>
          </div>
        )}

        {reason.code === 'ClockTampering' && 'details' in reason && (
          <div className="bg-yellow-50 border border-yellow-100 rounded-xl p-4 mb-6">
            <p className="text-sm text-yellow-700">
              检测到系统时间{reason.details.direction === 'backward' ? '回拨' : '前跳'}
              ，请检查系统时间设置。
            </p>
          </div>
        )}

        <div className="space-y-3">
          <button
            onClick={handleReactivate}
            disabled={isLoading}
            className="w-full py-3 bg-[#FF5E5E] text-white font-bold rounded-xl hover:bg-[#E54545] active:scale-[0.98] transition-all flex items-center justify-center gap-2"
          >
            <RefreshCw size={20} />
            重新激活设备
          </button>

          {can_auto_recover && (
            <button
              onClick={() => window.location.reload()}
              disabled={isLoading}
              className="w-full py-3 bg-gray-100 text-gray-700 font-medium rounded-xl hover:bg-gray-200 transition-all"
            >
              稍后再试
            </button>
          )}
        </div>
      </div>
    </div>
  );
};

export default ActivationRequiredScreen;
```

**Step 2: 创建 index.ts**

```typescript
// red_coral/src/screens/Status/index.ts
export { default as ActivationRequiredScreen } from './ActivationRequiredScreen';
```

**Step 3: 添加路由**

在 `red_coral/src/App.tsx` 中添加路由：

```tsx
import { ActivationRequiredScreen } from '@/screens/Status';

// 在 Routes 中添加
<Route path="/status/activation-required" element={<ActivationRequiredScreen />} />
```

**Step 4: 验证**

Run: `cd red_coral && npm run dev`
Expected: 应用启动，新路由可访问

**Step 5: Commit**

```bash
git add red_coral/src/screens/Status/
git add red_coral/src/App.tsx
git commit -m "feat(frontend): add ActivationRequiredScreen with detailed status"
```

---

## 验证清单

- [ ] `cargo check --workspace` 通过
- [ ] `cd red_coral && cargo check -p red_coral-tauri` 通过
- [ ] `cd red_coral && npx tsc --noEmit` 通过
- [ ] `cd red_coral && npm run dev` 应用启动正常
- [ ] 模拟证书过期场景，验证显示正确的错误信息

---

## 总结

| Task | 文件 | 描述 |
|------|------|------|
| 1 | `shared/src/app_state.rs` | 定义状态类型 |
| 2 | `red_coral/src-tauri/.../types.rs` | 更新 AppState 枚举 |
| 3 | `red_coral/src-tauri/.../mod.rs` | 更新 get_app_state 逻辑 |
| 4 | `red_coral/src-tauri/.../health.rs` | 添加健康检查命令 |
| 5 | `red_coral/src/.../appState.ts` | TypeScript 类型 |
| 6 | `red_coral/src/screens/Status/` | 状态显示组件 |
