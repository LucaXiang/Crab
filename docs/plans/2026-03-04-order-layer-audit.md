# RedCoral 订单层全面技术审计

**日期**: 2026-03-04
**审计范围**: 订单系统全栈 (redb → SQLite 归档 → Hash 链 → 云端同步 → 前端展示)

---

## 审计统计

| 严重度 | 数量 |
|--------|------|
| CRITICAL | 4 |
| HIGH | 18 |
| MEDIUM | 20 |
| LOW | 11 |
| **合计** | **53** |

---

## 第 1 章: redb 活跃订单层

### 架构概述

- 31 种 EventType，26 个 Applier，redb 7 表
- 三阶段命令执行: prefetch (async) → sync redb write → post-action (async)
- 每事件即时快照，FNV-1a checksum 漂移检测
- `WriteTransaction` 是 `!Send`，不可跨 await

### 问题清单

#### F-001 [CRITICAL] OrderMergedApplier 裸 f64 加法

**文件**: `edge-server/src/orders/appliers/orders_merged.rs`

`paid_amount += paid_amount` 使用裸 f64 加法，而所有其他 applier (PaymentAdded, AmountSplit, AaSplitPaid) 均使用 `rust_decimal` 精确算术。两笔金额合并时 (如 33.10 + 16.90)，f64 加法可能产生 49.999999998，导致 `remaining_amount` 计算错误、`is_fully_paid()` 判断错误。

**修复**: 改为 `to_f64(to_decimal(snapshot.paid_amount) + to_decimal(paid_amount))`

#### F-002 [CRITICAL] next_daily_count 错误静默 fallback

**文件**: `edge-server/src/orders/storage.rs`, `manager/mod.rs`

`next_daily_count().unwrap_or(1)` 在存储 I/O 错误时静默返回 1，可能产生重复收据号和发票号，违反 Verifactu 唯一性法规要求。

**修复**: 向上传播错误，不 fallback

#### F-003 [HIGH] FNV-1a checksum 覆盖严重不足

**文件**: `shared/src/order/snapshot.rs:296-321`

仅覆盖 6/40+ 字段 (items.len, unit_price, total, paid_amount, last_sequence, status)。member_id、void_type、split 状态、stamp_redemptions、comps、paid_item_quantities、table_id 等关键字段均不在 checksum 中，漂移检测名不副实。

**修复**: 扩展覆盖范围或明确注释为"仅财务核心 checksum"

#### F-004 [HIGH] OrderCompletedApplier 不调用 recalculate_totals

**文件**: `edge-server/src/orders/appliers/order_completed.rs`

强制设 paid_item_quantities 但不调 recalculate_totals，`unpaid_quantity` 字段陈旧，影响厨房票和收据展示。

**修复**: 末尾添加 `recalculate_totals(snapshot)`

#### F-005 [HIGH] PaymentAddedApplier 部分支付不更新 unpaid_quantity

**文件**: `edge-server/src/orders/appliers/payment_added.rs`

仅当 `remaining_amount <= 0.0` 时才调 recalculate_totals，部分支付后 unpaid_quantity 保持陈旧。

**修复**: 总是调用 recalculate_totals

#### F-006 [HIGH] 规则缓存写入与订单事务不原子

**文件**: `edge-server/src/orders/manager/mod.rs`

`store_rule_snapshot` 在 OpenTable 主事务 commit 后独立写入 redb。崩溃窗口内规则丢失，后续 AddItems 无规则处理。

**修复**: 将 `store_rule_snapshot` 合并到 OpenTable 主写事务中

#### F-007 [HIGH] 双幂等检查语义不一致

**文件**: `edge-server/src/orders/manager/mod.rs:629,680`

事务外预检和事务内双检的返回结构不同，幂等响应不包含原始事件数据。

**修复**: 统一两处返回路径结构，或去掉预检只留事务内检查

#### F-008 [MEDIUM] find_active_order_for_table O(N) 全表扫描

**文件**: `edge-server/src/orders/storage.rs`

每次 OpenTable/MoveOrder 反序列化所有活跃订单 JSON 做全表扫描。50+ 活跃订单时性能可感知。

**修复**: 维护内存 `table_id → order_id` 映射

