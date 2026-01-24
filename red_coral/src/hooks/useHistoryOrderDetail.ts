import { useState, useEffect, useCallback } from 'react';
import { invokeApi } from '@/infrastructure/api';
import { HeldOrder, CartItem } from '@/core/domain/types';
import type { OrderEvent, OrderSnapshot, OrderStatus } from '@/core/domain/types/orderEvent';
import { logger } from '@/utils/logger';
import { Currency } from '@/utils/currency';

interface UseHistoryOrderDetailResult {
  order: HeldOrder | null;
  loading: boolean;
  error: string | null;
  refresh: () => void;
}

/**
 * Response type from order_get_events_for_order API (redb active orders)
 */
interface OrderEventListData {
  events: OrderEvent[];
  snapshot: OrderSnapshot | null;
}

/**
 * Archived order from SurrealDB (via get_order API)
 */
interface ArchivedOrder {
  id?: string;
  receipt_number: string;
  zone_name?: string;
  table_name?: string;
  status: string; // COMPLETED, VOID, MOVED, MERGED
  start_time: string; // ISO string
  end_time?: string;
  guest_count?: number;
  total_amount: number;
  paid_amount: number;
  discount_amount: number;
  surcharge_amount: number;
  items: ArchivedOrderItem[];
  payments: ArchivedOrderPayment[];
  prev_hash: string;
  curr_hash: string;
  related_order_id?: string;
  operator_id?: string;
  created_at?: string;
}

interface ArchivedOrderItem {
  spec: string;
  name: string;
  spec_name?: string;
  price: number;
  quantity: number;
  attributes: ArchivedOrderItemAttribute[];
  discount_amount: number;
  surcharge_amount: number;
  note?: string;
  is_sent: boolean;
}

interface ArchivedOrderItemAttribute {
  attr_id: string;
  option_idx: number;
  name: string;
  price: number;
}

interface ArchivedOrderPayment {
  method: string;
  amount: number;
  time: string;
  reference?: string;
}

/**
 * Convert archived order from SurrealDB to HeldOrder format
 */
function convertArchivedToHeldOrder(archived: ArchivedOrder): HeldOrder {
  // Parse timestamps
  const parseTime = (timeStr: string | undefined): number => {
    if (!timeStr) return 0;
    const ts = new Date(timeStr).getTime();
    return isNaN(ts) ? 0 : ts;
  };

  // Map items - use Currency for precise calculations
  const items: CartItem[] = archived.items.map((item, idx) => {
    // Calculate line total: (price * quantity) - discount + surcharge
    const subtotal = Currency.mul(item.price, item.quantity);
    const afterDiscount = Currency.sub(subtotal, item.discount_amount);
    const lineTotal = Currency.add(afterDiscount, item.surcharge_amount);
    // Calculate effective unit price
    const unitPrice = item.quantity > 0
      ? Currency.div(lineTotal, item.quantity).toNumber()
      : item.price;

    return {
      id: item.spec,
      product_id: item.spec,
      instance_id: `archived-${idx}`,
      name: item.name,
      price: item.price,
      quantity: item.quantity,
      unpaid_quantity: 0, // Archived orders are fully paid
      original_price: item.price,
      unit_price: unitPrice,
      line_total: lineTotal.toNumber(),
      manual_discount_percent: 0,
      surcharge: item.surcharge_amount,
      note: item.note || null,
      selected_options: item.attributes.map(attr => ({
        attribute_id: attr.attr_id,
        attribute_name: attr.name,
        option_idx: attr.option_idx,
        option_name: attr.name,
        price_modifier: attr.price,
      })),
      _removed: false,
    };
  });

  // Map payments
  const payments = archived.payments.map((p, idx) => ({
    payment_id: `payment-${idx}`,
    method: p.method,
    amount: p.amount,
    timestamp: parseTime(p.time),
    note: p.reference || null,
  }));

  // Calculate totals using Currency for precision
  const total = archived.total_amount;
  const paidAmount = archived.paid_amount;
  // original_total = total + discount - surcharge (reverse the adjustments)
  const originalTotal = Currency.sub(
    Currency.add(total, archived.discount_amount),
    archived.surcharge_amount
  ).toNumber();
  // remaining = max(0, total - paid)
  const remaining = Currency.max(0, Currency.sub(total, paidAmount)).toNumber();

  return {
    order_id: archived.id || archived.receipt_number,
    table_id: null,
    table_name: archived.table_name || null,
    zone_id: null,
    zone_name: archived.zone_name || null,
    guest_count: archived.guest_count || 1,
    is_retail: archived.table_name === 'RETAIL',
    status: archived.status as OrderStatus,
    items,
    payments,
    original_total: originalTotal,
    subtotal: total,
    total_discount: archived.discount_amount,
    total_surcharge: archived.surcharge_amount,
    tax: 0,
    discount: archived.discount_amount,
    total,
    paid_amount: paidAmount,
    remaining_amount: remaining,
    receipt_number: archived.receipt_number,
    start_time: parseTime(archived.start_time),
    end_time: parseTime(archived.end_time),
    created_at: parseTime(archived.created_at || archived.start_time),
    updated_at: parseTime(archived.end_time || archived.start_time),
    last_sequence: 0,
    timeline: [], // Archived orders don't have timeline
  };
}

