# CLAUDE.md Restructure Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Slim root CLAUDE.md from 216 lines to ~65 lines by extracting topic-specific rules into `docs/claude/` on-demand files, and de-duplicate cross-file redundancy.

**Architecture:** Extract 4 topic files (conventions, logging, testing, schema-workflow) from root CLAUDE.md into `docs/claude/`. Root keeps index + red-line rules only. Sub-crate CLAUDE.md files (red_coral, shared) remove duplicated conventions and reference the canonical source.

**Tech Stack:** Markdown files only. No code changes.

---

### Task 1: Create docs/claude/ directory and conventions.md

**Files:**
- Create: `docs/claude/conventions.md`

**Step 1: Create the directory**

Run: `mkdir -p docs/claude`

**Step 2: Create conventions.md**

Write `docs/claude/conventions.md` with this exact content:

```markdown
# 全栈约定与架构原则

本文件是所有跨前后端约定的单一真相源。修改类型、添加约定时参考此文件。

## 类型对齐

TypeScript (前端) <-> Rust (后端) 类型必须完全匹配：
1. 先改 Rust 类型 (`shared/`, `edge-server/src/db/models/`)
2. 再改 TypeScript (`red_coral/src/core/domain/types/`)
3. 验证: `cargo check && npx tsc --noEmit`

## 全栈统一约定

| 约定 | 说明 |
|------|------|
| **ID 格式** | 全栈统一 i64 整数 (SQLite INTEGER PRIMARY KEY) |
| **时间戳** | `i64` Unix 毫秒 (Rust `i64` / TS `number` / SQLite INTEGER) |
| **金额计算** | 后端 `rust_decimal`，前端 `Currency` (decimal.js)，禁止原生浮点 |
| **货币** | 欧元 (EUR)，前端用 `formatCurrency()` 格式化 |
| **支付方式** | 统一大写: `CASH`, `CARD` |
| **API 错误码** | `shared::error::ErrorCode` (u16，按领域分区 0xxx-9xxx) |
| **命令错误码** | `shared::order::types::CommandErrorCode` — 订单命令失败的结构化错误码，详见 `shared/CLAUDE.md`；前端 `commandErrorMessage(code)` 自动翻译 |
| **异步运行时** | `tokio`，trait object 场景用 `#[async_trait]` |
| **共享状态** | `Arc` 包装，`ServerState` 设计为 clone-cheap |
| **依赖管理** | 所有依赖在 workspace `Cargo.toml` 统一声明 |
| **Tauri 命令参数** | Tauri 2 **仅自动映射顶层命令参数名** (camelCase<->snake_case)，不要在 Rust 命令上加 `rename_all`；**嵌套 struct 字段由 serde 反序列化，前端发送时必须手动转为 snake_case**，接收时手动转为 camelCase |

## 架构原则

| 原则 | 说明 |
|------|------|
| **服务端权威** | 所有金额计算、状态变更由服务端完成，前端不做乐观更新 |
| **事件溯源** | 订单用 Event Sourcing + CQRS，详见 `edge-server/CLAUDE.md` |
| **离线优先** | 边缘节点必须支持完全离线运行 |
| **RBAC 双层防御** | 前端 PermissionGate + 后端 `require_permission()` 中间件 |
| **mTLS 安全** | 生产环境必须启用双向 TLS (TLS 1.3 + aws-lc-rs) |
```

**Step 3: Verify file exists and content is correct**

Run: `wc -l docs/claude/conventions.md`
Expected: ~30 lines

---

### Task 2: Create docs/claude/logging.md

**Files:**
- Create: `docs/claude/logging.md`

**Step 1: Create logging.md**

Write `docs/claude/logging.md` with this exact content:

```markdown
# 日志规范 (tracing)

## 语言与格式

| 规则 | 说明 |
|------|------|
| **语言** | 日志消息统一英文，代码注释可以中文 |
| **无 emoji** | 日志消息禁止 emoji，用文本标签替代（如 `[STARTUP]`, `[TLS]`） |
| **结构化字段** | 优先用 `tracing::info!(order_id = %id, "Order completed")` 而非字符串拼接 |
| **无方括号前缀** | 禁止 `[function_name]` 调试前缀，用 `#[instrument]` 或 `target` 替代 |