#### F-009 [MEDIUM] 每事件全量 recalculate_totals

**文件**: `edge-server/src/order_money/mod.rs`

120 item 宴会订单每次 AddItems 都重算所有 item。短期可接受，长期考虑增量更新。

#### F-010 [MEDIUM] get_events_since 全表扫描

**文件**: `edge-server/src/orders/storage.rs`

events 表 key 是 `(order_id, sequence)` 复合键，无法按 sequence 范围查询。活跃订单事件清理后影响有限。

#### F-011 [MEDIUM] 每事件写 Snapshot 写放大

**文件**: `edge-server/src/orders/manager/mod.rs`

每事件 2-10KB JSON snapshot 写入。崩溃恢复有价值，短期可接受。

#### F-012 [MEDIUM] OrderMergedOutApplier 不清零金额

**文件**: `edge-server/src/orders/appliers/orders_merged.rs`

合并出订单的 subtotal/total/remaining_amount 保持旧值，归档后统计报表金额不准确。

**修复**: clear items/payments 后调用 recalculate_totals

#### F-013 [MEDIUM] AaSplitCancelled 依赖隐式约束无断言

**文件**: `edge-server/src/orders/appliers/order_split.rs`

假设 `aa_paid_shares` 已为 0 但无 debug_assert 验证。

**修复**: 添加 `debug_assert_eq!` + 防御性清零

#### F-014 [MEDIUM] stamp 自动取消基于 Phase A 陈旧数据

**文件**: `edge-server/src/orders/manager/mod.rs`

Phase A 预取的 stamp 数据在 Phase B 使用时可能已过期。已知限制，需文档化。

#### F-015 [LOW] processed_commands 无清理机制

无过期删除，约 7MB/年增长，无界但量级可控。

#### F-016 [LOW] dead_letter 无监控告警

仅 `warn!` 日志，无消息总线通知或 health check 暴露。财务数据未归档无法被发现。

**修复**: 升级为 `error!`，添加消息总线广播和 health check 字段

#### F-017 [LOW] instance_id SHA-256 截断 64-bit

生日碰撞概率 ~2.7×10⁻¹⁶/100 items，实际安全。

#### F-018 [LOW] item_removed 仅删第一匹配

设计行为，已有注释说明。

---

## 第 2 章: SQLite 归档层

### 架构概述

- `archive_order_internal()` 单原子 SQLite 事务
- `hash_chain_lock` Mutex 序列化所有链写入
- Receipt 编号: `{store_number:02}-{YYYYMMDD}-{daily_seq:04}`
- 防超退: `SUM(credit_note.total_credit)` 实时校验
- Dead letter: 3 次失败后转 bad_archives/ JSON

### 问题清单

#### A-1 [MEDIUM] archive_order 内层重试持有 semaphore

**文件**: `edge-server/src/archiving/service.rs:258-298`

重试退避期间 semaphore permit 被占用，阻塞其他订单归档。

**修复**: 删除内层重试，统一由 ArchiveWorker 外层控制

#### A-2 [LOW] 收据号格式不匹配文档 + 死代码

**文件**: `edge-server/src/archiving/service.rs:244`, `orders/manager/mod.rs:170`

`generate_next_receipt_number()` 是死代码，`system_state.order_count` 存在双重递增。

**修复**: 删除死代码路径，更新文档

#### A-3 [LOW] 防超退 f64 SUM 精度

**文件**: `edge-server/src/archiving/credit_note.rs:187`

`SUM(total_credit)` 在 SQLite REAL 下有累积浮点误差，边界情况可能失精。实际业务风险低。

#### A-4 [MEDIUM] archived_order_item 缺 UNIQUE(order_pk, instance_id) 约束

**文件**: `edge-server/migrations/0001_initial.sql:643`

同一订单理论上可能出现重复 instance_id 行，find() 取第一个导致数量校验基于错误行。

**修复**: 添加 `UNIQUE(order_pk, instance_id)` 约束

#### A-5 [LOW] Dead letter 启动时无条件重置

**文件**: `edge-server/src/archiving/worker.rs:94-99`

持久性故障会导致每次重启无限循环重试。

**修复**: 添加 dead_letter_at 时间戳，只恢复 24h 内的条目

