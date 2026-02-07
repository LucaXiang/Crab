/**
 * Audit Details Renderers (审计详情渲染器)
 *
 * 职责分离设计：
 * - 每个 AuditAction 类型有独立的 Renderer
 * - Renderer 负责将审计详情转换为 UI 展示数据
 * - 通过注册表映射，无需 switch case
 *
 * 类似 Timeline/renderers.ts 的策略模式
 */

import type React from 'react';
import type { AuditEntry } from '@/core/domain/types/api';
import { formatCurrency } from '@/utils/currency/formatCurrency';

// ============================================================================
// Types
// ============================================================================

type TranslateFn = (key: string, params?: Record<string, string | number>) => string;

/** 字段变更记录（与后端 FieldChange 对齐） */
export interface FieldChange {
  field: string;
  from: unknown;
  to: unknown;
}

/** 审计详情展示数据 */
export interface AuditDisplayData {
  /** 主要内容（单行或多行） */
  lines: AuditDetailLine[];
  /** 变更列表（用于 UPDATE 操作） */
  changes?: AuditChangeItem[];
  /** 是否为空 */
  isEmpty: boolean;
}

/** 审计详情行 */
export interface AuditDetailLine {
  label: string;
  value: React.ReactNode;
  /** 值的颜色类名 */
  valueClass?: string;
}

/** 变更项 */
export interface AuditChangeItem {
  field: string;
  fieldLabel: string;
  from: React.ReactNode;
  to: React.ReactNode;
}

// ============================================================================
// Renderer Interface
// ============================================================================

/**
 * Audit Details Renderer Interface (类似 Rust trait)
 *
 * 每个资源类型实现这个接口，定义如何渲染审计详情
 */
interface AuditDetailsRenderer {
  render(entry: AuditEntry, details: Record<string, unknown>, t: TranslateFn): AuditDisplayData;
}

// ============================================================================
// Helper Functions
// ============================================================================

/** 格式化时间戳 */
function formatTimestamp(ts: number): string {
  return new Date(ts).toLocaleString('zh-CN', {
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
  });
}

/** 格式化布尔值 */
function formatBoolean(value: boolean, t: TranslateFn): string {
  return t(`audit.detail.value.${value}`);
}

/** 格式化数组 */
function formatArray(arr: unknown[], field?: string, t?: TranslateFn): string {
  if (arr.length === 0) return '-';
  const defaultLabel = t?.('audit.detail.value.default_spec') ?? '默认';
  return arr.map(item => {
    if (typeof item === 'object' && item !== null) {
      const obj = item as Record<string, unknown>;
      // specs 数组：显示 name + price（name 为空时显示 "默认"）
      if (field === 'specs' && 'price' in obj) {
        const name = obj.name ? String(obj.name) : defaultLabel;
        return `${name}: ${formatCurrency(obj.price as number)}`;
      }
      // options 数组：显示 name + price_adjustment
      if ('name' in obj && 'price_adjustment' in obj) {
        const adj = obj.price_adjustment as number;
        return adj !== 0 ? `${obj.name} (+${formatCurrency(adj)})` : String(obj.name);
      }
      // 其他有 name 的对象
      if ('name' in obj) return String(obj.name);
      return JSON.stringify(item);
    }
    return String(item);
  }).join(', ');
}

/** 时间戳字段 */
const TIMESTAMP_FIELDS = new Set([
  'created_at',
  'updated_at',
  'opened_at',
  'closed_at',
  'valid_from',
  'valid_until',
]);

/** 货币字段 */
const CURRENCY_FIELDS = new Set([
  'price',
  'total',
  'subtotal',
  'discount',
  'surcharge',
  'starting_cash',
  'expected_cash',
  'actual_cash',
  'cash_variance',
  'loss_amount',
]);

/** 百分比字段 */
const PERCENT_FIELDS = new Set([
  'tax_rate',
  'discount_percent',
  'surcharge_percent',
  'value', // price_rule value when mode is PERCENTAGE
]);

/** 布尔字段 */
const BOOLEAN_FIELDS = new Set([
  'is_active',
  'is_available',
  'is_stackable',
  'is_exclusive',
  'is_kitchen_print_enabled',
  'is_label_print_enabled',
]);

