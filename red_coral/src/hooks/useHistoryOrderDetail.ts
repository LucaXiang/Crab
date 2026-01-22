import { useState, useEffect, useCallback } from 'react';
import { invokeApi } from '@/infrastructure/api';
import { HeldOrder, CartItem } from '@/core/domain/types';
import type { OrderEvent, OrderSnapshot } from '@/core/domain/types/orderEvent';
import { logger } from '@/utils/logger';

interface UseHistoryOrderDetailResult {
  order: HeldOrder | null;
  loading: boolean;
  error: string | null;
  refresh: () => void;
}

/**
 * Response type from order_get_events_for_order API
 */
interface OrderEventListData {
  events: OrderEvent[];
  snapshot: OrderSnapshot | null;
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
      // invokeApi auto-unwraps ApiResponse and throws ApiError on failure
      const data = await invokeApi<OrderEventListData>('order_get_events_for_order', {
        order_id: orderId,
      });

      // Server returns computed snapshot (no client-side event sourcing needed)
      const snapshot = data.snapshot;

      if (!snapshot) {
        throw new Error('No snapshot found for order');
      }

      // Convert OrderSnapshot to HeldOrder format
      // Map snapshot items to CartItem (adding product_id alias)
      const mappedItems: CartItem[] = snapshot.items.map(item => ({
        ...item,
        product_id: item.id,
      }));

      // Build HeldOrder from snapshot
      const rebuiltOrder: HeldOrder = {
        ...snapshot,
        items: mappedItems,
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