/**
 * Hook for fetching full order details (items, timeline, etc.)
 *
 * Fetches archived orders from SurrealDB via get_order API.
 * For history view, orders are already completed and stored in SurrealDB.
 */
export const useHistoryOrderDetail = (order_id: string | number | null): UseHistoryOrderDetailResult => {
  const [order, setOrder] = useState<HeldOrder | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const fetchOrderDetail = useCallback(async () => {
    if (!order_id) {
      setOrder(null);
      return;
    }

    if (!('__TAURI__' in window)) {
      logger.warn('Order details only available in Tauri environment', { component: 'useHistoryOrderDetail', action: 'fetchOrderDetail' });
      return;
    }

    setLoading(true);
    setError(null);

    const orderId = String(order_id);
    logger.debug('Fetching archived order from SurrealDB', { component: 'useHistoryOrderDetail', action: 'fetchOrderDetail', order_id: orderId });

    try {
      // Fetch archived order from SurrealDB
      const archivedOrder = await invokeApi<ArchivedOrder>('get_order', {
        id: `order:${orderId}`,
      });

      // Convert to HeldOrder format
      const heldOrder = convertArchivedToHeldOrder(archivedOrder);

      logger.debug('Loaded archived order from SurrealDB', {
        component: 'useHistoryOrderDetail',
        action: 'fetchOrderDetail',
        order_id: orderId,
        itemCount: heldOrder.items.length,
      });

      setOrder(heldOrder);
    } catch (err) {
      // Fallback: Try fetching from redb (active order system)
      logger.debug('Archived order not found, trying active order system', { component: 'useHistoryOrderDetail', action: 'fetchOrderDetail', order_id: orderId });

      try {
        const data = await invokeApi<OrderEventListData>('order_get_events_for_order', {
          order_id: orderId,
        });

        const snapshot = data.snapshot;
        if (!snapshot) {
          throw new Error('No snapshot found for order');
        }

        const mappedItems: CartItem[] = snapshot.items.map(item => ({
          ...item,
          product_id: item.id,
        }));

        const rebuiltOrder: HeldOrder = {
          ...snapshot,
          items: mappedItems,
          timeline: [],
        };

        setOrder(rebuiltOrder);
      } catch (fallbackErr) {
        logger.error('Failed to fetch order details', err, { component: 'useHistoryOrderDetail', action: 'fetchOrderDetail', order_id: orderId });
        setError(err instanceof Error ? err.message : 'Failed to load order');
        setOrder(null);
      }
    } finally {
      setLoading(false);
    }
  }, [order_id]);

  useEffect(() => {
    fetchOrderDetail();
  }, [fetchOrderDetail]);

  return {
    order,
    loading,
    error,
    refresh: fetchOrderDetail,
  };
};
