/**
 * Receipt Builder - 从订单快照构建收据打印数据
 *
 * 纯数据转换，无 I/O。将 HeldOrder + StoreInfo 转为 Rust ReceiptData 格式。
 */

import type { HeldOrder, AppliedRule } from '@/core/domain/types';
import type { ArchivedOrderDetail } from '@/core/domain/types/archivedOrder';
import type { StoreInfo } from '@/core/domain/types/api';
import type { ReceiptData, ReceiptItem, ReceiptStoreInfo, ReceiptSurchargeInfo, ReceiptDiscountInfo, ReceiptRuleAdjustment } from '@/infrastructure/print/printService';
import { Currency } from '@/utils/currency';

function formatTimestamp(ms: number): string {
  return new Date(ms).toLocaleString('es-ES', {
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
  });
}

function buildStoreInfo(storeInfo: StoreInfo | null): ReceiptStoreInfo | null {
  if (!storeInfo || !storeInfo.name) return null;
  return {
    name: storeInfo.name,
    address: storeInfo.address,
    nif: storeInfo.nif,
    phone: storeInfo.phone ?? null,
    email: storeInfo.email ?? null,
    website: storeInfo.website ?? null,
    logo_url: storeInfo.logo_url ?? null,
  };
}

/**
 * 聚合所有应用的价格规则（item-level + order-level）到整单级别
 * 按 rule_id 分组，合并 calculated_amount
 */
function aggregateRuleAdjustments(order: HeldOrder): ReceiptRuleAdjustment[] {
  const ruleMap = new Map<number, { rule: AppliedRule; totalAmount: number }>();

  // 收集所有 item-level 规则
  for (const item of order.items) {
    if (item._removed || item.is_comped) continue;
    for (const rule of item.applied_rules) {
      if (rule.skipped) continue;
      const lineAmount = Currency.mul(rule.calculated_amount, item.quantity).toNumber();
      const existing = ruleMap.get(rule.rule_id);
      if (existing) {
        existing.totalAmount = Currency.add(existing.totalAmount, lineAmount).toNumber();
      } else {
        ruleMap.set(rule.rule_id, { rule, totalAmount: lineAmount });
      }
    }
  }

  // 收集 order-level 规则
  for (const rule of order.order_applied_rules) {
    if (rule.skipped) continue;
    const existing = ruleMap.get(rule.rule_id);
    if (existing) {
      existing.totalAmount = Currency.add(existing.totalAmount, rule.calculated_amount).toNumber();
    } else {
      ruleMap.set(rule.rule_id, { rule, totalAmount: rule.calculated_amount });
    }
  }

  return Array.from(ruleMap.values())
    .filter((entry) => Currency.gt(Currency.abs(entry.totalAmount), 0.005))
    .map((entry) => ({
      name: entry.rule.receipt_name || entry.rule.display_name || entry.rule.name,
      rule_type: entry.rule.rule_type,
      adjustment_type: entry.rule.adjustment_type,
      value: entry.rule.adjustment_value,
      amount: Currency.abs(entry.totalAmount).toNumber(),
    }));
}

export function buildReceiptData(
  order: HeldOrder,
  storeInfo: StoreInfo | null,
  opts?: { reprint?: boolean; voidReason?: string; prePayment?: boolean },
): ReceiptData {
  const now = Date.now();

  const store_info = buildStoreInfo(storeInfo);

  // 整单手动附加费
  let surcharge: ReceiptSurchargeInfo | null = null;
  if (order.order_manual_surcharge_amount > 0) {
    if (order.order_manual_surcharge_percent != null && order.order_manual_surcharge_percent > 0) {
      surcharge = {
        name: 'Suplemento',
        type: 'percentage',
        value: order.order_manual_surcharge_percent,
        amount: order.order_manual_surcharge_amount,
      };
    } else if (order.order_manual_surcharge_fixed != null && order.order_manual_surcharge_fixed > 0) {
      surcharge = {
        name: 'Suplemento',
        type: 'fixed',
        value: order.order_manual_surcharge_fixed,
        amount: order.order_manual_surcharge_amount,
      };
    }
  }

  // 整单手动折扣
  let discount: ReceiptDiscountInfo | null = null;
  if (order.order_manual_discount_amount > 0) {
    if (order.order_manual_discount_percent != null && order.order_manual_discount_percent > 0) {
      discount = {
        name: 'Descuento',
        type: 'percentage',
        value: order.order_manual_discount_percent,
        amount: order.order_manual_discount_amount,
      };
    } else if (order.order_manual_discount_fixed != null && order.order_manual_discount_fixed > 0) {
      discount = {
        name: 'Descuento',
        type: 'fixed',
        value: order.order_manual_discount_fixed,
        amount: order.order_manual_discount_amount,
      };
    }
  }

  // 价格规则（整单聚合）
  const rule_adjustments = aggregateRuleAdjustments(order);

  // PVP = original_price（菜单原价），IMPORTE = line_total（最终行合计）
  const items: ReceiptItem[] = order.items
    .filter((item) => !item._removed && !item.is_comped)
    .map((item) => ({
      name: item.name,
      quantity: item.quantity,
      price: item.original_price > 0 ? item.original_price : item.price,
      total: item.line_total,
      tax_rate: item.tax_rate / 100, // 21 -> 0.21
      discount_percent: item.manual_discount_percent ?? null,
      original_price: item.original_price > 0 && item.original_price !== item.price
        ? item.original_price : null,
      selected_options: item.selected_options
        ? item.selected_options
            .filter((opt) => opt.show_on_receipt)
            .map((opt) => ({
              attribute_name: opt.attribute_name,
              option_name: opt.option_name,
              receipt_name: opt.receipt_name ?? null,
              price_modifier: opt.price_modifier ?? 0,
              show_on_receipt: opt.show_on_receipt,
            }))
        : null,
      spec_name: item.selected_specification?.receipt_name
        || item.selected_specification?.name
        || null,
    }));

  return {
    order_id: order.receipt_number,
    timestamp: formatTimestamp(now),
    table_name: order.table_name ?? 'Mostrador',
    zone_name: order.zone_name ?? null,
    guest_count: order.guest_count || null,
    opened_at: order.start_time ? formatTimestamp(order.start_time) : null,
    checkout_time: order.end_time ? formatTimestamp(order.end_time) : formatTimestamp(now),
    void_reason: opts?.voidReason ?? null,
    reprint: opts?.reprint ?? false,
    pre_payment: opts?.prePayment ?? false,
    store_info,
    surcharge,
    discount,
    rule_adjustments,
    items,
    total_amount: order.total,
    queue_number: order.queue_number ?? null,
    qr_data: null,
  };
}

