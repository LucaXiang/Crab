# Tenant 认证状态详细信息设计

> 日期: 2026-01-26
> 状态: 草案
> 作者: Claude

## 背景

当前 red_coral (Tauri POS) 在 Server 模式下，当 Tenant 认证状态有问题时，直接跳转到 `/setup` 页面，用户不知道发生了什么。

### 当前问题

1. **AppState 缺少详细原因**
   - `ServerNeedActivation` 没有携带失败原因
   - 用户看到的都是同一个"选择模式"界面

2. **路由映射过于粗暴**
   ```typescript
   // 所有这些状态都去 /setup
   case 'ServerNeedActivation':
   case 'ServerActivating':
   case 'ServerCheckingSubscription':
     return '/setup';
   ```

3. **前端无法显示有意义的信息**
   - 证书过期？设备变更？时钟异常？用户一无所知

## 设计目标

1. 后端传递详细的状态原因和上下文信息
2. 前端能够根据原因显示不同的 UI 和恢复建议
3. 提供健康检查 API 用于诊断

---

## 详细设计

### 1. 激活失败原因枚举

```rust
// shared/src/client/mod.rs 或新文件

/// 需要激活的原因
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "code", content = "details")]
pub enum ActivationRequiredReason {
    /// 首次激活 (无本地凭据)
    FirstTimeSetup,
    
    /// 证书过期
    CertificateExpired {
        /// 过期时间 (RFC3339)
        expired_at: String,
        /// 已过期天数
        days_overdue: i64,
    },
    
    /// 证书即将过期 (警告状态，可继续使用)
    CertificateExpiringSoon {
        /// 到期时间 (RFC3339)
        expires_at: String,
        /// 剩余天数
        days_remaining: i64,
    },
    
    /// 证书文件缺失或损坏
    CertificateInvalid {
        /// 错误描述
        error: String,
    },
    
    /// 签名验证失败
    SignatureInvalid {
        /// 失败的组件: "binding" | "credential" | "subscription"
        component: String,
        /// 错误描述
        error: String,
    },
    
    /// 硬件 ID 不匹配 (设备克隆/迁移检测)
    DeviceMismatch {
        /// 原绑定的 device_id (脱敏显示)
        expected: String,
        /// 当前机器的 device_id (脱敏显示)
        actual: String,
    },
    
    /// 时钟篡改检测
    ClockTampering {
        /// 时钟偏移方向
        direction: ClockDirection,
        /// 偏移秒数
        drift_seconds: i64,
        /// 上次验证时间
        last_verified_at: String,
    },
    
    /// Binding 验证失败 (通用)
    BindingInvalid {
        error: String,
    },
    
    /// Identity Token 过期 (10年长期凭据)
    TokenExpired {
        expired_at: String,
    },
    
    /// 网络错误导致无法验证
    NetworkError {
        error: String,
        /// 是否可以离线继续 (基于本地缓存)
        can_continue_offline: bool,
    },
    
    /// 设备已被远程吊销
    Revoked {
        /// 吊销时间
        revoked_at: String,
        /// 吊销原因 (管理员备注)
        reason: String,
    },
}

/// 时钟偏移方向
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClockDirection {
    /// 时钟回拨 (可能试图延长过期时间)
    Backward,
    /// 时钟前跳 (可能试图跳过在线验证)
    Forward,
}
```

### 2. 订阅阻止详情

```rust
/// 订阅阻止详细信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionBlockedInfo {
    /// 订阅状态
    pub status: SubscriptionStatus,
    
    /// 计划类型
    pub plan: PlanType,
    
    /// 过期时间 (如果已过期)
    pub expired_at: Option<String>,
    
    /// 宽限期天数 (如果适用)
    pub grace_period_days: Option<i64>,
    
    /// 宽限期结束时间
    pub grace_period_ends_at: Option<String>,
    
    /// 是否在宽限期内 (可降级使用)
    pub in_grace_period: bool,
    
    /// 联系支持的 URL
    pub support_url: Option<String>,
    
    /// 续费 URL
    pub renewal_url: Option<String>,
    
    /// 商户友好的消息
    pub user_message: String,
}
```

### 3. 激活进度

```rust
/// 激活进度信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivationProgress {
    /// 当前步骤
    pub step: ActivationStep,
    /// 总步骤数
    pub total_steps: u8,
    /// 当前步骤序号 (1-based)
    pub current_step: u8,
    /// 当前步骤的描述消息
    pub message: String,
    /// 当前步骤开始时间
    pub started_at: String,
}

/// 激活步骤
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActivationStep {
    /// 验证凭据
    Authenticating,
    /// 下载证书
    DownloadingCertificates,
    /// 验证设备绑定
    VerifyingBinding,
    /// 检查订阅状态
    CheckingSubscription,
    /// 启动边缘服务
    StartingServer,
    /// 完成
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
}
```

