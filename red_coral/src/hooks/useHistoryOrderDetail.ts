import { useState, useEffect, useCallback } from 'react';
import { invokeApi } from '@/infrastructure/api';
import { HeldOrder, CartItem } from '@/core/domain/types';
import type { OrderSnapshot } from '@/core/domain/types/orderEvent';
import { logger } from '@/utils/logger';

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
  events: unknown[];
  snapshot: OrderSnapshot | null;
}

/**
 * Hook for fetching full order details (items, timeline, etc.)
 *
 * Backend returns OrderSnapshot directly - no frontend conversion needed.
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
      // Backend returns OrderSnapshot directly (with timeline)
      const snapshot = await invokeApi<HeldOrder>('get_order', {
        id: `order:${orderId}`,
      });

      // Add product_id for CartItem compatibility
      const items: CartItem[] = snapshot.items.map(item => ({
        ...item,
        product_id: item.id,
      }));

      const heldOrder: HeldOrder = {
        ...snapshot,
        items,
      };

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