## 级别选择

| 级别 | 适用场景 | 示例 |
|------|----------|------|
| **error** | 需要人工介入的故障，数据可能丢失 | 归档失败、存储损坏、关键通道关闭 |
| **warn** | 可自动恢复的异常，降级运行 | 重连中、缓存未命中回退、权限拒绝 |
| **info** | 业务里程碑，低频事件 | 服务启动/停止、用户登录/登出、订单完成、设备激活 |
| **debug** | 内部流程细节，开发排查用 | 命令处理过程、消息转发、缓存操作、锁获取 |
| **trace** | 极高频协议细节 | 心跳 pong、逐帧数据 |

## info 准入标准

以下场景**适合** info：
- 服务/Worker 启动和停止（每个生命周期各一条）
- 用户登录/登出（权威层记一次）
- 订单状态终结（Completed/Voided/Merged）
- 设备激活/停用
- TLS/mTLS 配置变更
- 异常恢复成功（如重连成功）

以下场景**禁止** info，应用 debug 或删除：
- 每次 HTTP 请求的处理细节（access log 用独立 `target: "http_access"`）
- 消息总线每条消息的收发
- 广播/同步的常规成功
- 锁获取/释放
- 内部初始化步骤（用最终结果一条 info 代替多条过程 info）
- 周期性任务的"无事发生"（如"No records to cleanup"）

## 单一权威点

同一业务操作只在**权威层**记录一次，避免跨层重复：
- 登录：`edge-server/api/auth` 记录，`bridge` 和 `crab-client` 不重复记录
- 命令处理：`OrdersManager` 记录结果，调用方不重复记录
- 消息转发：发送端或接收端记录一次，不两头都记

## 禁止事项

- 禁止使用 `println!` / `eprintln!`（全部用 `tracing` 宏）
- 禁止使用 `log::` crate（统一用 `tracing`）
- 禁止在日志中输出密码、JWT token、证书私钥等敏感数据
- 禁止在 info 级别记录每次请求/消息的细节（用 debug）
- 禁止"成功是常态"的冗余确认日志（如 `debug!("Sync broadcast successful")`，删除）
- 禁止中文日志消息（日志统一英文）
```

**Step 2: Verify**

Run: `wc -l docs/claude/logging.md`
Expected: ~55 lines

---

### Task 3: Create docs/claude/testing.md

**Files:**
- Create: `docs/claude/testing.md`

**Step 1: Create testing.md**

Write `docs/claude/testing.md` with this exact content:

```markdown
# 测试规范

- **命名**: `test_<action>_<scenario>` (如 `test_add_items_with_discount_rule`)
- **运行**: `cargo test --workspace --lib` (只跑单元测试，不跑 doc tests)
- **组织**: 按职责拆分测试文件，单文件不超过 500 行 (参考 `orders/manager/tests/`)
- **断言**: 用 `assert_eq!` / `assert!(matches!(..))` 而非 `unwrap()` 后比较
- **金额**: 测试中的金额断言使用 `rust_decimal::dec!()` 宏
```

**Step 2: Verify**

Run: `wc -l docs/claude/testing.md`
Expected: ~8 lines

---

### Task 4: Create docs/claude/schema-workflow.md

**Files:**
- Create: `docs/claude/schema-workflow.md`

**Step 1: Create schema-workflow.md**

Write `docs/claude/schema-workflow.md` with this exact content:

```markdown
# Schema 变更工作流

修改数据库 schema 时按以下顺序执行:

1. `sqlx migrate add -r -s <desc> --source edge-server/migrations` — 创建迁移
2. 编写 up/down SQL
3. `sqlx db reset -y --source edge-server/migrations` — 重置并应用
4. 更新 Rust 模型 (`edge-server/src/db/models/`) + 共享类型 (`shared/`)
5. `cargo sqlx prepare --workspace` — 更新离线元数据
6. 更新 TypeScript 类型 (`red_coral/src/core/domain/types/`)
7. 验证: `cargo check --workspace && cd red_coral && npx tsc --noEmit`
```

**Step 2: Verify**

Run: `wc -l docs/claude/schema-workflow.md`
Expected: ~11 lines

**Step 3: Commit all 4 topic files**

Run:
```bash
git add docs/claude/conventions.md docs/claude/logging.md docs/claude/testing.md docs/claude/schema-workflow.md
git commit -m "docs: extract CLAUDE.md topic files to docs/claude/ for on-demand loading"
```

---

### Task 5: Rewrite root CLAUDE.md (slim version)

**Files:**
- Modify: `CLAUDE.md` (full rewrite)

**Step 1: Rewrite CLAUDE.md**

Replace entire `CLAUDE.md` with this content:

```markdown
# CLAUDE.md

