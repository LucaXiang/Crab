# Plan 与门店限制设计

> Date: 2026-01-30
> Status: Approved

## 业务模型

```
Tenant (品牌/公司)
  ├── Store 1 = edge-server 1 (门店A)
  │     ├── Client (点菜宝 - red_coral Client 模式)
  │     ├── Client (KDS)
  │     └── Client (收银台 POS - red_coral Server 模式，内嵌 edge-server)
  ├── Store 2 = edge-server 2 (门店B)
  │     └── ...
  └── Store N
```

- **1 edge-server = 1 门店**
- **1 red_coral Server 模式 = 内嵌 1 edge-server = 1 门店**
- **Client（终端）** 通过 mTLS 连接 edge-server，数量不限
- **Tenant 切换不存在于 UI** — 一个 red_coral 实例绑定一个门店

## Plan 定义

| | Basic | Pro | Enterprise |
|---|---|---|---|
| **门店数 (max_stores)** | 1 | 3 | 0 (无限) |
| **终端数/门店** | 不限 | 不限 | 不限 |
| **功能** | 全部 | 全部 | 全部 |

- 不提供免费服务
- 现阶段功能不按 Plan 区分
- `max_stores = 0` 表示无限制

## 类型变更

### PlanType 重命名

```rust
// Before
pub enum PlanType { Free, Pro, Enterprise }

// After
pub enum PlanType { Basic, Pro, Enterprise }
```

### SubscriptionInfo 新增字段

```rust
// shared/src/activation.rs
pub struct SubscriptionInfo {
    // ... existing fields ...
    pub max_stores: u32,  // Plan 允许的最大门店数，0 = 无限
}
```

```rust
// edge-server/src/services/tenant_binding.rs
pub struct Subscription {
    // ... existing fields ...
    pub max_stores: u32,
}
```

### PlanType::max_stores() 方法

```rust
impl PlanType {
    pub fn max_stores(&self) -> u32 {
        match self {
            PlanType::Basic => 1,
            PlanType::Pro => 3,
            PlanType::Enterprise => 0, // 无限
        }
    }
}
```

## 校验策略

**校验点：crab-auth 激活时**

```
客户端请求激活
  → crab-auth 查询 tenant 已激活的门店数
  → 对比 Plan.max_stores
  → 超过上限 → 拒绝激活 (新 ErrorCode)
  → 未超过 → 正常激活
```

**现阶段**：crab-auth 为 mock 服务，暂不做真正的设备注册表和数量校验。
字段先定义好，mock 逻辑返回 Plan 对应的 max_stores 值。

## 订阅阻止恢复策略

**原则：不进入死状态，周期性重试，自动恢复**

### edge-server 启动流程（Phase 4）

```
Phase 3: wait_for_activation() + load TLS
Phase 4: while subscription_blocked {
             log warning
             sleep 60s
             sync_subscription()   ← 从 auth-server 拉取最新订阅
             re-check
         }
Phase 5: start TLS tasks + HTTPS
```

- 订阅阻止时**不启动 HTTPS/MessageBus**，但**不 park 死**
- 每 60s 自动从 auth-server 同步订阅状态
- 订阅恢复后自动继续启动
- 离线场景：sync 失败时使用缓存，下次重试

### 前端 SubscriptionBlockedScreen

- 提供 "重新检查" 按钮（手动触发 sync_subscription）
- 提供续费链接（打开外部浏览器）
- 提供联系支持链接
- 提供关闭应用按钮
- **不提供**租户切换入口

## 前端路由拆分

```
/activate                     → 设备激活（输入 auth 凭据）
                                → 成功后检查订阅 → blocked 跳 /status/subscription-blocked
/setup                        → 模式选择(Server/Client) + 端口配置
/login                        → 员工登录
/pos                          → POS 主界面
/status/subscription-blocked  → 订阅阻止页面
/status/activating            → 激活进度页面
/status/checking              → 订阅检查中页面
```

- `/activate` 和 `/setup` 分离，职责清晰
- 已激活的设备重启后跳过 `/activate`，直接进 `/setup` 或 `/login`

## 前端变更

- **移除** TenantSelectScreen 及所有租户切换 UI 入口
- **移除** SetupScreen 中 "切换租户? 点击选择" 链接
- **移除** SubscriptionBlockedScreen 中 "切换租户" 按钮
- **移除** `/tenant-select` 路由

## 影响范围

| 组件 | 变更 |
|------|------|
| `shared/src/activation.rs` | PlanType: Free→Basic, SubscriptionInfo 加 max_stores |
| `edge-server/src/services/tenant_binding.rs` | PlanType: Free→Basic, Subscription 加 max_stores |
| `crab-auth/src/api.rs` | mock 逻辑更新 Plan + max_stores |
| `red_coral/src-tauri/src/core/bridge/mod.rs` | PlanType 映射更新 |
| `red_coral/src/screens/TenantSelect/` | 删除 |
| `red_coral/src/screens/Setup/index.tsx` | 移除切换租户链接 |
| `red_coral/src/screens/Status/SubscriptionBlockedScreen.tsx` | 移除切换租户按钮 |
| `red_coral/src/App.tsx` (路由) | 移除 /tenant-select 路由 |
| `red_coral/src/core/domain/types/appState.ts` | PlanType 更新 |