/** 枚举字段 */
const ENUM_FIELDS = new Set([
  'mode',
  'rule_type',
  'scope',
  'printer_type',
]);

/**
 * 格式化单个字段值
 */
function formatFieldValue(
  field: string,
  value: unknown,
  t: TranslateFn
): React.ReactNode {
  if (value === null || value === undefined) {
    return t('audit.detail.value.none');
  }

  // 时间戳
  if (TIMESTAMP_FIELDS.has(field) && typeof value === 'number') {
    return formatTimestamp(value);
  }

  // 货币
  if (CURRENCY_FIELDS.has(field) && typeof value === 'number') {
    return formatCurrency(value);
  }

  // 百分比
  if (PERCENT_FIELDS.has(field) && typeof value === 'number') {
    return `${value}%`;
  }

  // 布尔
  if (BOOLEAN_FIELDS.has(field) && typeof value === 'boolean') {
    return formatBoolean(value, t);
  }

  // 枚举
  if (ENUM_FIELDS.has(field) && typeof value === 'string') {
    const translated = t(`audit.detail.value.${value.toLowerCase()}`);
    return translated.startsWith('audit.detail.value.') ? value : translated;
  }

  // 数组
  if (Array.isArray(value)) {
    return formatArray(value, field, t);
  }

  // 对象（如 selected_specification）
  if (typeof value === 'object' && value !== null) {
    if ('name' in value) return (value as { name: string }).name;
    return JSON.stringify(value);
  }

  return String(value);
}

/**
 * 获取字段的翻译标签
 */
function getFieldLabel(field: string, t: TranslateFn): string {
  const key = `audit.detail.field.${field}`;
  const translated = t(key);
  // 如果没有翻译，返回原字段名（将 snake_case 转为可读格式）
  return translated === key ? field.replace(/_/g, ' ') : translated;
}

// ============================================================================
// Generic Renderers
// ============================================================================

/**
 * 通用 CREATE 渲染器
 *
 * 处理快照格式：完整的 JSON 对象
 */
function createSnapshotRenderer(excludeFields: string[] = []): AuditDetailsRenderer {
  return {
    render(_entry, details, t) {
      const lines: AuditDetailLine[] = [];
      const exclude = new Set(['error', ...excludeFields]);

      for (const [field, value] of Object.entries(details)) {
        if (exclude.has(field)) continue;
        lines.push({
          label: getFieldLabel(field, t),
          value: formatFieldValue(field, value, t),
        });
      }

      return {
        lines,
        isEmpty: lines.length === 0,
      };
    },
  };
}

/**
 * 通用 UPDATE 渲染器
 *
 * 处理 diff 格式：{"changes": [{"field": "...", "from": ..., "to": ...}]}
 */
function createDiffRenderer(): AuditDetailsRenderer {
  return {
    render(_entry, details, t) {
      const changesRaw = details.changes as FieldChange[] | undefined;

      if (!changesRaw || changesRaw.length === 0) {
        // 可能是旧格式或无变更
        const note = details.note as string | undefined;
        if (note === 'no_changes_detected') {
          return {
            lines: [{ label: t('audit.detail.note'), value: t('audit.detail.no_changes') }],
            isEmpty: false,
          };
        }
        // 回退到快照渲染
        return createSnapshotRenderer().render(_entry, details, t);
      }

      const changes: AuditChangeItem[] = changesRaw.map((change) => ({
        field: change.field,
        fieldLabel: getFieldLabel(change.field, t),
        from: formatFieldValue(change.field, change.from, t),
        to: formatFieldValue(change.field, change.to, t),
      }));

      return {
        lines: [],
        changes,
        isEmpty: changes.length === 0,
      };
    },
  };
}

/**
 * 通用 DELETE 渲染器
 *
 * 处理格式：{"name": "..."}
 */
function createDeleteRenderer(): AuditDetailsRenderer {
  return {
    render(_entry, details, t) {
      const name = details.name as string | undefined;
      if (!name) {
        return { lines: [], isEmpty: true };
      }

      return {
        lines: [{
          label: t('audit.detail.field.name'),
          value: name,
        }],
        isEmpty: false,
      };
    },
  };
}