#### A-6 [LOW] 0002 迁移文件未入 git

**文件**: `edge-server/migrations/0002_archived_order_upgrade_fields.sql`

当前在 `??` 未追踪状态。

#### A-7 [LOW] save_to_bad_archive_sync 同步 I/O

**文件**: `edge-server/src/archiving/service.rs:297`

async 上下文中用 `std::fs::write`。失败路径极少触发，影响低。

#### A-8 [MEDIUM] desglose SUM(line_total - tax) 浮点误差

**文件**: `edge-server/src/archiving/service.rs:817-823`

SQLite REAL 运算可能有微小误差影响 Verifactu 税率分拆。

**修复**: 使用 `ROUND(SUM(...), 2)` 或 Rust Decimal 重算

---

## 第 3 章: Hash 链层

### 架构概述

- chain_entry 统一链: ORDER + CREDIT_NOTE + ANULACION + UPGRADE + BREAK 交错
- hash = SHA-256(prev_hash || entry.chain_bytes())
- huella 独立链: AEAT Verifactu 税务合规
- 两链共用 hash_chain_lock，各自独立链尾指针

### 问题清单

#### H-1 [HIGH] BREAK 唯一索引逻辑在特定时序下异常

**文件**: `migrations/0001_initial.sql:732`, `cloud/worker.rs:947`

`INSERT OR IGNORE` 依赖 `UNIQUE(entry_type, entry_pk)` 防重复 BREAK，当链已恢复后再次触发同一 BREAK 时 INSERT 静默跳过但不重设 last_chain_hash。设计上可接受但需文档化。

#### H-2 [CRITICAL] 两层重试 3×3=9 次语义不一致

**文件**: `archiving/service.rs:267`, `archiving/worker.rs:176`

内层 3 次 + 外层 3 次 = 最多 9 次实际尝试，但外层 `retry_count` 只计 3 次。

**修复**: 删除 OrderArchiveService 内层重试，外层 ArchiveWorker 做唯一重试控制

#### H-3 [HIGH] credit_note 在事务外读 last_chain_hash

**文件**: `archiving/credit_note.rs:208-211`

与 anulacion (事务内读) 实现不一致。hash_chain_lock 保证安全，但维护风险。

**修复**: 统一为事务内读取

#### H-4 [MEDIUM] huella 读取用 fetch_optional

**文件**: `archiving/invoice.rs:82-87`

system_state 不存在时返回 None，huella 链从创世重开。理论风险。

**修复**: 改为 fetch_one + 显式错误处理

#### H-5 [LOW] verify_hash() warn 不拒绝

**文件**: `shared/src/cloud/sync.rs:542`

hash 链在云端无法作为篡改检测手段。合规角度需强化。

#### H-6 [LOW] write_str 用 u32 截断 cast

**文件**: `shared/src/order/canonical.rs:72`

`bytes.len() as u32` 理论截断。

**修复**: 改为 `u32::try_from().expect()`

#### H-7 [LOW] write_vec 同上

**文件**: `shared/src/order/canonical.rs:159`

#### H-8 [MEDIUM] CHAIN_BREAK 被验证逻辑分类为篡改

**文件**: `cloud/worker.rs:961`, `archiving/service.rs:~900`

`prev_hash == "CHAIN_BREAK"` 未被 `verify_daily_chain()` 识别为系统事故，被归类为数据损坏。

**修复**: 添加 `"CHAIN_BREAK"` 到 chain_resets 识别逻辑

---

## 第 4 章: 云端同步层

### 架构概述

- WebSocket 主路 + HTTP 兜底，指数退避重连
- 两条独立管道: chain_entry 链 + invoice huella 链
- 幂等 LWW upsert (ON CONFLICT ... WHERE version <= EXCLUDED.version)
- 批量 50 条/次，5 分钟周期 + archive_notify 触发

### 问题清单

#### Sync-1 [HIGH] version 字段语义混用

**文件**: `crab-cloud/src/db/sync_store.rs:384,555,958,1043`

catalog 用 wall-clock 版本计数器，归档层用 entry_pk 作 version。正确但脆弱。

**修复**: 文档化语义差异或分离为不同字段

#### Sync-2 [HIGH] catalog changelog 未知资源被标记为已同步