**现阶段项目是开发阶段, 不要适配层,不要兼容性,不要留下技术债,不要留下历史包裹**

## Crab - 分布式餐饮管理系统

Rust workspace 架构，专注离线优先、边缘计算、mTLS 安全通信的 POS 系统。

## Workspace 成员

| Crate | 用途 | 详细文档 |
|-------|------|----------|
| `shared` | 共享类型、协议、错误系统、事件溯源定义 | [`shared/CLAUDE.md`](shared/CLAUDE.md) |
| `edge-server` | 边缘服务器 (SQLite + Axum + MessageBus + 事件溯源) | [`edge-server/CLAUDE.md`](edge-server/CLAUDE.md) |
| `crab-client` | 统一客户端库 (Local/Remote + Typestate + 心跳重连) | [`crab-client/CLAUDE.md`](crab-client/CLAUDE.md) |
| `crab-cert` | PKI/证书管理 (Root CA -> Tenant CA -> Entity) | [`crab-cert/CLAUDE.md`](crab-cert/CLAUDE.md) |
| `crab-auth` | 认证服务器 (激活 + 订阅校验) | [`crab-auth/CLAUDE.md`](crab-auth/CLAUDE.md) |
| `crab-printer` | ESC/POS 热敏打印底层库 (GBK 编码) | [`crab-printer/CLAUDE.md`](crab-printer/CLAUDE.md) |
| `red_coral` | **Tauri POS 前端** (React 19 + Zustand + Tailwind) | [`red_coral/CLAUDE.md`](red_coral/CLAUDE.md) |

## 命令

```bash
# Rust workspace
cargo check --workspace        # 类型检查
cargo build --workspace        # 编译
cargo test --workspace --lib   # 测试
cargo clippy --workspace       # Lint

# SQLx CLI (详见 memory/sqlx-cli-skill.md)
sqlx migrate add -r -s <desc> --source edge-server/migrations   # 新建迁移
sqlx migrate run --source edge-server/migrations                 # 运行迁移
sqlx migrate info --source edge-server/migrations                # 查看状态
sqlx db reset -y --source edge-server/migrations                 # 重置数据库
cargo sqlx prepare --workspace                                   # 离线元数据

# POS 前端 (red_coral/)
cd red_coral && npm run tauri:dev   # Tauri 开发
cd red_coral && npx tsc --noEmit    # TS 类型检查
```

## 核心架构

- **edge-server**: 餐厅本地运行，内含 Axum API + MessageBus (TCP/TLS) + SQLite + redb 事件存储
- **red_coral**: Tauri POS 前端，双模式运行 — Server 模式内嵌 edge-server (LocalClient)，Client 模式 mTLS 远程连接 (RemoteClient)
- **crab-auth**: 云端认证服务，负责设备激活、证书签发、订阅校验
- **订单系统**: Event Sourcing + CQRS，redb 存储事件，SQLite 归档查询

## 禁止事项

- 直接删除 Order/OrderEvent 记录 (用 VOID 状态管理)
- 前端直接进行金额浮点运算 (用 `Currency` 类)
- 跳过类型对齐直接部署
- 在非 mTLS 环境传输敏感数据
- 子 crate 单独声明依赖版本
- 使用 String 格式 ID (用 i64)
- 使用 `string` 格式的时间戳 (用 `i64` Unix 毫秒)
- EventApplier 中执行 I/O 或副作用
- 使用 `f64` 进行金额计算 (用 `rust_decimal`)
- 添加转换函数/兼容层/适配器来修复类型不匹配 (从源头修)
- 使用 INTEGER cents 存储金额 (用 REAL + `rust_decimal`)
- 使用 JSON TEXT 列存储嵌套对象 (用独立关联表)

