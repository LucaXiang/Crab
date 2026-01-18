import { useState, useEffect, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { HeldOrder } from '@/core/domain/types';
import { OrderEvent } from '@/core/domain/events';
import { reduceOrderEvents, createEmptyOrder } from '@/core/services/order/eventReducer';
import { logger } from '@/utils/logger';

interface UseHistoryOrderDetailResult {
  order: HeldOrder | null;
  loading: boolean;
  error: string | null;
  refresh: () => void;
}

/**
 * Hook for fetching full order details (items, timeline, etc.)
 *
 * Performance optimization: Only loads when orderKey is provided.
 * Reconstructs order state from events via event sourcing.
 */
export const useHistoryOrderDetail = (order_id: number | null): UseHistoryOrderDetailResult => {
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
    logger.debug('Fetching order details', { component: 'useHistoryOrderDetail', action: 'fetchOrderDetail', order_id });

    try {
      // Fetch events for this specific order
      const response = await invoke<{ events: OrderEvent[] }>('get_order_events', {
        params: { order_id },
      });

      if (!response.events || response.events.length === 0) {
        throw new Error('No events found for order');
      }

      // Rebuild order state from events
      const rebuiltOrder = reduceOrderEvents(response.events, createEmptyOrder(order_id.toString()));

      logger.debug('Rebuilt order from events', { component: 'useHistoryOrderDetail', action: 'fetchOrderDetail', order_id, itemCount: rebuiltOrder.items.length });
      setOrder(rebuiltOrder);
    } catch (err) {
      logger.error('Failed to fetch order details', err, { component: 'useHistoryOrderDetail', action: 'fetchOrderDetail', order_id });
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
