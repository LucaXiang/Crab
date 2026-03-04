# Statistics Alignment Design: RedCoral ↔ Console

**Date**: 2026-03-04
**Scope**: RedCoral 统计页重写 + Console 数据对齐 + 日期范围统一

## 目标

1. RedCoral 和 Console 的统计数据计算逻辑完全一致
2. 两端统一日期范围预设
3. RedCoral 补齐 Red Flags、Shifts、发票明细功能
4. 修复已知 bug (GROUP BY, Tauri 缺失字段)

## Tab 结构

```
概览 | 发票明细 | 日报 & 班次 | 审计日志
```

### Tab 1: 概览 (Overview)

**内容**:
- **Red Flags 警示条** (顶部，条件显示): 有异常操作时显示红色提醒卡片 (如 "今日作废 3 单"、"删除菜品 5 次")，点击展开操作员明细。无异常时隐藏
- **KPI 卡片** (4 行):
  - Row 1: 净营收 | 订单数 | 客数 | 客单价
  - Row 2: 支付方式×2 | 人均消费 | 平均用餐时长
  - Row 3: 作废订单 | 损失订单 | 退款 | Anulación
  - Row 4: 毛营收 | 折扣 | 税额 | 附加费 | 平均菜品数
- **趋势图**: 小时趋势 (当期 + 上期 + 上周同天) / 日趋势 (多天范围)
- **Breakdown 图表**: 支付方式饼图、税率明细、分类销售 Top 10、商品排行 Top 10、标签销售、服务类型、区域销售、退款方式

**数据对齐**: KPI 字段和 breakdown 与 Console StoreOverviewDisplay 完全一致 (22 主指标 + 8 组 breakdown)

### Tab 2: 发票明细 (Invoices)

**新增功能** — 替代原 "订单明细" tab

**内容**: 分页发票表

| 列 | 字段 | 说明 |
|----|------|------|
| 发票号 | `invoice_number` | Serie+日期+序号 |
| 类型 | `tipo_factura` | F2 (销售) / R5 (更正) |
| 关联单号 | `receipt_number` 或 `credit_note_number` | 原始业务单号 |
| 金额 | `total` | 含税总额 |
| 税额 | `tax_total` | 税额合计 |
| AEAT 状态 | `aeat_status` | PENDING / SUBMITTED / ACCEPTED / REJECTED |
| 日期 | `created_at` | 开票时间 |

**筛选**: 类型 (F2/R5)、AEAT 状态、日期范围
**高亮**: REJECTED 状态红色标记

**后端**: 新增 `GET /api/invoices?from=&to=&tipo=&aeat_status=&page=&limit=`

### Tab 3: 日报 & 班次 (Reports & Shifts)

**内容**: 两个区域，上下排列

**日报区域** (已有，保持):
- 日报列表 + 生成/查看功能
- 业务日期、订单数、营业额、税额、折扣、附加费

**班次区域** (新增):
- 班次记录表

| 列 | 字段 | 说明 |
|----|------|------|
| 操作员 | `operator_name` | 员工姓名 |
| 状态 | `status` | OPEN / CLOSED |
| 开始 | `start_time` | 开班时间 |
| 结束 | `end_time` | 关班时间 |
| 时长 | 计算 | end - start |
| 初始现金 | `starting_cash` | 开班金额 |
| 预期现金 | `expected_cash` | 系统计算 |
| 实际现金 | `actual_cash` | 关班清点 |
| 差异 | `variance` | actual - expected，负值红色 |

**后端**: 已有 shifts CRUD API，新增列表查询支持时间范围过滤

### Tab 4: 审计日志 (Audit Log)

**已有，保持**: 审计记录 + 日期预设 + 操作员搜索 + action/resource 筛选

## 日期范围统一

**两端统一预设集**:

| 预设 | 标签 (zh) | 标签 (es) | 计算逻辑 |
|------|-----------|-----------|---------|
| `today` | 今天 | Hoy | business_day_cutoff → now |
| `yesterday` | 昨天 | Ayer | 前一个 business day |
| `this_week` | 本周 | Esta semana | 本周一 + cutoff → now |
| `this_month` | 本月 | Este mes | 本月1日 + cutoff → now |
| `last_month` | 上月 | Mes anterior | 上月1日 + cutoff → 本月1日 + cutoff |
| `custom` | 自定义 | Personalizado | 用户选择起止日期 |

**改动点**:
- RedCoral: 增加 `yesterday` 和 `last_month` 预设
- Console: 增加 `last_month` 预设
- Edge-server: `calculate_time_range()` 支持 `yesterday` 和 `last_month`
- 所有 tab 共享同一个时间选择器

## Bug 修复 (一并完成)

| Bug | 位置 | 修复 |
|-----|------|------|
| Cloud `GROUP BY name` | `crab-cloud/src/db/tenant_queries.rs:1261` | → `GROUP BY i.category_name` (已修) |
| Tauri 缺失 `anulacion_count/amount` | `red_coral/src-tauri/src/commands/statistics.rs` | 补字段到 StoreOverview struct |

## Red Flags 后端 (edge-server 新增)

**API**: `GET /api/statistics/red-flags?from={millis}&to={millis}`

**响应**:
```json
{
  "summary": {
    "item_removals": 5,
    "item_comps": 3,
    "order_voids": 2,
    "order_discounts": 8,
    "price_modifications": 1
  },
  "operator_breakdown": [
    {
      "operator_id": 123,
      "operator_name": "Juan",
      "item_removals": 3,
      "item_comps": 1,
      "order_voids": 1,
      "order_discounts": 4,
      "price_modifications": 0
    }
  ]
}
```

**数据源**: `archived_order_event` 表，按 event_type 分类统计
- `item_removals` ← ItemRemoved 事件
- `item_comps` ← ItemComped 事件
- `order_voids` ← OrderVoided 事件
- `order_discounts` ← OrderDiscountApplied 事件
- `price_modifications` ← ItemModified (价格变更) 事件

按 `operator_id` GROUP BY 得到操作员 breakdown

## Console 侧同步改动

- `TimeRangeSelector` 补 `last_month` 预设
- Cloud `tenant_queries.rs` GROUP BY bug 已修
- 确保 StoreOverview 响应字段与 RedCoral 完全一致

## 不在范围内

- 订单明细功能 → 已在 History 页面覆盖
- Tenant 级聚合 (多门店汇总) → Console 独有，RedCoral 不需要
- 导出/打印统计报表 → 后续迭代