// ============================================================================
// Resource-Specific Renderers
// ============================================================================

// ---- 订单 ----

const OrderCompletedRenderer: AuditDetailsRenderer = {
  render(_entry, details, t) {
    const lines: AuditDetailLine[] = [];

    // 总金额
    if (details.final_total != null) {
      lines.push({
        label: t('audit.detail.field.final_total'),
        value: formatCurrency(details.final_total as number),
      });
    }

    // 小票号
    if (details.receipt_number) {
      lines.push({
        label: t('audit.detail.field.receipt_number'),
        value: String(details.receipt_number),
      });
    }

    // 支付明细
    const paymentSummary = details.payment_summary as Array<{ method: string; amount: number }> | undefined;
    if (paymentSummary && paymentSummary.length > 0) {
      paymentSummary.forEach((p) => {
        const methodKey = `checkout.method.${p.method.toLowerCase()}`;
        const methodDisplay = t(methodKey);
        lines.push({
          label: methodDisplay.startsWith('checkout.method.') ? p.method : methodDisplay,
          value: formatCurrency(p.amount),
        });
      });
    }

    return { lines, isEmpty: lines.length === 0 };
  },
};

const OrderVoidedRenderer: AuditDetailsRenderer = {
  render(_entry, details, t) {
    const lines: AuditDetailLine[] = [];

    // 作废类型
    if (details.void_type) {
      const voidTypeKey = `checkout.void.type.${(details.void_type as string).toLowerCase()}`;
      lines.push({
        label: t('audit.detail.field.void_type'),
        value: t(voidTypeKey),
        valueClass: details.void_type === 'LOSS_SETTLED' ? 'text-orange-600' : 'text-red-600',
      });
    }

    // 损失金额
    if (details.loss_amount != null) {
      lines.push({
        label: t('audit.detail.field.loss_amount'),
        value: formatCurrency(details.loss_amount as number),
        valueClass: 'text-red-600',
      });
    }

    // 损失原因
    if (details.loss_reason) {
      const reasonKey = `checkout.void.loss_reason.${(details.loss_reason as string).toLowerCase()}`;
      lines.push({
        label: t('audit.detail.field.loss_reason'),
        value: t(reasonKey),
      });
    }

    // 备注
    if (details.note) {
      lines.push({
        label: t('audit.detail.field.note'),
        value: String(details.note),
      });
    }

    return { lines, isEmpty: lines.length === 0 };
  },
};

const OrderMergedRenderer: AuditDetailsRenderer = {
  render(_entry, details, t) {
    const lines: AuditDetailLine[] = [];

    if (details.source_table_name) {
      lines.push({
        label: t('audit.detail.field.source_table'),
        value: String(details.source_table_name),
      });
    }

    if (details.target_table_name) {
      lines.push({
        label: t('audit.detail.field.target_table'),
        value: String(details.target_table_name),
      });
    }

    if (details.merged_paid_amount != null) {
      lines.push({
        label: t('audit.detail.field.merged_paid_amount'),
        value: formatCurrency(details.merged_paid_amount as number),
      });
    }

    return { lines, isEmpty: lines.length === 0 };
  },
};

// ---- 班次 ----

const ShiftOpenedRenderer: AuditDetailsRenderer = {
  render(_entry, details, t) {
    const lines: AuditDetailLine[] = [];

    if (details.starting_cash != null) {
      lines.push({
        label: t('audit.detail.field.starting_cash'),
        value: formatCurrency(details.starting_cash as number),
      });
    }

    if (details.opened_at != null) {
      lines.push({
        label: t('audit.detail.field.opened_at'),
        value: formatTimestamp(details.opened_at as number),
      });
    }

    return { lines, isEmpty: lines.length === 0 };
  },
};

