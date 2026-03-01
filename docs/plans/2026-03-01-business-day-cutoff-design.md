# Business Day Cutoff 精确化设计

## 目标

将 `business_day_cutoff` 从 `String` ("HH:MM") 改为 `i32`（从午夜 00:00 起的偏移分钟数），范围 0-360（对应 00:00-06:00），统一全栈处理逻辑。

## 动机

1. 消除各层的字符串解析（`parseInt(raw.split(':')[0])`）
2. 支持分钟级精度（如 03:30 = 210）
3. 校验简单：`0 <= value <= 360`
4. 前端直接用数字做时间计算

## 数据表示

| 时间 | 旧值 (String) | 新值 (i32) |
|------|---------------|------------|
| 00:00 | "00:00" | 0 |
| 02:00 | "02:00" | 120 |
| 03:30 | "03:30" | 210 |
| 06:00 | "06:00" | 360 |

默认值：`0`（午夜分界 = 传统日历日）

## 架构决策

### 前端全权处理 cutoff 逻辑

- Cloud API 保持只接收 `from`/`to` 时间戳，不感知 cutoff
- 前端负责：
  1. 根据 cutoff 计算正确的 from/to 时间范围
  2. 对返回的 hourly trend 数据做营业日排序
- 后端 SQL 继续用 `EXTRACT(HOUR)` 按日历小时分组

### 跨门店聚合

- Dashboard（租户级）取所有门店中**最大的** cutoff 值
- 最大 cutoff = 最晚的营业日开始 = 最保守的"今天"定义

### 营收趋势小时排序

```
cutoff=210 (03:30) → cutoffHour = floor(210/60) = 3
toBizOrder(h) = (h - cutoffHour + 24) % 24
排序: [3, 4, 5, ..., 23, 0, 1, 2]
```

## 变更范围

### Shared (Rust)

- `shared/src/models/store_info.rs`:
  - `StoreInfo.business_day_cutoff: String` → `i32`
  - `StoreInfoUpdate.business_day_cutoff: Option<String>` → `Option<i32>`
  - 删除 `default_cutoff()` 函数，改用 `#[serde(default)]`
- `shared/src/cloud/sync.rs`:
  - `business_day_cutoff: Option<String>` → `Option<i32>`

### Edge Server

- **Migration**: `ALTER TABLE store_info` 改 `business_day_cutoff` 为 `INTEGER NOT NULL DEFAULT 0`
- `edge-server/src/utils/time.rs`:
  - `parse_cutoff(&str) -> NaiveTime` → 直接接收 `i32` 分钟
  - `current_business_date()` 适配
- `edge-server/src/api/store_info/handler.rs`:
  - `validate_update()` 校验 `0 <= value <= 360`
- 所有引用 `business_day_cutoff` 的文件适配类型变化

### Cloud (PostgreSQL)

- **Migration**: `ALTER TABLE stores` 改列类型 `INTEGER DEFAULT 0`
- `crab-cloud/src/db/store/store_info.rs` 等适配

### Console 前端 (crab-console)

- `TimeRangeSelector.tsx`:
  - `cutoffHour` prop → `cutoffMinutes` prop
  - `startOfBusinessDay(date, minutes)` 用 `Math.floor(m/60)` + `m%60`
  - `getPresetRange()` 参数适配
- `StoreOverviewScreen.tsx` / `DashboardScreen.tsx`:
  - 直接使用 API 返回的 `i32` 值，无需解析
  - 传 `cutoffMinutes` 给 TimeRangeSelector 和 StoreOverviewDisplay
- `StoreOverviewDisplay.tsx`:
  - `cutoffHour` prop → `cutoffMinutes` prop
  - 趋势排序用 `Math.floor(cutoffMinutes / 60)` 作为 cutoff hour

### Red Coral POS 前端

- 设置页面 `business_day_cutoff` 输入改为数字（分钟）或时间选择器
- TypeScript 类型适配

## 校验规则

- **后端**：`0 <= business_day_cutoff <= 360`，超出返回 `ValidationError`
- **前端**：同样 clamp 到 0-360

## 不变量

- Business day = 24 小时（从 cutoff 分钟开始到次日同一分钟）
- 每个日历小时 (0-23) 在一个 business day 内只出现一次
- API 的 `from`/`to` 时间戳由前端根据 cutoff 计算，后端无需感知
