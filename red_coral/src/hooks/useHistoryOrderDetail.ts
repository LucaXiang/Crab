import { useState, useEffect, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { HeldOrder } from '@/core/domain/types';
import { OrderEvent as ESOrderEvent } from '@/core/domain/types/orderEvent';
import { toFrontendEvents } from '@/core/stores/order/orderAdapter';
import { reduceOrderEvents, createEmptyOrder } from '@/core/services/order/eventReducer';
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
  events: ESOrderEvent[];
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

      const esEvents = response.data?.events || [];

      if (esEvents.length === 0) {
        throw new Error('No events found for order');
      }

      // Convert ES events to frontend event format
      const frontendEvents = toFrontendEvents(esEvents);

      // Rebuild order state from events using the existing reducer
      const rebuiltOrder = reduceOrderEvents(frontendEvents, createEmptyOrder(orderId));

      logger.debug('Rebuilt order from ES events', {
        component: 'useHistoryOrderDetail',
        action: 'fetchOrderDetail',
        order_id: orderId,
        eventCount: esEvents.length,
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
