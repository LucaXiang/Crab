# 时间戳统一方案 (Timestamp Unification)

## 决策记录

| 决策 | 选择 | 理由 |
|------|------|------|
| 时间戳格式 | `i64` Unix 毫秒 | 前端已用，无时区歧义，排序快 |
| DB 存储 | `int` (i64) | 全链路一致，不用 `<datetime>` |
| Rust 类型 | 裸 `i64` | 简单，serde 零成本 |
| 日内时刻 | `String` "HH:MM" | `business_day_cutoff` 等保持不变 |
| 时区显示 | 前端负责 | 后端全 UTC millis，前端 `Intl.DateTimeFormat` |
| 生成方式 | `util::now_millis()` | 内部 `chrono::Utc::now().timestamp_millis()` |

## 全链路数据流

```
生成 (Rust)          存储 (SurrealDB)      传输 (JSON)         显示 (前端)
now_millis()  →      INT (i64)        →    number (i64)   →   Intl.DateTimeFormat
```

## 类型约定

| 场景 | Rust 类型 | DB 类型 | TS 类型 | 示例 |
|------|-----------|---------|---------|------|
| 时间点 | `i64` | `int` | `number` | `1706500800000` |
| 可选时间点 | `Option<i64>` | `option<int>` | `number \| null` | |
| 日内时刻 | `String` | `string` | `string` | `"06:00"` |
| 日期 | `String` | `string` | `string` | `"2026-01-29"` |

## 不改动的字段

- `business_day_cutoff` — "HH:MM" 日内时刻
- `active_start_time` / `active_end_time` — PriceRule 的 "HH:MM" 生效时段
- `business_date` — "YYYY-MM-DD" 营业日日期
- JWT `exp` claim — 标准要求 Unix 秒，保持 `usize`

## 影响范围

### A. `Option<String>` → `Option<i64>` (20 个字段)

| Model | 字段 |
|-------|------|
| Shift | `start_time`, `end_time`, `last_active_at`, `created_at`, `updated_at` |
| ShiftSummary | `start_time`, `end_time` |
| Order (archived) | `start_time`, `end_time`, `created_at` |
| StoreInfo | `created_at`, `updated_at` |
| LabelTemplate | `created_at`, `updated_at` |
| ImageRef | `created_at` |
| DailyReport | `generated_at` |
| SystemState | `last_sync_time`, `created_at`, `updated_at` |
| SystemStateUpdate | `last_sync_time` |

### B. `DateTime<Utc>` → `i64` (15+ 个字段)

| Model/Module | 字段 |
|-------------|------|
| edge PriceRule | `valid_from`, `valid_until`, `created_at` |
| edge PriceRuleCreate | `valid_from`, `valid_until` |
| edge PriceRuleUpdate | `valid_from`, `valid_until` |
| TenantBinding Subscription | `starts_at`, `expires_at`, `last_checked_at`, `signature_valid_until` |
| ActivationState | `cert_expires_at` |
| shared types.rs | `Timestamp` alias, `AuditLog::created_at`, `HealthStatus::timestamp` |

### C. shared crate 的 `String` → `i64`

| Model | 字段 |
|-------|------|
| shared PriceRule | `valid_from`, `valid_until`, `created_at` |
| shared PriceRuleCreate/Update | `valid_from`, `valid_until` |
| shared activation | `bound_at`, `last_verified_at`, `starts_at`, `expires_at`, `signature_valid_until` |
| shared app_state | `ActivationProgress::started_at` |

### D. SurrealDB Schema — `<datetime>` → `int` (22 个字段, 10 个 .surql 文件)

所有 `DEFAULT time::now()` 改为无默认值，由应用层传入 `now_millis()`。

### E. TypeScript — `string` → `number` (~20 个字段)

models.ts, appState.ts 中所有 ISO 8601 string 时间字段改为 `number`。

### F. 秒 → 毫秒 (4 个字段)

| 位置 | 字段 |
|------|------|
| Rust KitchenOrder | `created_at` (seconds → millis) |
| Rust LabelPrintRecord | `created_at` (seconds → millis) |
| TS KitchenOrder | `created_at` (seconds → millis) |
| TS LabelPrintRecord | `created_at` (seconds → millis) |

### G. Session cache — `u64` 秒 → `i64` 毫秒

| 字段 | 当前 |
|------|------|
| `EmployeeSession::expires_at` | `Option<u64>` 秒 |
| `EmployeeSession::logged_in_at` | `u64` 秒 |
| `CachedEmployee::token_expires_at` | `Option<u64>` 秒 |
| `CachedEmployee::last_online_login` | `u64` 秒 |

### H. 消除所有 `chrono::Utc::now()` 直接调用

全部替换为 `now_millis()`，chrono 仅在 `now_millis()` 内部使用。

需要保留 chrono 的场景：
- JWT 过期计算 (crab-auth) — 输出仍为 Unix 秒
- `util::now_millis()` 内部
- 日期运算 (如 7 天后) — 可用 millis 算术替代

## 前端显示层迁移

所有 `new Date(isoString)` 调用改为 `new Date(millis)`。
`toLocaleDateString`/`toLocaleTimeString` 调用保持不变（`Date` 构造器接受 millis）。

特殊场景：
- `UserManagement.tsx:175` — 当前用 `timestamp * 1000` (秒→毫秒)，统一后去掉 `* 1000`
- `DailyReport` 相关 — 日期字符串 "YYYY-MM-DD" 保持不变（不是时间戳）
- `PriceRuleWizard` — `datetime-local` input 需要 millis ↔ local datetime 转换
