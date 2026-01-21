import { useState, useEffect, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { HeldOrder } from '@/core/domain/types';
import type { OrderEvent, OrderSnapshot } from '@/core/domain/types/orderEvent';
import { logger } from '@/utils/logger';

interface UseHistoryOrderDetailResult {
  order: HeldOrder | null;
  loading: boolean;
  error: string | null;
  refresh: () => void;
}

/**
 * API response wrapper type matching Rust ApiResponse<OrderEventListData>
 */
interface ApiResponse<T> {
  success: boolean;
  data?: T;
  error?: {
    code: string;
    message: string;
  };
}

interface OrderEventListData {
  events: OrderEvent[];
  snapshot?: OrderSnapshot | null;
}

/**
 * Hook for fetching full order details (items, timeline, etc.)
 *
 * Performance optimization: Only loads when orderKey is provided.
 * Reconstructs order state from events via event sourcing.
 *
 * Uses the new Event Sourcing API (order_get_events_for_order) which returns
 * events in the shared::order::OrderEvent format.
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

    // Convert order_id to string (new ES API uses string UUIDs)
    const orderId = String(order_id);
    logger.debug('Fetching order details via ES API', { component: 'useHistoryOrderDetail', action: 'fetchOrderDetail', order_id: orderId });

    try {
      // Fetch events for this specific order using the new ES API
      const response = await invoke<ApiResponse<OrderEventListData>>('order_get_events_for_order', {
        order_id: orderId,
      });

      if (!response.success) {
        throw new Error(response.error?.message || 'Failed to fetch order events');
      }

      // Server returns computed snapshot (no client-side event sourcing needed)
      const snapshot = response.data?.snapshot;

      if (!snapshot) {
        throw new Error('No snapshot found for order');
      }

      // Convert OrderSnapshot to HeldOrder format
      const rebuiltOrder: HeldOrder = {
        key: snapshot.order_id,
        table_id: snapshot.table_id || '',
        table_name: snapshot.table_name || '',
        zone_id: snapshot.zone_id,
        zone_name: snapshot.zone_name,
        guest_count: snapshot.guest_count,
        is_retail: snapshot.is_retail,
        status: snapshot.status,
        items: snapshot.items.map(item => ({
          id: item.id,
          product_id: item.id,
          instance_id: item.instance_id,
          name: item.name,
          price: item.price,
          original_price: item.original_price,
          quantity: item.quantity,
          unpaid_quantity: item.unpaid_quantity,
          selected_options: item.selected_options || [],
          selected_specification: item.selected_specification,
          discount_percent: item.discount_percent,
          surcharge: item.surcharge,
          note: item.note,
          authorizer_id: item.authorizer_id,
          authorizer_name: item.authorizer_name,
          _removed: item._removed,
        })),
        payments: snapshot.payments,
        subtotal: snapshot.subtotal,
        tax: snapshot.tax,
        discount: snapshot.discount,
        total: snapshot.total,
        paid_amount: snapshot.paid_amount,
        paid_item_quantities: snapshot.paid_item_quantities,
        receipt_number: snapshot.receipt_number || undefined,
        is_pre_payment: snapshot.is_pre_payment,
        start_time: snapshot.start_time,
        end_time: snapshot.end_time || undefined,
        timeline: [], // Timeline is loaded separately if needed
      };

      logger.debug('Loaded order snapshot from server', {
        component: 'useHistoryOrderDetail',
        action: 'fetchOrderDetail',
        order_id: orderId,
        itemCount: rebuiltOrder.items.length,
      });

      setOrder(rebuiltOrder);
    } catch (err) {
      logger.error('Failed to fetch order details', err, { component: 'useHistoryOrderDetail', action: 'fetchOrderDetail', order_id: orderId });
      setError(err instanceof Error ? err.message : 'Failed to load order');
      setOrder(null);
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