**文件**: `edge-server/src/cloud/worker.rs:594-611,628-638`

未知 `SyncResource` variant 被加入 `changelog_ids` 永久标记为已同步，数据永久丢失。WS 发送后不等 SyncAck 就标记。

**修复**: 未知资源不加入 changelog_ids；考虑等 SyncAck 或依赖 cursor 路径保证持久性

#### Sync-3 [HIGH] 重连无 jitter → thundering herd

**文件**: `edge-server/src/cloud/worker.rs:89-121`

纯 ×2 倍增无随机化，多店同时重连造成云端压力峰值。

**修复**: 添加 `jitter_ms = rand(0..delay/2)`

#### Sync-4 [HIGH] huella 拒绝永久阻塞发票同步

**文件**: `crab-cloud/src/db/sync_store.rs:1007`, `edge-server/src/cloud/worker.rs:1126-1160`

verify_huella() 失败返回错误 → edge `break` 停止同步 → 同一发票反复被重新获取和拒绝 → 后续所有发票永远无法同步。

**修复**: 返回结构化错误码，edge 区分 huella 拒绝并标记为 sync_error（不阻塞队列）

#### Sync-5 [MEDIUM] 初始同步非一致性快照

**文件**: `edge-server/src/cloud/worker.rs:1294-1300`

读 Products 和读 Categories 之间可能发生变更，batch 内部不一致。

**修复**: 原子读取 resource_versions 快照

#### Sync-6 [MEDIUM] 活跃订单推送非原子

**文件**: `edge-server/src/cloud/worker.rs:207-214`

逐个发送 WS 消息，中途断开导致云端收到部分快照。

**修复**: 包装为单条 `ActiveOrderFullSnapshot` 消息

#### Sync-7 [MEDIUM] 金额字段用 float8 而非 NUMERIC

**文件**: `crab-cloud/src/db/sync_store.rs:431,646`

store_archived_orders/store_order_items 金额列用 PG `float8`，违反 CLAUDE.md 约定的 `NUMERIC(12,2)`。

**修复**: 迁移金额列到 NUMERIC(12,2)，Rust 侧 f64→Decimal 转换

#### Sync-8 [MEDIUM] 云端无链连续性检测

**文件**: `crab-cloud/src/db/sync_store.rs`

store_chain_entries 入库时不验证 prev_hash 是否等于前一行 curr_hash。链 gap 仅在 edge 日志可见。

**修复**: 添加 gap 检测告警

#### Sync-9 [LOW] TLS/cert 错误当 transient 处理

**文件**: `edge-server/src/cloud/worker.rs:103-113`

120s 退避重试，应归类为 auth 级错误用 1800s。

#### Sync-10 [LOW] debounce buffer 断线丢失

断线时 pending 被 drop，初始同步路径恢复。可接受。

#### Sync-11 [LOW] changelog 未知 action 被标记已同步

同 Sync-2 的 action 维度。

#### Sync-12 [LOW] ensure_store 并发注册 store_number 竞争

**文件**: `crab-cloud/src/db/sync_store.rs:36-54`

`MAX(store_number) + 1` 无 advisory lock，并发注册可能重复。

---

## 第 5 章: 前端展示层

### 架构概述

- 4 Zustand stores: Active/Cart/Checkout/Draft，Server Authority 模式
- Tauri command → redb → broadcast → React
- PaymentFlow 7 模式路由，12+ 权限守卫
- 5 种 chain entry 类型详情页

### 问题清单

#### FE-01 [HIGH] CheckoutStore 双轨查找 fallback 脏读

**文件**: `red_coral/src/core/stores/order/useCheckoutStore.ts:25-46`

`useCheckoutOrder()` 先查 activeOrdersStore，失败回退 `checkoutOrder` 快照副本（可能过期）。

**修复**: 删除 checkoutOrder fallback，统一单一路径

#### FE-02 [HIGH] 零售订单创建绕过 checkCommandLock

**文件**: `red_coral/src/hooks/useOrderHandlers.ts:110`

`createRetailOrder` 直接 invoke Tauri API，不经过 sendCommand，disconnected 时仍可执行。

**修复**: 开头添加 checkCommandLock() 检查

