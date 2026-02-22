/**
 * Receipt Builder - 从订单快照构建收据打印数据
 *
 * 纯数据转换，无 I/O。将 HeldOrder + StoreInfo 转为 Rust ReceiptData 格式。
 */

import type { HeldOrder } from '@/core/domain/types';
import type { StoreInfo } from '@/core/domain/types/api';
import type { ReceiptData, ReceiptItem, ReceiptStoreInfo, ReceiptSurchargeInfo } from '@/infrastructure/print/printService';

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

export function buildReceiptData(
  order: HeldOrder,
  storeInfo: StoreInfo | null,
  opts?: { reprint?: boolean; voidReason?: string },
): ReceiptData {
  const now = Date.now();

  const store_info: ReceiptStoreInfo | null =
    storeInfo && storeInfo.name
      ? {
          name: storeInfo.name,
          address: storeInfo.address,
          nif: storeInfo.nif,
          phone: storeInfo.phone ?? null,
          email: storeInfo.email ?? null,
          website: storeInfo.website ?? null,
          logo_url: storeInfo.logo_url ?? null,
        }
      : null;

  // 整单附加费
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

  const items: ReceiptItem[] = order.items
    .filter((item) => !item._removed && !item.is_comped)
    .map((item) => ({
      name: item.name,
      quantity: item.quantity,
      price: item.unit_price,
      total: item.line_total,
      tax_rate: item.tax_rate / 100, // 21 -> 0.21
      discount_percent: item.manual_discount_percent ?? null,
      original_price: item.original_price !== item.price ? item.original_price : null,
      selected_options: item.selected_options
        ? item.selected_options.map((opt) => ({
            attribute_name: opt.attribute_name,
            option_name: opt.option_name,
            receipt_name: null,
            price_modifier: opt.price_modifier ?? 0,
          }))
        : null,
      spec_name: item.selected_specification?.name ?? null,
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
    store_info,
    surcharge,
    items,
    total_amount: order.total,
    qr_data: null,
  };
}