/**
 * 从归档订单构建收据数据（用于重打）
 */
export function buildArchivedReceiptData(
  order: ArchivedOrderDetail,
  storeInfo: StoreInfo | null,
): ReceiptData {
  const store_info = buildStoreInfo(storeInfo);

  // 整单手动附加费
  let surcharge: ReceiptSurchargeInfo | null = null;
  if (order.order_manual_surcharge_amount > 0) {
    surcharge = {
      name: 'Suplemento',
      type: 'fixed',
      value: order.order_manual_surcharge_amount,
      amount: order.order_manual_surcharge_amount,
    };
  }

  // 整单手动折扣
  let discount: ReceiptDiscountInfo | null = null;
  if (order.order_manual_discount_amount > 0) {
    discount = {
      name: 'Descuento',
      type: 'fixed',
      value: order.order_manual_discount_amount,
      amount: order.order_manual_discount_amount,
    };
  }

  // 归档订单的规则调整：从 items 的 applied_rules 聚合
  const ruleMap = new Map<number, { name: string; rule_type: string; adjustment_type: string; value: number; totalAmount: number }>();
  for (const item of order.items) {
    if (item.is_comped) continue;
    for (const rule of item.applied_rules ?? []) {
      const lineAmount = Currency.mul(rule.calculated_amount, item.quantity).toNumber();
      const existing = ruleMap.get(rule.rule_id);
      if (existing) {
        existing.totalAmount = Currency.add(existing.totalAmount, lineAmount).toNumber();
      } else {
        ruleMap.set(rule.rule_id, {
          name: rule.receipt_name || rule.display_name || rule.name,
          rule_type: rule.rule_type,
          adjustment_type: rule.adjustment_type,
          value: rule.adjustment_value,
          totalAmount: lineAmount,
        });
      }
    }
  }
  const rule_adjustments: ReceiptRuleAdjustment[] = Array.from(ruleMap.values())
    .filter((entry) => Currency.gt(Currency.abs(entry.totalAmount), 0.005))
    .map((entry) => ({
      name: entry.name,
      rule_type: entry.rule_type,
      adjustment_type: entry.adjustment_type,
      value: entry.value,
      amount: Currency.abs(entry.totalAmount).toNumber(),
    }));

  // PVP = 原价
  const items: ReceiptItem[] = order.items
    .filter((item) => !item.is_comped)
    .map((item) => ({
      name: item.name,
      quantity: item.quantity,
      price: item.price,
      total: item.line_total,
      tax_rate: item.tax_rate / 100,
      discount_percent: null,
      original_price: item.discount_amount > 0 ? item.price : null,
      selected_options: item.selected_options.length > 0
        ? item.selected_options.map((opt) => ({
            attribute_name: opt.attribute_name,
            option_name: opt.option_name,
            receipt_name: null,
            price_modifier: opt.price_modifier ?? 0,
            show_on_receipt: true,
          }))
        : null,
      spec_name: item.spec_name,
    }));

  const voidReason = order.void_type === 'CANCELLED' ? 'ANULADO'
    : order.void_type === 'LOSS_SETTLED' ? 'PÉRDIDA'
    : null;

  return {
    order_id: order.receipt_number,
    timestamp: formatTimestamp(Date.now()),
    table_name: order.table_name ?? 'Mostrador',
    zone_name: order.zone_name ?? null,
    guest_count: order.guest_count || null,
    opened_at: order.start_time ? formatTimestamp(order.start_time) : null,
    checkout_time: order.end_time ? formatTimestamp(order.end_time) : null,
    void_reason: voidReason,
    reprint: true,
    pre_payment: false,
    store_info,
    surcharge,
    discount,
    rule_adjustments,
    items,
    total_amount: order.total,
    queue_number: order.queue_number ?? null,
    qr_data: null,
  };
}