#### FE-03 [HIGH] currentOrderKey 语义歧义

**文件**: `red_coral/src/core/stores/order/useCheckoutStore.ts:29-43`

堂食存 table_id，零售存 order_id。换桌后查找失败。

**修复**: 统一始终存储 order_id

#### FE-04 [HIGH] ItemSplitPage 浮点合计

**文件**: `red_coral/src/screens/Checkout/payment/ItemSplitPage.tsx:184-186`

`total += unit_price * quantity` 使用裸浮点运算，与 useMemo 中的 Currency 计算不一致。

**修复**: 使用已有的 `splitTotal` useMemo 值

#### FE-05 [HIGH] RefundModal reason 发送翻译文本

**文件**: `red_coral/src/screens/History/RefundModal.tsx:133`

`reason: t('credit_note.reason.${reason}')` 发送中文/西班牙文给后端。

**修复**: 发送原始枚举值 `reason`

#### FE-06 [HIGH] savePendingRetailOrder 无原子性

**文件**: `red_coral/src/hooks/useOrderHandlers.ts:110-113`

命令成功后写 localStorage，崩溃窗口内 pending marker 丢失。

**修复**: 命令前写 PENDING_CREATION marker，成功后更新

#### FE-07 [HIGH] AA Split 首次支付金额验证竞争

**文件**: `red_coral/src/screens/Checkout/payment/AmountSplitPage.tsx:173-180`

useEffect 链存在一帧延迟，amountSplitValue 可能是旧值。

**修复**: 提交前重新计算 amount

#### FE-08 [MEDIUM] CartStore calculateTotal 非原子 set

**文件**: `red_coral/src/core/stores/cart/useCartStore.ts:94,101`

两次独立 set() 之间可能 re-render，totalAmount 短暂不一致。

#### FE-09 [MEDIUM] 重连窗口 connected/isInitialized 两次 set

**文件**: `red_coral/src/core/hooks/useOrderEventListener.ts:67`

connected 和 isInitialized 非原子设置，存在单帧不一致。

#### FE-10 [MEDIUM] Gap≤5 不补全 Timeline

**文件**: `red_coral/src/core/stores/order/useActiveOrdersStore.ts:215-221`

1 < gap ≤ 5 仅 warn，Timeline 永久缺失中间事件。

#### FE-11 [MEDIUM] useRetailOrderRecovery 未等 isInitialized

**文件**: `red_coral/src/hooks/useRetailOrderRecovery.ts:46`

store 未初始化就开始轮询，浪费 10s。

#### FE-12 [MEDIUM] SelectModePage void 按钮双触发

**文件**: `red_coral/src/screens/Checkout/payment/SelectModePage.tsx:282-294`

EscalatableGate onAuthorized + button onClick 同时绑定 onVoid。

#### FE-13 [MEDIUM] ItemSplitPage Cash/Card 并发提交

**文件**: `red_coral/src/screens/Checkout/payment/ItemSplitPage.tsx:480-494`

Cash Modal 和 Card 按钮可同时触发。

#### FE-14 [MEDIUM] AnulacionModal 展示原始英文错误

**文件**: `red_coral/src/screens/History/AnulacionModal.tsx:50`

`result.reason` 直接展示英文字符串，违反 ErrorCode i18n 规范。

#### FE-15 [MEDIUM] TABLE_OCCUPIED 用字符串匹配

**文件**: `red_coral/src/hooks/useOrderHandlers.ts:72`

`raw.includes('occupied') || raw.includes('7002')` 脆弱匹配。

#### FE-16 [MEDIUM] Order Note 无错误处理

**文件**: `red_coral/src/screens/Checkout/payment/SelectModePage.tsx:641,657`

addOrderNote 无 try/catch，失败静默。

#### FE-17 [MEDIUM] HistoryDetail sequence=index 替代

**文件**: `red_coral/src/screens/History/HistoryDetail.tsx:50`

`convertArchivedEventToOrderEvent` 用数组下标代替真实 sequence。

#### FE-18 [MEDIUM] ChainEntryItem.status 弱类型

**文件**: `red_coral/src/core/domain/types/chainEntry.ts:15`

`status: string | null` 无枚举约束，拼写错误编译器无法捕获。

