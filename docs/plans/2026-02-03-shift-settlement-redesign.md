# 班次跨营业日结算重设计

**日期**: 2026-02-03
**状态**: 已确认

## 问题

当前 `ShiftAutoCloseScheduler` 在 `business_day_cutoff` 时间点静默自动关闭跨营业日的僵尸班次（`abnormal_close = true`），跳过了现金盘点环节。这导致：

1. 收银员无法核对实际现金金额
2. 现金差异无法被发现和记录
3. 静默操作违反了餐饮行业的收银审计要求

## 设计目标

取消静默自动关闭，改为**强制人工结算**：在 Server 模式设备上弹出阻断式弹窗，要求操作员选择正常收班（输入现金金额）或异常关闭（填写原因）。

## 约束

- 只有 Server 模式（主收银台）需要处理，Client 模式不关心班次
- 系统不会 24 小时运行，可能在 cutoff 前关机，第二天才启动
- 每个门店只有一个收银台
- 班次是按操作员（不是终端）管理的

## 触发场景

### 场景 1: 运行中到达 cutoff 时间

程序在 `business_day_cutoff` 时间前后一直运行着。

**后端**: `ShiftAutoCloseScheduler` 到达 cutoff 时间后，不再自动关闭班次，改为通过 WebSocket/MessageBus 广播 `shift:settlement_required` 事件，携带过期班次列表。

**前端**: Server 模式设备监听该事件，弹出阻断式 `ShiftActionModal`。

### 场景 2: 启动时检测到过期班次

程序在 cutoff 前关机，第二天启动时检测到有未关闭的班次。

**后端**: `ShiftAutoCloseScheduler` 启动扫描时，发现过期班次不再自动关闭，改为广播 `shift:settlement_required`。

**前端**: 登录后检测到过期班次（`fetchCurrentShift` 返回的班次 `start_time` 早于当前营业日起始时间），弹出同样的阻断式 `ShiftActionModal`。

## 实现方案

### 后端改动

**`edge-server/src/shifts.rs`** — `ShiftAutoCloseScheduler`：

- `recover_and_broadcast()` 改为 `detect_and_notify()`
- 不再调用 `repo.recover_stale_shifts()`（该方法直接 UPDATE 数据库）
- 改为查询过期的 OPEN 班次，通过 `broadcast_sync("shift", "settlement_required", ...)` 通知前端
- 启动扫描和定时触发都走同一逻辑

**`edge-server/src/db/repository/shift.rs`** — 新增查询方法：

- `find_stale_shifts(business_day_start: i64) -> Vec<Shift>`：查询 `start_time < business_day_start AND status = 'OPEN'` 的班次，不做任何修改

**`edge-server/src/api/shifts/handler.rs`** — `recover_stale` API：

- 保持 API 接口不变（前端调试用），但标注为仅 debug 使用
- 正常流程不再通过 API 自动关闭

### 前端改动

**新增 `useStaleShiftGuard` hook** (`red_coral/src/core/hooks/useStaleShiftGuard.ts`)：

- 仅在 Server 模式 (`appState.type` 以 `Server` 开头) 下激活
- 监听 `shift:settlement_required` WebSocket 事件
- 登录后主动检测：调用 `fetchCurrentShift`，判断班次是否跨营业日
- 检测到过期班次时，设置 `staleShift` 状态，触发 `ShiftActionModal`

**复用 `ShiftActionModal`** (`red_coral/src/features/shift/ShiftActionModal.tsx`)：

- 已支持 `close`（正常收班，数字键盘输入实际现金）和 `force_close`（异常关闭，输入备注）
- 弹窗呈现两个选项：
  1. **正常收班** — 打开 `close` 模式，操作员输入实际现金金额
  2. **异常关闭** — 打开 `force_close` 模式，操作员填写原因
- 弹窗为阻断式，不可关闭（无 X 按钮，无取消），必须选择一个操作

**修改 `ShiftActionModal`**：

- 新增 `blocking` prop，为 true 时隐藏关闭按钮和取消按钮
- 过期班次场景：先展示选择页面（收班 / 异常关闭），选择后进入对应操作界面

### 数据流

```
[ShiftAutoCloseScheduler]
  ├─ 定时/启动扫描
  ├─ 查询过期 OPEN 班次
  └─ broadcast_sync("shift", "settlement_required", shift_id, shift_data)
       │
       ▼
[前端 useStaleShiftGuard]
  ├─ 监听事件 / 登录后主动检测
  ├─ 设置 staleShift 状态
  └─ 渲染阻断式 ShiftActionModal
       │
       ▼
[操作员选择]
  ├─ 正常收班 → closeShift API → 记录现金差异 → 完成
  └─ 异常关闭 → forceCloseShift API → 记录原因 → 完成
```

## 不做的事

- 不需要兜底逻辑（如超时自动关闭），因为程序不是 24 小时运行的
- 不需要 Client 模式支持
- 不修改现有的 `useShiftCloseGuard`（应用退出守卫），它独立处理退出场景