## 修复原则

类型不匹配或数据不一致时，**从 SOURCE 向外修**：数据库 schema -> Rust shared 类型 -> 前端 TypeScript 类型。禁止反向添加 `Number()`/`String()` 转换包装或适配代码。

## 提交规范

- 提交前必须通过零警告零错误: `cargo clippy --workspace` + `cd red_coral && npx tsc --noEmit`
- 只 stage 当前任务范围内的文件，不包含无关 crate/目录的变更

## 执行风格

- 设计意图明确时直接实现，不要过度提问或扩大范围
- 方向已给出时优先行动，减少规划
- UI 布局指令（按钮位置、网格列数、对齐方式）必须一次到位，实现前逐项核对约束

## 按需加载

处理以下场景时，先读取对应文件：

| 场景 | 文件 |
|------|------|
| 修改跨前后端类型、添加约定 | [`docs/claude/conventions.md`](docs/claude/conventions.md) |
| 编写/审查 tracing 日志 | [`docs/claude/logging.md`](docs/claude/logging.md) |
| 编写测试代码 | [`docs/claude/testing.md`](docs/claude/testing.md) |
| 修改数据库 schema | [`docs/claude/schema-workflow.md`](docs/claude/schema-workflow.md) |

## 响应语言

使用中文回答。
```

**Step 2: Verify line count**

Run: `wc -l CLAUDE.md`
Expected: ~80 lines (down from 216)

**Step 3: Commit**

Run:
```bash
git add CLAUDE.md
git commit -m "docs: slim root CLAUDE.md from 216 to ~80 lines with on-demand index"
```

---

### Task 6: De-duplicate red_coral/CLAUDE.md

**Files:**
- Modify: `red_coral/CLAUDE.md:236-249` (replace §关键约束 table)

**Step 1: Replace the §关键约束 section**

In `red_coral/CLAUDE.md`, replace the entire `## 关键约束` table (lines 236-249) with a slimmed version that only keeps red_coral-specific constraints and references the canonical source for shared ones:

Replace this old content (lines 236-249):
```
## 关键约束

| 约束 | 说明 |
|------|------|
| **金额计算** | 必须使用 `Currency` 类 (decimal.js)，禁止浮点运算 |
| **货币格式** | 使用 `formatCurrency()` 统一格式化，禁止硬编码货币符号 |
| **服务端权威** | 不做乐观更新，所有状态以服务端响应为准 |
| **类型对齐** | `core/domain/types/api/` 必须与 Rust `shared/models` 对齐 |
| **snake_case** | 表单字段和 API 数据统一使用 snake_case |
| **Tauri 命令参数** | Tauri 2 **仅自动映射顶层命令参数名** (camelCase↔snake_case)，不要加 `rename_all`；`invokeApi` 顶层参数用 camelCase（如 `sinceSequence`, `orderId`）；**嵌套 struct/数组内的字段由 serde 反序列化，前端必须手动转为 snake_case 发送**（如 `LabelField.dataSource` → `data_source`），接收时手动转回 camelCase |
| **懒加载** | Tauri Client 使用懒加载，禁止模块作用域直接创建 |
| **错误处理** | 关键加载失败使用 dialog 阻断，禁止静默错误 |
| **时间戳** | `number` 类型 (i64 Unix 毫秒)，禁止 string 格式 |
| **Zustand Selector** | selector 必须返回稳定引用；需要派生数据时先选原始数据再用 `useMemo`，禁止在 selector 内 `new Map()`/`.filter()`/`.map()` |
```

With this new content:
```
## 关键约束

跨前后端的通用约定（金额、时间戳、类型对齐、Tauri 命令参数等）见 [`docs/claude/conventions.md`](../docs/claude/conventions.md)。

以下为 red_coral 前端专属约束：

| 约束 | 说明 |
|------|------|
| **货币格式** | 使用 `formatCurrency()` 统一格式化，禁止硬编码货币符号 |
| **snake_case** | 表单字段和 API 数据统一使用 snake_case |
| **Tauri 命令参数** | `invokeApi` 顶层参数用 camelCase（如 `sinceSequence`, `orderId`）；**嵌套 struct/数组内的字段由 serde 反序列化，前端必须手动转为 snake_case 发送**（如 `LabelField.dataSource` → `data_source`），接收时手动转回 camelCase |
| **懒加载** | Tauri Client 使用懒加载，禁止模块作用域直接创建 |
| **错误处理** | 关键加载失败使用 dialog 阻断，禁止静默错误 |
| **Zustand Selector** | selector 必须返回稳定引用；需要派生数据时先选原始数据再用 `useMemo`，禁止在 selector 内 `new Map()`/`.filter()`/`.map()` |
```