const ShiftClosedRenderer: AuditDetailsRenderer = {
  render(_entry, details, t) {
    const lines: AuditDetailLine[] = [];

    if (details.expected_cash != null) {
      lines.push({
        label: t('audit.detail.field.expected_cash'),
        value: formatCurrency(details.expected_cash as number),
      });
    }

    if (details.actual_cash != null) {
      lines.push({
        label: t('audit.detail.field.actual_cash'),
        value: formatCurrency(details.actual_cash as number),
      });
    }

    if (details.cash_variance != null) {
      const variance = details.cash_variance as number;
      lines.push({
        label: t('audit.detail.field.cash_variance'),
        value: formatCurrency(variance),
        valueClass: variance < 0 ? 'text-red-600' : variance > 0 ? 'text-green-600' : undefined,
      });
    }

    if (details.closed_at != null) {
      lines.push({
        label: t('audit.detail.field.closed_at'),
        value: formatTimestamp(details.closed_at as number),
      });
    }

    return { lines, isEmpty: lines.length === 0 };
  },
};

// ---- 系统 ----

const SystemStartupRenderer: AuditDetailsRenderer = {
  render(_entry, details, t) {
    const lines: AuditDetailLine[] = [];

    if (details.source) {
      const sourceKey = `audit.detail.value.${details.source}`;
      lines.push({
        label: t('audit.detail.field.source'),
        value: t(sourceKey),
      });
    }

    return { lines, isEmpty: lines.length === 0 };
  },
};

const SystemAbnormalShutdownRenderer: AuditDetailsRenderer = {
  render(_entry, details, t) {
    const lines: AuditDetailLine[] = [];

    if (details.kind) {
      const kindKey = `audit.detail.value.${details.kind}`;
      lines.push({
        label: t('audit.detail.field.kind'),
        value: t(kindKey),
        valueClass: 'text-red-600',
      });
    }

    if (details.last_activity_timestamp != null) {
      lines.push({
        label: t('audit.detail.field.last_activity_timestamp'),
        value: formatTimestamp(details.last_activity_timestamp as number),
      });
    }

    if (details.note) {
      const noteKey = `audit.detail.value.${details.note}`;
      const translated = t(noteKey);
      lines.push({
        label: t('audit.detail.field.note'),
        value: translated.startsWith('audit.detail.value.') ? String(details.note) : translated,
      });
    }

    return { lines, isEmpty: lines.length === 0 };
  },
};

const ResolveSystemIssueRenderer: AuditDetailsRenderer = {
  render(_entry, details, t) {
    const lines: AuditDetailLine[] = [];

    if (details.response) {
      const responseKey = `audit.detail.value.${details.response}`;
      lines.push({
        label: t('audit.detail.field.response'),
        value: t(responseKey),
      });
    }

    if (details.note) {
      lines.push({
        label: t('audit.detail.field.note'),
        value: String(details.note),
      });
    }

    return { lines, isEmpty: lines.length === 0 };
  },
};

// ---- 认证 ----

const LoginFailedRenderer: AuditDetailsRenderer = {
  render(_entry, details, t) {
    const lines: AuditDetailLine[] = [];

    if (details.reason) {
      const reasonKey = `audit.detail.value.${details.reason}`;
      lines.push({
        label: t('audit.detail.field.reason'),
        value: t(reasonKey),
        valueClass: 'text-red-600',
      });
    }

    if (details.username) {
      lines.push({
        label: t('audit.detail.field.username'),
        value: String(details.username),
      });
    }

    return { lines, isEmpty: lines.length === 0 };
  },
};

// ============================================================================
// Renderer Registry
// ============================================================================

type AuditActionType =
  | 'system_startup'
  | 'system_shutdown'
  | 'system_abnormal_shutdown'
  | 'system_long_downtime'
  | 'resolve_system_issue'
  | 'login_success'
  | 'login_failed'
  | 'logout'
  | 'order_completed'
  | 'order_voided'
  | 'order_merged'
  | 'employee_created'
  | 'employee_updated'
  | 'employee_deleted'
  | 'role_created'
  | 'role_updated'
  | 'role_deleted'
  | 'shift_opened'
  | 'shift_closed'
  | 'product_created'
  | 'product_updated'
  | 'product_deleted'
  | 'category_created'
  | 'category_updated'
  | 'category_deleted'
  | 'tag_created'
  | 'tag_updated'
  | 'tag_deleted'
  | 'attribute_created'
  | 'attribute_updated'
  | 'attribute_deleted'
  | 'price_rule_created'
  | 'price_rule_updated'
  | 'price_rule_deleted'
  | 'zone_created'
  | 'zone_updated'
  | 'zone_deleted'
  | 'table_created'
  | 'table_updated'
  | 'table_deleted'
  | 'label_template_created'
  | 'label_template_updated'
  | 'label_template_deleted'
  | 'print_destination_created'
  | 'print_destination_updated'
  | 'print_destination_deleted'
  | 'daily_report_generated'
  | 'print_config_changed'
  | 'store_info_changed';