### 4. 改进后的 AppState

```rust
// red_coral/src-tauri/src/core/bridge/types.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum AppState {
    // === 通用状态 ===
    Uninitialized,

    // === Server 模式 ===
    
    /// 无租户配置
    ServerNoTenant,
    
    /// 需要激活 - 携带详细原因
    ServerNeedActivation {
        /// 需要激活的原因
        reason: ActivationRequiredReason,
        /// 是否可以尝试自动恢复 (如刷新证书)
        can_auto_recover: bool,
        /// 给用户的恢复建议
        recovery_hint: String,
    },
    
    /// 正在激活 - 携带进度
    ServerActivating {
        progress: ActivationProgress,
    },
    
    /// 检查订阅中
    ServerCheckingSubscription,
    
    /// 订阅阻止 - 携带详细信息
    ServerSubscriptionBlocked {
        info: SubscriptionBlockedInfo,
    },
    
    /// 服务器就绪，等待员工登录
    ServerReady,
    
    /// 员工已登录
    ServerAuthenticated,

    // === Client 模式 ===
    
    ClientDisconnected,
    ClientNeedSetup,
    ClientConnecting { 
        server_url: String,
    },
    ClientConnected,
    ClientAuthenticated,
}
```

### 5. 健康检查 API (新增)

```rust
/// 健康检查响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    /// 整体健康状态
    pub overall: HealthLevel,
    
    /// 各组件详细状态
    pub components: ComponentsHealth,
    
    /// 检查时间
    pub checked_at: String,
    
    /// 设备信息
    pub device_info: DeviceInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentsHealth {
    /// 证书健康状态
    pub certificate: CertificateHealth,
    
    /// 订阅健康状态
    pub subscription: SubscriptionHealth,
    
    /// 网络连接状态
    pub network: NetworkHealth,
    
    /// 数据库状态
    pub database: DatabaseHealth,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CertificateHealth {
    pub status: HealthLevel,
    /// 证书到期时间
    pub expires_at: Option<String>,
    /// 剩余有效天数
    pub days_remaining: Option<i64>,
    /// 证书指纹 (用于核对)
    pub fingerprint: Option<String>,
    /// 签发者
    pub issuer: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionHealth {
    pub status: HealthLevel,
    pub plan: Option<String>,
    pub subscription_status: Option<String>,
    /// 签名有效期
    pub signature_valid_until: Option<String>,
    /// 需要刷新
    pub needs_refresh: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkHealth {
    pub status: HealthLevel,
    /// 到 auth-server 的连通性
    pub auth_server_reachable: bool,
    /// 最后成功连接时间
    pub last_connected_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseHealth {
    pub status: HealthLevel,
    /// 数据库大小
    pub size_bytes: Option<u64>,
    /// 最后写入时间
    pub last_write_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    /// 设备 ID (脱敏)
    pub device_id: String,
    /// 实体 ID
    pub entity_id: Option<String>,
    /// 租户 ID
    pub tenant_id: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum HealthLevel {
    /// 一切正常
    Healthy,
    /// 有警告 (如证书快过期)
    Warning,
    /// 严重问题 (如证书已过期)
    Critical,
    /// 无法确定
    Unknown,
}
```

---

## 前端展示映射

### 1. 激活原因 → 用户消息

| 原因 | 用户看到的消息 | 恢复建议 |
|------|--------------|---------|
| `FirstTimeSetup` | "欢迎！请激活您的设备" | "输入管理员提供的凭据完成激活" |
| `CertificateExpired { days_overdue: 30 }` | "设备证书已过期 30 天" | "请重新激活设备以更新证书" |
| `CertificateExpiringSoon { days_remaining: 7 }` | "证书将在 7 天后过期" | "建议尽快重新激活以更新证书" |
| `DeviceMismatch` | "检测到硬件变更" | "如果更换了设备，请联系管理员重新激活" |
| `ClockTampering { direction: Backward, drift_seconds: 7200 }` | "系统时间异常：回拨了 2 小时" | "请检查系统时间设置是否正确" |
| `ClockTampering { direction: Forward, drift_seconds: 2592000 }` | "系统时间异常：前跳了 30 天" | "请检查系统时间设置是否正确" |
| `Revoked { reason: "管理员操作" }` | "此设备已被停用" | "请联系管理员了解详情" |
| `NetworkError { can_continue_offline: true }` | "无法连接服务器" | "可以离线继续使用，联网后将自动同步" |
| `NetworkError { can_continue_offline: false }` | "无法连接服务器" | "请检查网络连接后重试" |