#### FE-19 [MEDIUM] Kitchen/Label 重打印无权限守卫

**文件**: `red_coral/src/screens/History/HistoryDetail.tsx:226-237`

Receipt 重打印有 EscalatableGate，但 Kitchen/Label 没有。

#### FE-20 [MEDIUM] generateCartKey 浮点精度

**文件**: `red_coral/src/utils/pricing.ts:75`

`discount` 浮点序列化不稳定，`0.005 > 0.01` 为 false 导致小折扣被忽略。

#### FE-21 [LOW] lastSequence 死代码

initializeOrders 总是传 sinceSequence: 0，lastSequence 未被使用。

#### FE-22 [LOW] BREAK 类型自动选择

History 页自动选中第一条 BREAK，detail 区域显示 BREAK 提示而非最近有效 ORDER。

#### FE-23 [LOW] navigateToOrder chainId=-1

跨页导航时 hash 区域静默消失。

#### FE-24 [LOW] OrderItemRow key 使用 product_id

**文件**: `red_coral/src/screens/History/HistoryDetail.tsx:377`

同名商品不同规格会 key 重复，应用 instance_id。

---

## 第 6 章: 跨层问题

### XL-1 [HIGH] 金额精度链路不一致

| 层 | 存储 | 运算 | 问题 |
|----|------|------|------|
| redb applier | f64 | rust_decimal | F-001 遗漏 |
| SQLite 归档 | REAL | f64 SUM | A-3 累积误差 |
| cloud PG | float8 | f64 bind | Sync-7 违反约定 |
| 前端 | number | Currency/raw f64 | FE-04 不一致 |

需要从底层开始统一: PG NUMERIC → Decimal → f64 序列化

### XL-2 [HIGH] 错误处理 i18n 不一致

- FE-05: RefundModal 发送翻译后文本
- FE-14: AnulacionModal 展示英文错误
- FE-15: TABLE_OCCUPIED 字符串匹配
- FE-16: Order Note 无错误处理

统一走 `ErrorCode → errorCode.<CODE>` 翻译 key

### XL-3 [MEDIUM] 重试/退避策略碎片化

- H-2: 两层重试 3×3=9 次
- A-1: 内层重试持有 semaphore
- Sync-3: 重连无 jitter
- Sync-4: huella 拒绝无 retry path

需要统一重试策略: 单层控制 + jitter + 区分 transient/permanent

---

## 修复优先级

### P0 — 立即修复 (数据正确性 / 阻塞性)

| ID | 修改量 | 描述 |
|----|--------|------|
| F-001 | 1 行 | OrderMergedApplier f64→Decimal |
| F-002 | 1 行 | next_daily_count 传播错误 |
| Sync-4 | ~30 行 | huella 拒绝标记 sync_error 不阻塞队列 |
| FE-05 | 1 行 | RefundModal reason 发枚举值 |

### P1 — 短期修复 (HIGH 级别)

| ID | 修改量 | 描述 |
|----|--------|------|
| F-004/F-005 | 各 1 行 | recalculate_totals 总是调用 |
| F-006 | ~10 行 | 规则缓存写入合并到主事务 |
| H-2/A-1 | ~20 行 | 删除内层重试 |
| H-3 | ~5 行 | credit_note 事务内读 last_chain_hash |
| Sync-2 | ~5 行 | 未知资源不标记已同步 |
| Sync-3 | ~3 行 | 添加 jitter |
| FE-02 | ~3 行 | 零售订单创建前 checkCommandLock |
| FE-03 | ~10 行 | currentOrderKey 统一为 order_id |
| FE-04 | 1 行 | 使用 splitTotal useMemo |

### P2 — 计划修复 (MEDIUM 级别)

| ID | 描述 |
|----|------|
| F-003 | checksum 扩展或注释 |
| F-012 | MergedOut 清零金额 |
| A-4 | 添加 UNIQUE 约束 |
| A-8 | desglose ROUND |
| H-4 | huella fetch_one |
| H-8 | CHAIN_BREAK 分类修正 |
| Sync-5-8 | 快照一致性、原子推送、NUMERIC 迁移、gap 检测 |
| FE-08-20 | 前端 MEDIUM 问题批量修复 |
