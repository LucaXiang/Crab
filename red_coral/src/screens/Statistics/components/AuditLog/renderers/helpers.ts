import type React from 'react';
import { formatCurrency } from '@/utils/currency/formatCurrency';
import type { TranslateFn } from './types';

/** 格式化时间戳 */
export function formatTimestamp(ts: number): string {
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
export function formatBoolean(value: boolean, t: TranslateFn): string {
  return t(`audit.detail.value.${value}`);
}

/** 格式化数组 */
export function formatArray(arr: unknown[], field?: string, t?: TranslateFn): string {
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
export function formatFieldValue(
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
export function getFieldLabel(field: string, t: TranslateFn): string {
  const key = `audit.detail.field.${field}`;
  const translated = t(key);
  // 如果没有翻译，返回原字段名（将 snake_case 转为可读格式）
  return translated === key ? field.replace(/_/g, ' ') : translated;
}