### 2. 订阅阻止 → 用户消息

| 状态 | 用户看到的消息 | 操作按钮 |
|------|--------------|---------|
| `Canceled` | "订阅已取消" | [联系客服] [续费] |
| `Unpaid` | "订阅欠费" | [立即续费] |
| `PastDue` (宽限期内) | "订阅已过期，宽限期还剩 X 天" | [续费] |
| `PastDue` (宽限期结束) | "订阅已过期，服务已暂停" | [续费] |

### 3. 前端路由建议

```typescript
getRouteForState: (state: AppState): string => {
  switch (state.type) {
    // 需要显示具体原因的状态
    case 'ServerNeedActivation':
      return '/status/need-activation';
    
    // 显示进度的状态
    case 'ServerActivating':
      return '/status/activating';
    
    // 显示订阅问题的状态
    case 'ServerSubscriptionBlocked':
      return '/status/subscription-blocked';
    
    // 首次设置
    case 'ServerNoTenant':
      return '/setup';
    
    // 其他...
  }
}
```

---

## Tauri 命令

### 现有命令增强

```rust
// get_app_state 返回增强后的 AppState
#[tauri::command]
pub async fn get_app_state(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
) -> Result<ApiResponse<AppState>, String>;
```

### 新增命令

```rust
// 获取详细健康状态
#[tauri::command]
pub async fn get_health_status(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
) -> Result<ApiResponse<HealthStatus>, String>;

// 尝试自动恢复 (如刷新证书/订阅)
#[tauri::command]
pub async fn try_auto_recover(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
) -> Result<ApiResponse<bool>, String>;
```

---

## 实现路径

### Phase 1: 定义类型 (shared crate)
1. 在 `shared/src/client/` 添加状态类型定义
2. 确保类型可以序列化到 TypeScript

### Phase 2: 后端实现 (red_coral/src-tauri)
1. 修改 `get_app_state` 返回详细原因
2. 实现 `get_health_status` 命令
3. 在各个验证点收集错误原因

### Phase 3: 前端适配 (red_coral/src)
1. 更新 TypeScript 类型定义
2. 创建状态显示组件 (`StatusScreen`)
3. 修改路由逻辑

---

## 类型对齐

### Rust → TypeScript 映射

```typescript
// red_coral/src/core/domain/types/api/appState.ts

export type ActivationRequiredReason =
  | { code: 'FirstTimeSetup' }
  | { code: 'CertificateExpired'; details: { expired_at: string; days_overdue: number } }
  | { code: 'CertificateExpiringSoon'; details: { expires_at: string; days_remaining: number } }
  | { code: 'CertificateInvalid'; details: { error: string } }
  | { code: 'SignatureInvalid'; details: { component: string; error: string } }
  | { code: 'DeviceMismatch'; details: { expected: string; actual: string } }
  | { code: 'ClockTampering'; details: { direction: 'Backward' | 'Forward'; drift_seconds: number; last_verified_at: string } }
  | { code: 'BindingInvalid'; details: { error: string } }
  | { code: 'TokenExpired'; details: { expired_at: string } }
  | { code: 'NetworkError'; details: { error: string; can_continue_offline: boolean } }
  | { code: 'Revoked'; details: { revoked_at: string; reason: string } };

export type AppState =
  | { type: 'Uninitialized' }
  | { type: 'ServerNoTenant' }
  | { type: 'ServerNeedActivation'; data: { reason: ActivationRequiredReason; can_auto_recover: boolean; recovery_hint: string } }
  | { type: 'ServerActivating'; data: { progress: ActivationProgress } }
  | { type: 'ServerCheckingSubscription' }
  | { type: 'ServerSubscriptionBlocked'; data: { info: SubscriptionBlockedInfo } }
  | { type: 'ServerReady' }
  | { type: 'ServerAuthenticated' }
  // Client 模式...
  ;
```

---

## 参考

- 当前 AppState 定义: `red_coral/src-tauri/src/core/bridge/types.rs`
- 当前前端状态: `red_coral/src/core/stores/bridge/useBridgeStore.ts`
- SignedBinding 验证: `shared/src/activation.rs`
- CertManager 自检: `crab-client/src/cert/manager.rs`