/**
 * 审计详情渲染器注册表
 *
 * 自动映射 AuditAction → Renderer
 */
export const AUDIT_RENDERERS: Partial<Record<AuditActionType, AuditDetailsRenderer>> = {
  // 系统
  system_startup: SystemStartupRenderer,
  system_shutdown: createSnapshotRenderer(),
  system_abnormal_shutdown: SystemAbnormalShutdownRenderer,
  system_long_downtime: SystemAbnormalShutdownRenderer,
  resolve_system_issue: ResolveSystemIssueRenderer,

  // 认证
  login_success: createSnapshotRenderer(),
  login_failed: LoginFailedRenderer,
  logout: createSnapshotRenderer(),

  // 订单
  order_completed: OrderCompletedRenderer,
  order_voided: OrderVoidedRenderer,
  order_merged: OrderMergedRenderer,

  // 班次
  shift_opened: ShiftOpenedRenderer,
  shift_closed: ShiftClosedRenderer,

  // 员工
  employee_created: createSnapshotRenderer(['hash_pass', 'is_system']),
  employee_updated: createDiffRenderer(),
  employee_deleted: createDeleteRenderer(),

  // 角色
  role_created: createSnapshotRenderer(['is_system']),
  role_updated: createDiffRenderer(),
  role_deleted: createDeleteRenderer(),

  // 商品
  product_created: createSnapshotRenderer(),
  product_updated: createDiffRenderer(),
  product_deleted: createDeleteRenderer(),

  // 分类
  category_created: createSnapshotRenderer(),
  category_updated: createDiffRenderer(),
  category_deleted: createDeleteRenderer(),

  // 标签
  tag_created: createSnapshotRenderer(['is_system']),
  tag_updated: createDiffRenderer(),
  tag_deleted: createDeleteRenderer(),

  // 属性
  attribute_created: createSnapshotRenderer(),
  attribute_updated: createDiffRenderer(),
  attribute_deleted: createDeleteRenderer(),

  // 价格规则
  price_rule_created: createSnapshotRenderer(['created_by', 'created_at']),
  price_rule_updated: createDiffRenderer(),
  price_rule_deleted: createDeleteRenderer(),

  // 区域
  zone_created: createSnapshotRenderer(),
  zone_updated: createDiffRenderer(),
  zone_deleted: createDeleteRenderer(),

  // 桌台
  table_created: createSnapshotRenderer(),
  table_updated: createDiffRenderer(),
  table_deleted: createDeleteRenderer(),

  // 标签模板
  label_template_created: createSnapshotRenderer(),
  label_template_updated: createDiffRenderer(),
  label_template_deleted: createDeleteRenderer(),

  // 打印目的地
  print_destination_created: createSnapshotRenderer(),
  print_destination_updated: createDiffRenderer(),
  print_destination_deleted: createDeleteRenderer(),

  // 日结
  daily_report_generated: createSnapshotRenderer(),

  // 配置
  print_config_changed: createDiffRenderer(),
  store_info_changed: createDiffRenderer(),
};

/**
 * 渲染审计详情
 *
 * @param entry - AuditEntry
 * @param t - 翻译函数
 * @returns AuditDisplayData - UI 展示数据
 */
export function renderAuditDetails(
  entry: AuditEntry,
  t: TranslateFn
): AuditDisplayData {
  const details = entry.details as Record<string, unknown> | null;

  if (!details || typeof details !== 'object') {
    return { lines: [], isEmpty: true };
  }

  const renderer = AUDIT_RENDERERS[entry.action as AuditActionType];

  if (!renderer) {
    // Fallback: 使用通用快照渲染器
    return createSnapshotRenderer().render(entry, details, t);
  }

  return renderer.render(entry, details, t);
}