Removed rows (now in conventions.md): 金额计算, 服务端权威, 类型对齐, 时间戳

**Step 2: Verify line count reduction**

Run: `wc -l red_coral/CLAUDE.md`
Expected: ~262 lines (down from 268, ~6 lines saved)

**Step 3: Commit**

Run:
```bash
git add red_coral/CLAUDE.md
git commit -m "docs: de-duplicate red_coral CLAUDE.md, reference conventions.md"
```

---

### Task 7: De-duplicate shared/CLAUDE.md

**Files:**
- Modify: `shared/CLAUDE.md:146-157` (replace §类型对齐 section)

**Step 1: Replace the §类型对齐 section**

In `shared/CLAUDE.md`, replace lines 146-157 (the full §类型对齐 section):

Replace this old content:
```
## 类型对齐

修改 `models/` 时，必须同步更新:
- 前端: `red_coral/src/core/domain/types/api/models.ts`
- 验证: `cargo check && npx tsc --noEmit`

**关键约定**:
- 时间戳: `i64` Unix 毫秒 (非 string)
- ID: `i64` 整数 (SQLite INTEGER PRIMARY KEY)
- 金额: 服务端 `rust_decimal`，前端 `decimal.js`
- 枚举序列化: `SCREAMING_SNAKE_CASE`
- 可选字段: `#[serde(skip_serializing_if = "Option::is_none")]`
```

With this new content:
```
## 类型对齐

修改 `models/` 时，必须同步更新前端 `red_coral/src/core/domain/types/api/models.ts`，验证: `cargo check && npx tsc --noEmit`。

全栈约定（ID、时间戳、金额等）见 [`docs/claude/conventions.md`](../docs/claude/conventions.md)。

shared 专属序列化约定:
- 枚举序列化: `SCREAMING_SNAKE_CASE`
- 可选字段: `#[serde(skip_serializing_if = "Option::is_none")]`
```

**Step 2: Verify**

Run: `wc -l shared/CLAUDE.md`
Expected: ~155 lines (down from 161, ~6 lines saved)

**Step 3: Commit**

Run:
```bash
git add shared/CLAUDE.md
git commit -m "docs: de-duplicate shared CLAUDE.md, reference conventions.md"
```

---

### Task 8: Final verification

**Step 1: Count all file sizes**

Run:
```bash
wc -l CLAUDE.md shared/CLAUDE.md edge-server/CLAUDE.md red_coral/CLAUDE.md docs/claude/*.md
```

Expected:
- `CLAUDE.md`: ~80 lines (was 216)
- `shared/CLAUDE.md`: ~155 lines (was 161)
- `red_coral/CLAUDE.md`: ~262 lines (was 268)
- `docs/claude/conventions.md`: ~30 lines
- `docs/claude/logging.md`: ~55 lines
- `docs/claude/testing.md`: ~8 lines
- `docs/claude/schema-workflow.md`: ~11 lines

**Step 2: Verify no content was lost**

Spot-check that key rules exist in exactly one place:
- "金额计算" → `docs/claude/conventions.md` (not root CLAUDE.md)
- "info 准入标准" → `docs/claude/logging.md` (not root CLAUDE.md)
- "test_<action>_<scenario>" → `docs/claude/testing.md` (not root CLAUDE.md)
- "sqlx migrate add" → `docs/claude/schema-workflow.md` AND root CLAUDE.md §命令 (commands are duplicated intentionally — quick reference vs detailed workflow)
- "禁止事项" → root CLAUDE.md (kept as red line)

Run:
```bash
grep -rl "info 准入标准" . --include="*.md" | head -5
grep -rl "test_<action>" . --include="*.md" | head -5
```

Expected: Each term appears in exactly one `docs/claude/*.md` file (plus the plan file).
